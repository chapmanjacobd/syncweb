package commands

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/models"
	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"
)

var (
	swInstance *syncweb.Syncweb
	swMu       sync.Mutex
)

func (c *ServeCmd) setupSyncweb(g *SyncwebCmd) {
	swMu.Lock()
	defer swMu.Unlock()
	sw, err := syncweb.NewSyncweb(g.SyncwebHome, "syncweb", "")
	if err != nil {
		slog.Warn("Failed to initialize Syncweb instance", "error", err)
	} else {
		swInstance = sw
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
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil {
		return "", "", fmt.Errorf("syncweb not configured")
	}
	return swInstance.ResolveLocalPath(path)
}

func (c *ServeCmd) serveSyncwebContent(w http.ResponseWriter, r *http.Request, folderID, path, localPath string) {
	swMu.Lock()
	if swInstance == nil || !swInstance.IsRunning() {
		swMu.Unlock()
		http.Error(w, "Syncweb not configured or offline", http.StatusServiceUnavailable)
		return
	}
	sw := swInstance
	swMu.Unlock()

	slog.Info("Serving remote Syncweb file via block pulling", "path", path)
	rs, err := sw.NewReadSeeker(r.Context(), folderID, strings.TrimPrefix(path, "syncweb://"+folderID+"/"))
	if err != nil {
		slog.Error("Failed to create SyncwebReadSeeker", "path", path, "error", err)
		http.Error(w, "Failed to stream remote file", http.StatusInternalServerError)
		return
	}
	http.ServeContent(w, r, filepath.Base(localPath), time.Now(), rs)
}

func (c *ServeCmd) addSyncwebRoots(resultsMap map[string]models.LsEntry, counts map[string]int, path string) {
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance != nil && (path == "/" || path == "") && swInstance.IsRunning() {
		for _, folder := range swInstance.GetFolders() {
			id := folder.ID
			entryPath := fmt.Sprintf("syncweb://%s/", id)
			name := id
			if localPath, ok := swInstance.GetFolderPath(id); ok {
				name += " (" + filepath.Base(localPath) + ")"
			}
			resultsMap[entryPath] = models.LsEntry{
				Name:  name,
				Path:  entryPath,
				IsDir: true,
			}
			counts[entryPath] = 1000 // High priority for roots
		}
	}
}

// handleSyncwebEvents returns recent sync events.
// GET /api/syncweb/events
func (c *ServeCmd) handleSyncwebEvents(w http.ResponseWriter, r *http.Request) {
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	events := swInstance.GetEvents()
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(events)
}

// handleSyncwebFolders returns a list of configured Syncweb folders.
// GET /api/syncweb/folders
func (c *ServeCmd) handleSyncwebFolders(w http.ResponseWriter, r *http.Request) {
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	folders := swInstance.GetFolders()
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
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
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
	if err := swInstance.AddFolder(req.ID, req.ID, req.Path, config.FolderTypeSendReceive); err != nil {
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
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
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

	if err := swInstance.DeleteFolder(req.ID); err != nil {
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
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	folderID := r.URL.Query().Get("folder")
	prefix := r.URL.Query().Get("prefix")

	if folderID != "" && folderID != "/" {
		// Security check: ensure the folder is one we actually have
		configuredFolders := swInstance.GetFolders()
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

	if prefix != "" && !strings.HasSuffix(prefix, "/") {
		prefix += "/"
	}

	resultsMap := make(map[string]models.LsEntry)
	counts := make(map[string]int)

	if folderID == "" || folderID == "/" {
		c.addSyncwebRoots(resultsMap, counts, "/")
	} else {
		seq, cancel := swInstance.Node.App.Internals.AllGlobalFiles(folderID)
		defer cancel()

		for meta := range seq {
			name := meta.Name
			if !strings.HasPrefix(name, prefix) || name == prefix {
				continue
			}

			rel := strings.TrimPrefix(name, prefix)
			parts := strings.Split(rel, "/")
			entryName := parts[0]
			isDir := len(parts) > 1

			fullSyncwebPath := fmt.Sprintf("syncweb://%s/%s", folderID, filepath.Join(prefix, entryName))
			if _, ok := resultsMap[fullSyncwebPath]; ok {
				continue
			}

			localPath, _, _ := swInstance.ResolveLocalPath(fullSyncwebPath)
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
// Body: {"path": "syncweb://..."}
func (c *ServeCmd) handleSyncwebDownload(w http.ResponseWriter, r *http.Request) {
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
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

	localPath, folderID, err := swInstance.ResolveLocalPath(req.Path)
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

	folderPath, ok := swInstance.GetFolderPath(folderID)
	if !ok {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Folder root not found"})
		return
	}
	relativePath, _ := filepath.Rel(folderPath, localPath)

	if err := swInstance.Unignore(folderID, relativePath); err != nil {
		slog.Error("Syncweb download trigger failed", "path", req.Path, "error", err)
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Download trigger failed: " + err.Error()})
		return
	}

	w.WriteHeader(http.StatusAccepted)
	fmt.Fprintln(w, "Download triggered")
}

// handleSyncwebToggle toggles Syncweb between online and offline modes.
// POST /api/syncweb/toggle
// Body: {"offline": bool}
func (c *ServeCmd) handleSyncwebToggle(w http.ResponseWriter, r *http.Request) {
	swMu.Lock()
	defer swMu.Unlock()

	if swInstance == nil {
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
		if swInstance.IsRunning() {
			slog.Info("Stopping Syncweb backend (Offline Mode)")
			swInstance.Stop()
		}
	} else {
		if !swInstance.IsRunning() {
			slog.Info("Starting Syncweb backend (Online Mode)")
			if err := swInstance.Start(); err != nil {
				slog.Error("Failed to restart Syncweb", "error", err)
				w.Header().Set("Content-Type", "application/json")
				w.WriteHeader(http.StatusInternalServerError)
				json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Failed to restart Syncweb: " + err.Error()})
				return
			}
		}
	}

	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]bool{"offline": !swInstance.IsRunning()})
}

// handleSyncwebStatus returns the current status of Syncweb.
func (c *ServeCmd) handleSyncwebStatus(w http.ResponseWriter, r *http.Request) {
	swMu.Lock()
	defer swMu.Unlock()

	status := "Not Configured"
	offline := true
	if swInstance != nil {
		if swInstance.IsRunning() {
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
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
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

	re, err := regexp.Compile("(?i)" + query)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Invalid regex: " + err.Error()})
		return
	}

	var results []models.LsEntry
	cfg := swInstance.Node.Cfg.RawCopy()
	for _, f := range cfg.Folders {
		seq, cancel := swInstance.Node.App.Internals.AllGlobalFiles(f.ID)
		for meta := range seq {
			if re.MatchString(meta.Name) || re.MatchString(filepath.Base(meta.Name)) {
				fullSyncwebPath := fmt.Sprintf("syncweb://%s/%s", f.ID, meta.Name)
				localPath, _, _ := swInstance.ResolveLocalPath(fullSyncwebPath)
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
// GET /api/syncweb/stat?path=syncweb://...
func (c *ServeCmd) handleSyncwebStat(w http.ResponseWriter, r *http.Request) {
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
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

	localPath, folderID, err := swInstance.ResolveLocalPath(path)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	folderPath, _ := swInstance.GetFolderPath(folderID)
	relPath, _ := filepath.Rel(folderPath, localPath)

	info, ok, err := swInstance.GetGlobalFileInfo(folderID, relPath)
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
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	devices := swInstance.GetDevices()
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(devices)
}

// handleSyncwebPendingDevices returns a list of rejected/pending device IDs.
// GET /api/syncweb/pending
func (c *ServeCmd) handleSyncwebPendingDevices(w http.ResponseWriter, r *http.Request) {
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	pending := swInstance.GetPendingDevices()
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(pending)
}

// handleSyncwebDevicesAdd adds a new device.
// POST /api/syncweb/devices/add
// Body: {"id": "...", "name": "...", "introducer": bool}
func (c *ServeCmd) handleSyncwebDevicesAdd(w http.ResponseWriter, r *http.Request) {
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
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

	if err := swInstance.AddDevice(req.ID, req.Name, req.Introducer); err != nil {
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
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: "Syncweb not configured or offline"})
		return
	}

	pending := swInstance.GetPendingFolders()
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(pending)
}

// handleSyncwebFoldersJoin joins a pending folder.
// POST /api/syncweb/folders/join
// Body: {"folder_id": "...", "device_id": "...", "path": "..."}
func (c *ServeCmd) handleSyncwebFoldersJoin(w http.ResponseWriter, r *http.Request) {
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
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
	if err := swInstance.AddFolder(req.FolderID, req.FolderID, req.Path, 0); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	// Share with device if specified
	if req.DeviceID != "" {
		if err := swInstance.AddFolderDevice(req.FolderID, req.DeviceID); err != nil {
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
	swMu.Lock()
	defer swMu.Unlock()
	if swInstance == nil || !swInstance.IsRunning() {
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

	if err := swInstance.DeleteDevice(req.ID); err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(models.ErrorResponse{Error: err.Error()})
		return
	}

	w.WriteHeader(http.StatusAccepted)
	fmt.Fprintln(w, "Device deletion request accepted")
}
