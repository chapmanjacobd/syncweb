package commands

import (
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"

	"github.com/chapmanjacobd/syncweb/internal/models"
	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// Constants for serve command
const (
	// FolderRootPriority is the priority value for folder roots in search results
	FolderRootPriority = 1000
	// MaxPathLength is the maximum allowed length for path inputs
	MaxPathLength = 1024
	// MaxQueryLength is the maximum allowed length for query string inputs
	MaxQueryLength = 256
	// MaxPerPage is the maximum number of items per page in pagination
	MaxPerPage = 1000
)

// validateFolderID checks if a folder ID is valid (alphanumeric, dashes, underscores)
func validateFolderID(folderID string) error {
	if folderID == "" {
		return errors.New("folder ID cannot be empty")
	}
	if len(folderID) > 128 {
		return errors.New("folder ID too long (max 128 characters)")
	}
	// Folder IDs should be alphanumeric with dashes and underscores only
	validFolderID := regexp.MustCompile(`^[a-zA-Z0-9_-]+$`)
	if !validFolderID.MatchString(folderID) {
		return errors.New("folder ID contains invalid characters")
	}
	return nil
}

// validatePath checks if a path is safe and within allowed bounds
func validatePath(path string) error {
	if path == "" {
		return errors.New("path cannot be empty")
	}
	if len(path) > MaxPathLength {
		return fmt.Errorf("path too long (max %d characters)", MaxPathLength)
	}
	// Block directory traversal attempts
	if strings.Contains(path, "..") {
		return errors.New("path contains invalid sequence")
	}
	// Block null bytes
	if strings.ContainsAny(path, "\x00") {
		return errors.New("path contains null byte")
	}
	return nil
}

// validateQuery checks if a query string is safe and within allowed bounds
func validateQuery(query string) error {
	if len(query) > MaxQueryLength {
		return fmt.Errorf("query too long (max %d characters)", MaxQueryLength)
	}
	// Block null bytes
	if strings.ContainsAny(query, "\x00") {
		return errors.New("query contains null byte")
	}
	return nil
}

// validatePaginationParams validates and sanitizes pagination parameters
func validatePaginationParams(pageStr, perPageStr string) (page, perPage int, err error) {
	page = 1
	perPage = 100

	if pageStr != "" {
		if parsed, parseErr := strconv.Atoi(pageStr); parseErr == nil && parsed > 0 {
			page = parsed
		} else if pageStr != "" {
			return 0, 0, errors.New("invalid page number")
		}
	}

	if perPageStr != "" {
		if parsed, parseErr := strconv.Atoi(perPageStr); parseErr == nil && parsed > 0 {
			perPage = min(parsed, MaxPerPage)
		} else if perPageStr != "" {
			return 0, 0, errors.New("invalid per_page value")
		}
	}

	return page, perPage, nil
}

func (c *ServeCmd) setupSyncweb(g *SyncwebCmd) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	sw, err := syncweb.NewSyncweb(g.SyncwebHome, "syncweb", "")
	if err != nil {
		slog.Warn("Failed to initialize Syncweb instance", "error", err)
	} else {
		c.sw = sw
		if err := sw.Start(); err != nil {
			slog.Error("Failed to start Syncweb instance", "error", err)
		} else {
			slog.Info("Syncweb instance started", "myID", sw.Node.MyID())
			if err := utils.AutoCleanupMounts(); err != nil {
				slog.Warn("Failed to auto-cleanup mounts", "error", err)
			}
		}
	}
}

func (c *ServeCmd) resolveSyncwebPath(path string) (string, string, error) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil {
		return "", "", errors.New("syncweb not configured")
	}
	return c.sw.ResolveLocalPath(path)
}

func (c *ServeCmd) serveSyncwebContent(w http.ResponseWriter, r *http.Request, folderID, path, localPath string) {
	c.swMu.Lock()
	if c.sw == nil || !c.sw.IsRunning() {
		c.swMu.Unlock()
		http.Error(w, "Syncweb not configured or offline", http.StatusServiceUnavailable)
		return
	}
	// Store reference before releasing lock to avoid holding mutex during I/O
	// This is safe because we're storing the pointer value, and Syncweb handles
	// its own internal synchronization for operations
	sw := c.sw
	c.swMu.Unlock()

	slog.Info("Serving remote Syncweb file via block pulling", "path", path)
	var trimmedPath string
	if strings.HasPrefix(path, "sync://") {
		trimmedPath = strings.TrimPrefix(path, "sync://"+folderID+"/")
	} else {
		trimmedPath = strings.TrimPrefix(path, "syncweb://"+folderID+"/")
	}
	rs, err := sw.NewReadSeeker(r.Context(), folderID, trimmedPath)
	if err != nil {
		slog.Error("Failed to create SyncwebReadSeeker", "path", path, "error", err)
		http.Error(w, "Failed to stream remote file", http.StatusInternalServerError)
		return
	}
	http.ServeContent(w, r, filepath.Base(localPath), time.Now(), rs)
}

