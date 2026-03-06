package commands

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/models"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

type ServeCmd struct {
	Port      int    `short:"p" default:"8889" help:"Port to listen on"`
	PublicDir string `help:"Local directory for static assets"`
	ReadOnly  bool   `help:"Disable file modifications"`

	APIToken string `kong:"-"`
}

func (c *ServeCmd) Run(g *SyncwebCmd) error {
	models.SetupLogging(g.Verbose)
	c.APIToken = utils.RandomString(32)

	c.setupSyncweb(g)

	mux := http.NewServeMux()

	// API Routes
	mux.HandleFunc("/api/syncweb/folders", c.authMiddleware(c.handleSyncwebFolders))
	mux.HandleFunc("/api/syncweb/folders/add", c.authMiddleware(c.handleSyncwebFoldersAdd))
	mux.HandleFunc("/api/syncweb/folders/delete", c.authMiddleware(c.handleSyncwebFoldersDelete))
	mux.HandleFunc("/api/syncweb/folders/join", c.authMiddleware(c.handleSyncwebFoldersJoin))
	mux.HandleFunc("/api/syncweb/pending-folders", c.authMiddleware(c.handleSyncwebPendingFolders))
	mux.HandleFunc("/api/syncweb/ls", c.authMiddleware(c.handleSyncwebLs))
	mux.HandleFunc("/api/syncweb/find", c.authMiddleware(c.handleSyncwebFind))
	mux.HandleFunc("/api/syncweb/stat", c.authMiddleware(c.handleSyncwebStat))
	mux.HandleFunc("/api/syncweb/download", c.authMiddleware(c.handleSyncwebDownload))
	mux.HandleFunc("/api/syncweb/toggle", c.authMiddleware(c.handleSyncwebToggle))
	mux.HandleFunc("/api/syncweb/status", c.authMiddleware(c.handleSyncwebStatus))
	mux.HandleFunc("/api/syncweb/events", c.authMiddleware(c.handleSyncwebEvents))
	mux.HandleFunc("/api/syncweb/devices", c.authMiddleware(c.handleSyncwebDevices))
	mux.HandleFunc("/api/syncweb/pending", c.authMiddleware(c.handleSyncwebPendingDevices))
	mux.HandleFunc("/api/syncweb/devices/add", c.authMiddleware(c.handleSyncwebDevicesAdd))
	mux.HandleFunc("/api/syncweb/devices/delete", c.authMiddleware(c.handleSyncwebDevicesDelete))
	mux.HandleFunc("/api/mounts", c.authMiddleware(c.handleMounts))
	mux.HandleFunc("/api/mount", c.authMiddleware(c.handleMount))
	mux.HandleFunc("/api/unmount", c.authMiddleware(c.handleUnmount))
	mux.HandleFunc("/api/local/ls", c.authMiddleware(c.handleLocalLs))
	mux.HandleFunc("/api/raw", c.authMiddleware(c.handleRaw))

	// File Management Routes
	mux.HandleFunc("/api/file/move", c.authMiddleware(c.handleFileMove))
	mux.HandleFunc("/api/file/copy", c.authMiddleware(c.handleFileCopy))
	mux.HandleFunc("/api/file/delete", c.authMiddleware(c.handleFileDelete))

	// Static Files
	if c.PublicDir != "" {
		mux.Handle("/", http.FileServer(http.Dir(c.PublicDir)))
	} else {
		// Try to serve from web/ directory if it exists relative to the binary
		if info, err := os.Stat("web"); err == nil && info.IsDir() {
			mux.Handle("/", http.FileServer(http.Dir("web")))
		} else {
			mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
				fmt.Fprintf(w, "Syncweb Server Running. (No PublicDir configured and web/ directory not found)")
			})
		}
	}

	addr := fmt.Sprintf(":%d", c.Port)
	slog.Info("Syncweb server starting", "addr", addr, "token", c.APIToken)

	server := &http.Server{
		Addr:         addr,
		Handler:      mux,
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 0,
		IdleTimeout:  120 * time.Second,
	}

	return server.ListenAndServe()
}

