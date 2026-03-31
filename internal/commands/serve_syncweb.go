package commands

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/models"
	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"
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
		return fmt.Errorf("folder ID cannot be empty")
	}
	if len(folderID) > 128 {
		return fmt.Errorf("folder ID too long (max 128 characters)")
	}
	// Folder IDs should be alphanumeric with dashes and underscores only
	validFolderID := regexp.MustCompile(`^[a-zA-Z0-9_-]+$`)
	if !validFolderID.MatchString(folderID) {
		return fmt.Errorf("folder ID contains invalid characters")
	}
	return nil
}

// validatePath checks if a path is safe and within allowed bounds
func validatePath(path string) error {
	if path == "" {
		return fmt.Errorf("path cannot be empty")
	}
	if len(path) > MaxPathLength {
		return fmt.Errorf("path too long (max %d characters)", MaxPathLength)
	}
	// Block directory traversal attempts
	if strings.Contains(path, "..") {
		return fmt.Errorf("path contains invalid sequence")
	}
	// Block null bytes
	if strings.ContainsAny(path, "\x00") {
		return fmt.Errorf("path contains null byte")
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
		return fmt.Errorf("query contains null byte")
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
			return 0, 0, fmt.Errorf("invalid page number")
		}
	}

	if perPageStr != "" {
		if parsed, parseErr := strconv.Atoi(perPageStr); parseErr == nil && parsed > 0 {
			perPage = parsed
			if perPage > MaxPerPage {
				perPage = MaxPerPage
			}
		} else if perPageStr != "" {
			return 0, 0, fmt.Errorf("invalid per_page value")
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
		return "", "", fmt.Errorf("syncweb not configured")
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

// handleSyncwebEvents returns recent sync events.
// GET /api/syncweb/events
func (c *ServeCmd) handleSyncwebEvents(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	events := c.sw.GetEvents()
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(events)
}

// handleSyncwebFolders returns a list of configured Syncweb folders.
// GET /api/syncweb/folders
func (c *ServeCmd) handleSyncwebFolders(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	folders := c.sw.GetFolders()
	for i := range folders {
		folders[i].Path = "" // Hide local path from API
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(folders)
}

// handleSyncwebFoldersAdd adds a new sync folder.
// POST /api/syncweb/folders/add
// Body: {"id": "...", "path": "..."}
func (c *ServeCmd) handleSyncwebFoldersAdd(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	var req struct {
		ID   string `json:"id"`
		Path string `json:"path"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid request body"})
		return
	}

	// For now, we use SendReceive as default
	if err := c.sw.AddFolder(req.ID, req.ID, req.Path, config.FolderTypeSendReceive); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	w.WriteHeader(http.StatusAccepted)
	fmt.Fprintln(w, "Folder add request accepted")
}

// handleSyncwebFoldersDelete removes a sync folder.
// POST /api/syncweb/folders/delete
// Body: {"id": "..."}
func (c *ServeCmd) handleSyncwebFoldersDelete(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	var req struct {
		ID string `json:"id"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid request body"})
		return
	}

	if err := c.sw.DeleteFolder(req.ID); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	w.WriteHeader(http.StatusAccepted)
	fmt.Fprintln(w, "Folder deletion request accepted")
}

// handleSyncwebLs lists global files in a Syncweb folder.
// GET /api/syncweb/ls?folder=...&prefix=...
func (c *ServeCmd) handleSyncwebLs(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	folderID := r.URL.Query().Get("folder")
	prefix := r.URL.Query().Get("prefix")

	// Validate folder ID if provided
	if folderID != "" && folderID != "/" {
		if err := validateFolderID(folderID); err != nil {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid folder ID: " + err.Error()})
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
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusNotFound)
			json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Folder not found or not configured"})
			return
		}
	}

	// Validate prefix if provided
	if prefix != "" {
		if err := validateQuery(prefix); err != nil {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid prefix: " + err.Error()})
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
				cancel()
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
		cancel()
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

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(results)
}

// handleSyncwebDownload triggers a download for a Syncweb file.
// POST /api/syncweb/download
// Body: {"path": "sync://..."}
func (c *ServeCmd) handleSyncwebDownload(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	if r.Method != http.MethodPost {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusMethodNotAllowed)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Method not allowed"})
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
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Path required"})
		return
	}

	// Validate path
	if err := validatePath(req.Path); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid path: " + err.Error()})
		return
	}

	localPath, folderID, err := c.sw.ResolveLocalPath(req.Path)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	if c.isPathBlacklisted(localPath) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusForbidden)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Access denied"})
		return
	}

	folderPath, ok := c.sw.GetFolderPath(folderID)
	if !ok {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Folder root not found"})
		return
	}
	relativePath, _ := filepath.Rel(folderPath, localPath)

	if err := c.sw.Unignore(folderID, relativePath); err != nil {
		slog.Error("Syncweb download trigger failed", "path", req.Path, "error", err)
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: fmt.Sprintf("Download trigger failed: %v", err)})
		return
	}

	w.WriteHeader(http.StatusAccepted)
	fmt.Fprintln(w, "Download triggered")
}