func (c *ServeCmd) addSyncwebRoots(resultsMap map[string]models.LsEntry, counts map[string]int, path string) {
	// Note: Caller must hold c.swMu.Lock()
	if c.sw != nil && (path == "/" || path == "") && c.sw.IsRunning() {
		for _, folder := range c.sw.GetFolders() {
			id := folder.ID
			entryPath := fmt.Sprintf("sync://%s/", id)
			name := id
			if folder.Path != "" {
				name += " (" + filepath.Base(folder.Path) + ")"
			}
			resultsMap[entryPath] = models.LsEntry{
				Name:  name,
				Path:  entryPath,
				IsDir: true,
			}
			counts[entryPath] = FolderRootPriority // High priority for roots
		}
	}
}

// handleSyncwebEvents returns recent sync events
// GET /api/syncweb/events
func (c *ServeCmd) handleSyncwebEvents(w http.ResponseWriter, _ *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	events := c.sw.GetEvents()
	writeOK(w, events)
}

// handleSyncwebFolders returns a list of configured Syncweb folders
// GET /api/syncweb/folders
func (c *ServeCmd) handleSyncwebFolders(w http.ResponseWriter, _ *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	folders := c.sw.GetFolders()
	for i := range folders {
		folders[i].Path = "" // Hide local path from API
	}
	writeOK(w, folders)
}

// handleSyncwebFoldersAdd adds a new sync folder
// POST /api/syncweb/folders/add
// Body: {"id": "...", "path": "..."}
func (c *ServeCmd) handleSyncwebFoldersAdd(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	var req struct {
		ID   string `json:"id"`
		Path string `json:"path"`
	}
	if err := decodeJSON(r, &req); err != nil {
		writeBadRequest(w, "Invalid request body")
		return
	}

	// For now, we use SendReceive as default
	if err := c.sw.AddFolder(req.ID, req.ID, req.Path, config.FolderTypeSendReceive); err != nil {
		writeInternalServerError(w, err.Error())
		return
	}

	writeAccepted(w, "Folder add request accepted")
}

// handleSyncwebFoldersDelete removes a sync folder
// POST /api/syncweb/folders/delete
// Body: {"id": "..."}
func (c *ServeCmd) handleSyncwebFoldersDelete(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	var req struct {
		ID string `json:"id"`
	}
	if err := decodeJSON(r, &req); err != nil {
		writeBadRequest(w, "Invalid request body")
		return
	}

	if err := c.sw.DeleteFolder(req.ID); err != nil {
		writeInternalServerError(w, err.Error())
		return
	}

	writeAccepted(w, "Folder deletion request accepted")
}