func (c *ServeCmd) authMiddleware(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Set basic security headers
		w.Header().Set("X-Frame-Options", "SAMEORIGIN")
		w.Header().Set("X-Content-Type-Options", "nosniff")

		remoteHost, _, _ := net.SplitHostPort(r.RemoteAddr)
		isLocal := remoteHost == "127.0.0.1" || remoteHost == "::1"

		// Host header validation (DNS rebinding protection)
		// If we are on localhost, only allow localhost-related Host headers
		if isLocal {
			host, _, _ := net.SplitHostPort(r.Host)
			if host == "" {
				host = r.Host // No port in Host header
			}
			if host != "localhost" && host != "127.0.0.1" && host != "::1" {
				http.Error(w, "Host check failed (DNS rebinding protection)", http.StatusForbidden)
				return
			}
		}

		token := r.Header.Get("X-Syncweb-Token")
		if token == "" {
			token = r.URL.Query().Get("token")
		}
		if token == "" {
			cookie, err := r.Cookie("syncweb_token")
			if err == nil {
				token = cookie.Value
			}
		}

		if token != c.APIToken && !isLocal {
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}

		// Basic CSRF protection for state-changing requests
		if r.Method != http.MethodGet && r.Method != http.MethodHead && r.Method != http.MethodOptions {
			origin := r.Header.Get("Origin")
			if origin == "" {
				origin = r.Header.Get("Referer")
			}
			if origin != "" && isLocal {
				// The origin/referer should be local if it's a local browser making the request
				if !strings.Contains(origin, "localhost") && !strings.Contains(origin, "127.0.0.1") && !strings.Contains(origin, "::1") {
					http.Error(w, "CSRF block", http.StatusForbidden)
					return
				}
			}
		}

		next(w, r)
	}
}

func (c *ServeCmd) handleFileMove(w http.ResponseWriter, r *http.Request) {
	if c.ReadOnly {
		http.Error(w, "Read-only mode", http.StatusForbidden)
		return
	}
	var req struct {
		Src string `json:"src"`
		Dst string `json:"dst"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	src, _, err := c.resolveSyncwebPath(req.Src)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	dst, _, err := c.resolveSyncwebPath(req.Dst)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := os.Rename(src, dst); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	w.WriteHeader(http.StatusOK)
}

func (c *ServeCmd) handleFileCopy(w http.ResponseWriter, r *http.Request) {
	if c.ReadOnly {
		http.Error(w, "Read-only mode", http.StatusForbidden)
		return
	}
	var req struct {
		Src string `json:"src"`
		Dst string `json:"dst"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	src, _, err := c.resolveSyncwebPath(req.Src)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	dst, _, err := c.resolveSyncwebPath(req.Dst)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	info, err := os.Stat(src)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	if info.IsDir() {
		if err := utils.CopyDir(src, dst); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
	} else {
		if err := utils.CopyFile(src, dst); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
	}
	w.WriteHeader(http.StatusOK)
}

func (c *ServeCmd) handleFileDelete(w http.ResponseWriter, r *http.Request) {
	if c.ReadOnly {
		http.Error(w, "Read-only mode", http.StatusForbidden)
		return
	}
	var req struct {
		Path string `json:"path"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	path, _, err := c.resolveSyncwebPath(req.Path)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := os.RemoveAll(path); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	w.WriteHeader(http.StatusOK)
}

func (c *ServeCmd) handleRaw(w http.ResponseWriter, r *http.Request) {
	path := r.URL.Query().Get("path")
	if path == "" {
		http.Error(w, "Path required", http.StatusBadRequest)
		return
	}

	localPath, folderID, err := c.resolveSyncwebPath(path)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if c.isPathBlacklisted(localPath) {
		http.Error(w, "Access denied", http.StatusForbidden)
		return
	}

	if utils.FileExists(localPath) {
		http.ServeFile(w, r, localPath)
	} else {
		c.serveSyncwebContent(w, r, folderID, path, localPath)
	}
}

func (c *ServeCmd) isPathBlacklisted(path string) bool {
	// Add implementation for path blacklisting
	return false
}

func (c *ServeCmd) handleMounts(w http.ResponseWriter, r *http.Request) {
	devices, err := utils.GetBlockDevices()
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(devices)
}

func (c *ServeCmd) handleMount(w http.ResponseWriter, r *http.Request) {
	if c.ReadOnly {
		http.Error(w, "Read-only mode", http.StatusForbidden)
		return
	}
	var req struct {
		Device     string `json:"device"`
		Mountpoint string `json:"mountpoint"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := utils.Mount(req.Device, req.Mountpoint); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	w.WriteHeader(http.StatusOK)
}

func (c *ServeCmd) handleUnmount(w http.ResponseWriter, r *http.Request) {
	if c.ReadOnly {
		http.Error(w, "Read-only mode", http.StatusForbidden)
		return
	}
	var req struct {
		Mountpoint string `json:"mountpoint"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := utils.Unmount(req.Mountpoint); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	w.WriteHeader(http.StatusOK)
}

func (c *ServeCmd) handleLocalLs(w http.ResponseWriter, r *http.Request) {
	path := r.URL.Query().Get("path")
	if path == "" {
		path = "/"
	}

	entries, err := os.ReadDir(path)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	var results []models.LsEntry
	for _, entry := range entries {
		info, _ := entry.Info()
		results = append(results, models.LsEntry{
			Name:  entry.Name(),
			Path:  filepath.Join(path, entry.Name()),
			IsDir: entry.IsDir(),
			Size:  info.Size(),
			Local: true,
		})
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(results)
}