// handleSyncwebToggle toggles Syncweb between online and offline modes.
// POST /api/syncweb/toggle
// Body: {"offline": bool}
func (c *ServeCmd) handleSyncwebToggle(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()

	if c.sw == nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured"})
		return
	}

	var req struct {
		Offline bool `json:"offline"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid request body"})
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
				w.Header().Set("Content-Type", "application/json")
				w.WriteHeader(http.StatusInternalServerError)
				json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Failed to restart Syncweb: " + err.Error()})
				return
			}
		}
	}

	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]bool{"offline": !c.sw.IsRunning()})
}

// handleSyncwebStatus returns the current status of Syncweb.
func (c *ServeCmd) handleSyncwebStatus(w http.ResponseWriter, r *http.Request) {
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

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{
		"status":  status,
		"offline": offline,
	})
}

// handleSyncwebFind searches for files across all Syncweb folders.
// GET /api/syncweb/find?q=...
func (c *ServeCmd) handleSyncwebFind(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	query := r.URL.Query().Get("q")
	if query == "" {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Query required"})
		return
	}

	// Validate query
	if err := validateQuery(query); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid query: " + err.Error()})
		return
	}

	re, err := regexp.Compile("(?i)" + query)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid regex: " + err.Error()})
		return
	}

	var results []models.LsEntry
	cfg := c.sw.Node.Cfg.RawCopy()
	for _, f := range cfg.Folders {
		seq, cancel := c.sw.Node.App.Internals.AllGlobalFiles(f.ID)

		for meta := range seq {
			// Check context for cancellation
			if r.Context().Err() != nil {
				cancel()
				w.Header().Set("Content-Type", "application/json")
				json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Request cancelled"})
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
		cancel()
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(results)
}

// handleSyncwebStat returns detailed metadata for a file.
// GET /api/syncweb/stat?path=sync://...
func (c *ServeCmd) handleSyncwebStat(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	path := r.URL.Query().Get("path")
	if path == "" {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Path required"})
		return
	}

	// Validate path
	if err := validatePath(path); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid path: " + err.Error()})
		return
	}

	localPath, folderID, err := c.sw.ResolveLocalPath(path)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	folderPath, _ := c.sw.GetFolderPath(folderID)
	relPath, _ := filepath.Rel(folderPath, localPath)

	info, ok, err := c.sw.GetGlobalFileInfo(folderID, relPath)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}
	if !ok {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "File not found in cluster"})
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

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(res)
}

// handleSyncwebDevices returns a list of configured Syncthing devices.
// GET /api/syncweb/devices
func (c *ServeCmd) handleSyncwebDevices(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	devices := c.sw.GetDevices()
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(devices)
}

// handleSyncwebPendingDevices returns a list of rejected/pending device IDs.
// GET /api/syncweb/pending
func (c *ServeCmd) handleSyncwebPendingDevices(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	pending := c.sw.GetPendingDevices()
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(pending)
}

// handleSyncwebDevicesAdd adds a new device.
// POST /api/syncweb/devices/add
// Body: {"id": "...", "name": "...", "introducer": bool}
func (c *ServeCmd) handleSyncwebDevicesAdd(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	var req struct {
		ID         string `json:"id"`
		Name       string `json:"name"`
		Introducer bool   `json:"introducer"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid request body"})
		return
	}

	if err := c.sw.AddDevice(req.ID, req.Name, req.Introducer); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	w.WriteHeader(http.StatusAccepted)
	fmt.Fprintln(w, "Device add request accepted")
}

// handleSyncwebPendingFolders returns a list of pending folder invitations.
// GET /api/syncweb/pending-folders
func (c *ServeCmd) handleSyncwebPendingFolders(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	pending := c.sw.GetPendingFolders()
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(pending)
}

// handleSyncwebFoldersJoin joins a pending folder.
// POST /api/syncweb/folders/join
// Body: {"folder_id": "...", "device_id": "...", "path": "..."}
func (c *ServeCmd) handleSyncwebFoldersJoin(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	var req struct {
		FolderID string `json:"folder_id"`
		DeviceID string `json:"device_id"`
		Path     string `json:"path"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid request body"})
		return
	}

	// Add folder if it doesn't exist
	if err := c.sw.AddFolder(req.FolderID, req.FolderID, req.Path, 0); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	// Share with device if specified
	if req.DeviceID != "" {
		if err := c.sw.AddFolderDevice(req.FolderID, req.DeviceID); err != nil {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
			return
		}
	}

	w.WriteHeader(http.StatusAccepted)
	fmt.Fprintln(w, "Folder join request accepted")
}

// handleSyncwebDevicesDelete removes a device.
// POST /api/syncweb/devices/delete
// Body: {"id": "..."}
func (c *ServeCmd) handleSyncwebDevicesDelete(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	var req struct {
		ID string `json:"id"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid request body"})
		return
	}

	if err := c.sw.DeleteDevice(req.ID); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	w.WriteHeader(http.StatusAccepted)
	fmt.Fprintln(w, "Device deletion request accepted")
}