// handleSyncwebLs lists global files in a Syncweb folder
// GET /api/syncweb/ls?folder=...&prefix=..
func (c *ServeCmd) handleSyncwebLs(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	folderID := r.URL.Query().Get("folder")
	prefix := r.URL.Query().Get("prefix")

	// Validate folder ID if provided
	if folderID != "" && folderID != "/" {
		if err := validateFolderID(folderID); err != nil {
			writeBadRequest(w, "Invalid folder ID: "+err.Error())
			return
		}

		// Security check: ensure the folder is one we actually have
		configuredFolders := c.sw.GetFolders()
		found := false
		for _, f := range configuredFolders {
			if f.ID == folderID {
				found = true
				break
			}
		}
		if !found {
			writeError(w, http.StatusNotFound, "Folder not found or not configured")
			return
		}
	}

	// Validate prefix if provided
	if prefix != "" {
		if err := validateQuery(prefix); err != nil {
			writeBadRequest(w, "Invalid prefix: "+err.Error())
			return
		}
		if !strings.HasSuffix(prefix, "/") {
			prefix += "/"
		}
	}

	resultsMap := make(map[string]models.LsEntry)
	counts := make(map[string]int)

	if folderID == "" || folderID == "/" {
		c.addSyncwebRoots(resultsMap, counts, "/")
	} else {
		seq, cancel := c.sw.Node.App.Internals.AllGlobalFiles(folderID)

		for meta := range seq {
			// Check context for cancellation
			if r.Context().Err() != nil {
				_ = cancel()
				return
			}

			name := meta.Name
			if !strings.HasPrefix(name, prefix) || name == prefix {
				continue
			}

			rel := strings.TrimPrefix(name, prefix)
			parts := strings.Split(rel, "/")
			entryName := parts[0]
			isDir := len(parts) > 1

			fullSyncwebPath := fmt.Sprintf("sync://%s/%s", folderID, filepath.Join(prefix, entryName))
			if _, ok := resultsMap[fullSyncwebPath]; ok {
				continue
			}

			localPath, _, _ := c.sw.ResolveLocalPath(fullSyncwebPath)
			isLocal := utils.FileExists(localPath)

			entry := models.LsEntry{
				Name:     entryName,
				Path:     fullSyncwebPath,
				IsDir:    isDir,
				Local:    isLocal,
				Size:     meta.Size,
				Modified: meta.ModTime().Format(time.RFC3339),
			}
			if !isDir {
				entry.Type = utils.DetectMimeType(entryName)
			}
			resultsMap[fullSyncwebPath] = entry
		}
		_ = cancel()
	}

	results := make([]models.LsEntry, 0, len(resultsMap))
	for _, entry := range resultsMap {
		results = append(results, entry)
	}

	sort.Slice(results, func(i, j int) bool {
		if results[i].IsDir != results[j].IsDir {
			return results[i].IsDir
		}
		return results[i].Name < results[j].Name
	})

	writeOK(w, results)
}

// handleSyncwebDownload triggers a download for a Syncweb file
// POST /api/syncweb/download
// Body: {"path": "sync://..."}
func (c *ServeCmd) handleSyncwebDownload(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	if r.Method != http.MethodPost {
		writeMethodNotAllowed(w)
		return
	}

	var req struct {
		Path string `json:"path"`
	}

	// Try to decode from JSON body first
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		// Fallback to query param for compatibility if body is empty or malformed
		req.Path = r.URL.Query().Get("path")
	}

	if req.Path == "" {
		writeBadRequest(w, "Path required")
		return
	}

	// Validate path
	if err := validatePath(req.Path); err != nil {
		writeBadRequest(w, "Invalid path: "+err.Error())
		return
	}

	localPath, folderID, err := c.sw.ResolveLocalPath(req.Path)
	if err != nil {
		writeBadRequest(w, err.Error())
		return
	}

	if c.isPathBlacklisted(localPath) {
		writeError(w, http.StatusForbidden, "Access denied")
		return
	}

	folderPath, ok := c.sw.GetFolderPath(folderID)
	if !ok {
		writeInternalServerError(w, "Folder root not found")
		return
	}
	relativePath, _ := filepath.Rel(folderPath, localPath)

	if err := c.sw.Unignore(folderID, relativePath); err != nil {
		slog.Error("Syncweb download trigger failed", "path", req.Path, "error", err)
		writeInternalServerError(w, fmt.Sprintf("Download trigger failed: %v", err))
		return
	}

	writeAccepted(w, "Download triggered")
}

// handleSyncwebToggle toggles Syncweb between online and offline modes
// POST /api/syncweb/toggle
// Body: {"offline": bool}
func (c *ServeCmd) handleSyncwebToggle(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()

	if c.sw == nil {
		writeError(w, http.StatusServiceUnavailable, "Syncweb not configured")
		return
	}

	var req struct {
		Offline bool `json:"offline"`
	}
	if err := decodeJSON(r, &req); err != nil {
		writeBadRequest(w, "Invalid request body")
		return
	}

	if req.Offline {
		if c.sw.IsRunning() {
			slog.Info("Stopping Syncweb backend (Offline Mode)")
			c.sw.Stop()
		}
	} else {
		if !c.sw.IsRunning() {
			slog.Info("Starting Syncweb backend (Online Mode)")
			if err := c.sw.Start(); err != nil {
				slog.Error("Failed to restart Syncweb", "error", err)
				writeInternalServerError(w, "Failed to restart Syncweb: "+err.Error())
				return
			}
		}
	}

	writeOK(w, map[string]bool{"offline": !c.sw.IsRunning()})
}