// handleSyncwebCompletion returns folder completion percentage for a device.
// GET /api/syncweb/completion?device_id=...&folder_id=...
func (c *ServeCmd) handleSyncwebCompletion(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	deviceIDStr := r.URL.Query().Get("device_id")
	folderID := r.URL.Query().Get("folder_id")

	if deviceIDStr == "" || folderID == "" {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Missing device_id or folder_id parameter"})
		return
	}

	// Validate folder ID
	if err := validateFolderID(folderID); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid folder_id: " + err.Error()})
		return
	}

	deviceID, err := protocol.DeviceIDFromString(deviceIDStr)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid device_id: " + err.Error()})
		return
	}

	completion, err := c.sw.GetCompletion(deviceID, folderID)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(completion)
}

// handleSyncwebTree returns folder tree structure for browsing.
// GET /api/syncweb/tree?folder_id=...&prefix=...&levels=-1&dirs_only=false
func (c *ServeCmd) handleSyncwebTree(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	folderID := r.URL.Query().Get("folder_id")
	prefix := r.URL.Query().Get("prefix")
	levelsStr := r.URL.Query().Get("levels")
	dirsOnlyStr := r.URL.Query().Get("dirs_only")

	if folderID == "" {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Missing folder_id parameter"})
		return
	}

	// Validate folder ID
	if err := validateFolderID(folderID); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid folder_id: " + err.Error()})
		return
	}

	// Validate prefix if provided
	if prefix != "" {
		if err := validateQuery(prefix); err != nil {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid prefix: " + err.Error()})
			return
		}
	}

	levels := -1
	if levelsStr != "" {
		if parsed, err := strconv.Atoi(levelsStr); err == nil {
			levels = parsed
			// Limit levels to prevent excessive recursion
			if levels > 100 {
				levels = 100
			}
		}
	}

	dirsOnly := false
	if dirsOnlyStr == "true" {
		dirsOnly = true
	}

	tree, err := c.sw.GetGlobalTree(folderID, prefix, levels, dirsOnly)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{"tree": tree})
}

// handleSyncwebLocalChanged returns locally changed files for a folder.
// GET /api/syncweb/local-changed?folder_id=...&page=1&per_page=100
func (c *ServeCmd) handleSyncwebLocalChanged(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	folderID := r.URL.Query().Get("folder_id")
	pageStr := r.URL.Query().Get("page")
	perPageStr := r.URL.Query().Get("per_page")

	if folderID == "" {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Missing folder_id parameter"})
		return
	}

	// Validate folder ID
	if err := validateFolderID(folderID); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid folder_id: " + err.Error()})
		return
	}

	// Validate and parse pagination params
	page, perPage, err := validatePaginationParams(pageStr, perPageStr)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	files, err := c.sw.GetLocalChangedFiles(folderID, page, perPage)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{"files": files, "page": page, "per_page": perPage})
}

// handleSyncwebNeed returns paginated list of needed files for a folder.
// GET /api/syncweb/need?folder_id=...&page=1&per_page=100
func (c *ServeCmd) handleSyncwebNeed(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	folderID := r.URL.Query().Get("folder_id")
	pageStr := r.URL.Query().Get("page")
	perPageStr := r.URL.Query().Get("per_page")

	if folderID == "" {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Missing folder_id parameter"})
		return
	}

	// Validate folder ID
	if err := validateFolderID(folderID); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid folder_id: " + err.Error()})
		return
	}

	// Validate and parse pagination params
	page, perPage, err := validatePaginationParams(pageStr, perPageStr)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	remote, local, queued, err := c.sw.GetNeedFiles(folderID, page, perPage)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{
		"remote":   remote,
		"local":    local,
		"queued":   queued,
		"page":     page,
		"per_page": perPage,
	})
}

// handleSyncwebRemoteNeed returns files needed by a specific remote device.
// GET /api/syncweb/remote-need?folder_id=...&device_id=...&page=1&per_page=100
func (c *ServeCmd) handleSyncwebRemoteNeed(w http.ResponseWriter, r *http.Request) {
	c.swMu.Lock()
	defer c.swMu.Unlock()
	if c.sw == nil || !c.sw.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	folderID := r.URL.Query().Get("folder_id")
	deviceIDStr := r.URL.Query().Get("device_id")
	pageStr := r.URL.Query().Get("page")
	perPageStr := r.URL.Query().Get("per_page")

	if folderID == "" || deviceIDStr == "" {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Missing folder_id or device_id parameter"})
		return
	}

	// Validate folder ID
	if err := validateFolderID(folderID); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid folder_id: " + err.Error()})
		return
	}

	deviceID, err := protocol.DeviceIDFromString(deviceIDStr)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid device_id: " + err.Error()})
		return
	}

	// Validate and parse pagination params
	page, perPage, err := validatePaginationParams(pageStr, perPageStr)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	files, err := c.sw.GetRemoteNeedFiles(folderID, deviceID, page, perPage)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{"files": files, "page": page, "per_page": perPage})
}