// handleSyncwebStatus returns the current status of Syncweb
func (c *ServeCmd) handleSyncwebStatus(w http.ResponseWriter, _ *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()

	status := "Not Configured"
	offline := true
	if c.sw != nil {
		if c.sw.IsRunning() {
			status = "Online"
			offline = false
		} else {
			status = "Offline"
			offline = true
		}
	}

	writeOK(w, map[string]any{
		"status":  status,
		"offline": offline,
	})
}

// handleSyncwebFind searches for files across all Syncweb folders
// GET /api/syncweb/find?q=..
func (c *ServeCmd) handleSyncwebFind(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	query := r.URL.Query().Get("q")
	if query == "" {
		writeBadRequest(w, "Query required")
		return
	}

	// Validate query
	if err := validateQuery(query); err != nil {
		writeBadRequest(w, "Invalid query: "+err.Error())
		return
	}

	re, err := regexp.Compile("(?i)" + query)
	if err != nil {
		writeBadRequest(w, "Invalid regex: "+err.Error())
		return
	}

	var results []models.LsEntry
	cfg := c.sw.Node.Cfg.RawCopy()
	for _, f := range cfg.Folders {
		seq, cancel := c.sw.Node.App.Internals.AllGlobalFiles(f.ID)

		for meta := range seq {
			// Check context for cancellation
			if r.Context().Err() != nil {
				_ = cancel()
				return
			}

			if re.MatchString(meta.Name) || re.MatchString(filepath.Base(meta.Name)) {
				fullSyncwebPath := fmt.Sprintf("sync://%s/%s", f.ID, meta.Name)
				localPath, _, _ := c.sw.ResolveLocalPath(fullSyncwebPath)
				isLocal := utils.FileExists(localPath)

				results = append(results, models.LsEntry{
					Name:     filepath.Base(meta.Name),
					Path:     fullSyncwebPath,
					IsDir:    meta.Type == protocol.FileInfoTypeDirectory,
					Local:    isLocal,
					Size:     meta.Size,
					Type:     utils.DetectMimeType(meta.Name),
					Modified: meta.ModTime().Format(time.RFC3339),
				})
			}
		}
		_ = cancel()
	}

	writeOK(w, results)
}

// handleSyncwebStat returns detailed metadata for a file
// GET /api/syncweb/stat?path=sync://..
func (c *ServeCmd) handleSyncwebStat(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	path := r.URL.Query().Get("path")
	if path == "" {
		writeBadRequest(w, "Path required")
		return
	}

	// Validate path
	if err := validatePath(path); err != nil {
		writeBadRequest(w, "Invalid path: "+err.Error())
		return
	}

	localPath, folderID, err := c.sw.ResolveLocalPath(path)
	if err != nil {
		writeBadRequest(w, err.Error())
		return
	}

	folderPath, _ := c.sw.GetFolderPath(folderID)
	relPath, _ := filepath.Rel(folderPath, localPath)

	info, ok, err := c.sw.GetGlobalFileInfo(folderID, relPath)
	if err != nil {
		writeInternalServerError(w, err.Error())
		return
	}
	if !ok {
		writeError(w, http.StatusNotFound, "File not found in cluster")
		return
	}

	isLocal := utils.FileExists(localPath)
	res := map[string]any{
		"name":     filepath.Base(info.Name),
		"path":     path,
		"size":     info.Size,
		"modified": time.Unix(info.ModifiedS, 0),
		"is_dir":   info.Type == protocol.FileInfoTypeDirectory,
		"local":    isLocal,
		"folder":   folderID,
		"type":     utils.DetectMimeType(info.Name),
	}

	writeOK(w, res)
}

// handleSyncwebDevices returns a list of configured Syncthing devices
// GET /api/syncweb/devices
func (c *ServeCmd) handleSyncwebDevices(w http.ResponseWriter, _ *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	devices := c.sw.GetDevices()
	writeOK(w, devices)
}

// handleSyncwebPendingDevices returns a list of rejected/pending device IDs
// GET /api/syncweb/pending
func (c *ServeCmd) handleSyncwebPendingDevices(w http.ResponseWriter, _ *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	pending := c.sw.GetPendingDevices()
	writeOK(w, pending)
}

// handleSyncwebDevicesAdd adds a new device
// POST /api/syncweb/devices/add
// Body: {"id": "...", "name": "...", "introducer": bool}
func (c *ServeCmd) handleSyncwebDevicesAdd(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	var req struct {
		ID         string `json:"id"`
		Name       string `json:"name"`
		Introducer bool   `json:"introducer"`
	}
	if err := decodeJSON(r, &req); err != nil {
		writeBadRequest(w, "Invalid request body")
		return
	}

	if err := c.sw.AddDevice(req.ID, req.Name, req.Introducer); err != nil {
		writeInternalServerError(w, err.Error())
		return
	}

	writeAccepted(w, "Device add request accepted")
}

// handleSyncwebPendingFolders returns a list of pending folder invitations
// GET /api/syncweb/pending-folders
func (c *ServeCmd) handleSyncwebPendingFolders(w http.ResponseWriter, _ *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	pending := c.sw.GetPendingFolders()
	writeOK(w, pending)
}

// handleSyncwebFoldersJoin joins a pending folder
// POST /api/syncweb/folders/join
// Body: {"folder_id": "...", "device_id": "...", "path": "..."}
func (c *ServeCmd) handleSyncwebFoldersJoin(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	var req struct {
		FolderID string `json:"folder_id"`
		DeviceID string `json:"device_id"`
		Path     string `json:"path"`
	}
	if err := decodeJSON(r, &req); err != nil {
		writeBadRequest(w, "Invalid request body")
		return
	}

	// Add folder if it doesn't exist
	if err := c.sw.AddFolder(req.FolderID, req.FolderID, req.Path, 0); err != nil {
		writeInternalServerError(w, err.Error())
		return
	}

	// Share with device if specified
	if req.DeviceID != "" {
		if err := c.sw.AddFolderDevice(req.FolderID, req.DeviceID); err != nil {
			writeInternalServerError(w, err.Error())
			return
		}
	}

	writeAccepted(w, "Folder join request accepted")
}

// handleSyncwebDevicesDelete removes a device
// POST /api/syncweb/devices/delete
// Body: {"id": "..."}
func (c *ServeCmd) handleSyncwebDevicesDelete(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	var req struct {
		ID string `json:"id"`
	}
	if err := decodeJSON(r, &req); err != nil {
		writeBadRequest(w, "Invalid request body")
		return
	}

	if err := c.sw.DeleteDevice(req.ID); err != nil {
		writeInternalServerError(w, err.Error())
		return
	}

	writeAccepted(w, "Device deletion request accepted")
}

// handleSyncwebCompletion returns folder completion percentage for a device
// GET /api/syncweb/completion?device_id=...&folder_id=..
func (c *ServeCmd) handleSyncwebCompletion(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	deviceIDStr := r.URL.Query().Get("device_id")
	folderID := r.URL.Query().Get("folder_id")

	if deviceIDStr == "" || folderID == "" {
		writeBadRequest(w, "Missing device_id or folder_id parameter")
		return
	}

	// Validate folder ID
	if err := validateFolderID(folderID); err != nil {
		writeBadRequest(w, "Invalid folder_id: "+err.Error())
		return
	}

	deviceID, err := protocol.DeviceIDFromString(deviceIDStr)
	if err != nil {
		writeBadRequest(w, "Invalid device_id: "+err.Error())
		return
	}

	completion, err := c.sw.GetCompletion(deviceID, folderID)
	if err != nil {
		writeInternalServerError(w, err.Error())
		return
	}

	writeOK(w, completion)
}

// handleSyncwebTree returns folder tree structure for browsing
// GET /api/syncweb/tree?folder_id=...&prefix=...&levels=-1&dirs_only=false
func (c *ServeCmd) handleSyncwebTree(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	folderID := r.URL.Query().Get("folder_id")
	prefix := r.URL.Query().Get("prefix")
	levelsStr := r.URL.Query().Get("levels")
	dirsOnlyStr := r.URL.Query().Get("dirs_only")

	if folderID == "" {
		writeBadRequest(w, "Missing folder_id parameter")
		return
	}

	// Validate folder ID
	if err := validateFolderID(folderID); err != nil {
		writeBadRequest(w, "Invalid folder_id: "+err.Error())
		return
	}

	// Validate prefix if provided
	if prefix != "" {
		if err := validateQuery(prefix); err != nil {
			writeBadRequest(w, "Invalid prefix: "+err.Error())
			return
		}
	}

	levels := -1
	if levelsStr != "" {
		if parsed, err := strconv.Atoi(levelsStr); err == nil {
			levels = min(
				// Limit levels to prevent excessive recursion
				parsed, 100)
		}
	}

	dirsOnly := dirsOnlyStr == "true"

	tree, err := c.sw.GetGlobalTree(folderID, prefix, levels, dirsOnly)
	if err != nil {
		writeInternalServerError(w, err.Error())
		return
	}

	writeOK(w, map[string]any{"tree": tree})
}

// handleSyncwebLocalChanged returns locally changed files for a folder
// GET /api/syncweb/local-changed?folder_id=...&page=1&per_page=100
func (c *ServeCmd) handleSyncwebLocalChanged(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	folderID := r.URL.Query().Get("folder_id")
	pageStr := r.URL.Query().Get("page")
	perPageStr := r.URL.Query().Get("per_page")

	if folderID == "" {
		writeBadRequest(w, "Missing folder_id parameter")
		return
	}

	// Validate folder ID
	if err := validateFolderID(folderID); err != nil {
		writeBadRequest(w, "Invalid folder_id: "+err.Error())
		return
	}

	// Validate and parse pagination params
	page, perPage, err := validatePaginationParams(pageStr, perPageStr)
	if err != nil {
		writeBadRequest(w, err.Error())
		return
	}

	files, err := c.sw.GetLocalChangedFiles(folderID, page, perPage)
	if err != nil {
		writeInternalServerError(w, err.Error())
		return
	}

	writeOK(w, map[string]any{"files": files, "page": page, "per_page": perPage})
}

// handleSyncwebNeed returns paginated list of needed files for a folder
// GET /api/syncweb/need?folder_id=...&page=1&per_page=100
func (c *ServeCmd) handleSyncwebNeed(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	folderID := r.URL.Query().Get("folder_id")
	pageStr := r.URL.Query().Get("page")
	perPageStr := r.URL.Query().Get("per_page")

	if folderID == "" {
		writeBadRequest(w, "Missing folder_id parameter")
		return
	}

	// Validate folder ID
	if err := validateFolderID(folderID); err != nil {
		writeBadRequest(w, "Invalid folder_id: "+err.Error())
		return
	}

	// Validate and parse pagination params
	page, perPage, err := validatePaginationParams(pageStr, perPageStr)
	if err != nil {
		writeBadRequest(w, err.Error())
		return
	}

	remote, local, queued, err := c.sw.GetNeedFiles(folderID, page, perPage)
	if err != nil {
		writeInternalServerError(w, err.Error())
		return
	}

	writeOK(w, map[string]any{
		"remote":   remote,
		"local":    local,
		"queued":   queued,
		"page":     page,
		"per_page": perPage,
	})
}

// handleSyncwebRemoteNeed returns files needed by a specific remote device
// GET /api/syncweb/remote-need?folder_id=...&device_id=...&page=1&per_page=100
func (c *ServeCmd) handleSyncwebRemoteNeed(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		writeServiceUnavailable(w)
		return
	}

	folderID := r.URL.Query().Get("folder_id")
	deviceIDStr := r.URL.Query().Get("device_id")
	pageStr := r.URL.Query().Get("page")
	perPageStr := r.URL.Query().Get("per_page")

	if folderID == "" || deviceIDStr == "" {
		writeBadRequest(w, "Missing folder_id or device_id parameter")
		return
	}

	// Validate folder ID
	if err := validateFolderID(folderID); err != nil {
		writeBadRequest(w, "Invalid folder_id: "+err.Error())
		return
	}

	deviceID, err := protocol.DeviceIDFromString(deviceIDStr)
	if err != nil {
		writeBadRequest(w, "Invalid device_id: "+err.Error())
		return
	}

	// Validate and parse pagination params
	page, perPage, err := validatePaginationParams(pageStr, perPageStr)
	if err != nil {
		writeBadRequest(w, err.Error())
		return
	}

	files, err := c.sw.GetRemoteNeedFiles(folderID, deviceID, page, perPage)
	if err != nil {
		writeInternalServerError(w, err.Error())
		return
	}

	writeOK(w, map[string]any{"files": files, "page": page, "per_page": perPage})
}
