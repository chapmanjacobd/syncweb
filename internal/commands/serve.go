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
	"sync"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/models"
	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
	"github.com/chapmanjacobd/syncweb/web"
)

// Serve command examples
const serveExamples = `
Examples:
  # Start the web UI server (default port 8889)
  syncweb serve

  # Start on custom port
  syncweb serve -p 9000

  # Listen on all interfaces
  syncweb serve --listen=0.0.0.0

  # Use local web assets directory
  syncweb serve --public-dir=./web/dist

  # Enable read-only mode
  syncweb serve --read-only
`

type ServeCmd struct {
	Port      int    `help:"Port to listen on"                                 default:"8889" short:"p"`
	Listen    string `help:"Address to listen on (default: 127.0.0.1)"`
	PublicDir string `help:"Override embedded web assets with local directory"`
	ReadOnly  bool   `help:"Disable file modifications"`

	APIToken string `kong:"-"`

	// Syncweb instance (dependency injection for testability)
	sw   *syncweb.Syncweb
	swMu sync.RWMutex
}

// Help displays examples for the serve command
func (c *ServeCmd) Help() string {
	return serveExamples
}

func (c *ServeCmd) Run(g *SyncwebCmd) error {
	models.SetupLogging(g.Verbose)

	// Use environment variable for API token if set (for testing)
	if envToken := os.Getenv("SYNCWEB_API_TOKEN"); envToken != "" {
		c.APIToken = envToken
	} else {
		c.APIToken = utils.RandomString(32)
	}

	c.setupSyncweb(g)

	mux := http.NewServeMux()

	// API Routes
	mux.HandleFunc("/api/syncweb/folders", c.AuthMiddleware(c.handleSyncwebFolders))
	mux.HandleFunc("/api/syncweb/folders/add", c.AuthMiddleware(c.handleSyncwebFoldersAdd))
	mux.HandleFunc("/api/syncweb/folders/delete", c.AuthMiddleware(c.handleSyncwebFoldersDelete))
	mux.HandleFunc("/api/syncweb/folders/join", c.AuthMiddleware(c.handleSyncwebFoldersJoin))
	mux.HandleFunc("/api/syncweb/pending-folders", c.AuthMiddleware(c.handleSyncwebPendingFolders))
	mux.HandleFunc("/api/syncweb/ls", c.AuthMiddleware(c.handleSyncwebLs))
	mux.HandleFunc("/api/syncweb/find", c.AuthMiddleware(c.handleSyncwebFind))
	mux.HandleFunc("/api/syncweb/stat", c.AuthMiddleware(c.handleSyncwebStat))
	mux.HandleFunc("/api/syncweb/download", c.AuthMiddleware(c.handleSyncwebDownload))
	mux.HandleFunc("/api/syncweb/toggle", c.AuthMiddleware(c.handleSyncwebToggle))
	mux.HandleFunc("/api/syncweb/status", c.AuthMiddleware(c.handleSyncwebStatus))
	mux.HandleFunc("/api/syncweb/events", c.AuthMiddleware(c.handleSyncwebEvents))
	mux.HandleFunc("/api/syncweb/devices", c.AuthMiddleware(c.handleSyncwebDevices))
	mux.HandleFunc("/api/syncweb/pending", c.AuthMiddleware(c.handleSyncwebPendingDevices))
	mux.HandleFunc("/api/syncweb/devices/add", c.AuthMiddleware(c.handleSyncwebDevicesAdd))
	mux.HandleFunc("/api/syncweb/devices/delete", c.AuthMiddleware(c.handleSyncwebDevicesDelete))

	// New Syncthing Contract endpoints
	mux.HandleFunc("/api/syncweb/completion", c.AuthMiddleware(c.handleSyncwebCompletion))
	mux.HandleFunc("/api/syncweb/tree", c.AuthMiddleware(c.handleSyncwebTree))
	mux.HandleFunc("/api/syncweb/local-changed", c.AuthMiddleware(c.handleSyncwebLocalChanged))
	mux.HandleFunc("/api/syncweb/need", c.AuthMiddleware(c.handleSyncwebNeed))
	mux.HandleFunc("/api/syncweb/remote-need", c.AuthMiddleware(c.handleSyncwebRemoteNeed))

	mux.HandleFunc("/api/mounts", c.AuthMiddleware(c.handleMounts))
	mux.HandleFunc("/api/mount", c.AuthMiddleware(c.handleMount))
	mux.HandleFunc("/api/unmount", c.AuthMiddleware(c.handleUnmount))
	mux.HandleFunc("/api/local/ls", c.AuthMiddleware(c.handleLocalLs))
	mux.HandleFunc("/api/raw", c.AuthMiddleware(c.handleRaw))

	// File Management Routes
	mux.HandleFunc("/api/file/move", c.AuthMiddleware(c.handleFileMove))
	mux.HandleFunc("/api/file/copy", c.AuthMiddleware(c.handleFileCopy))
	mux.HandleFunc("/api/file/delete", c.AuthMiddleware(c.handleFileDelete))

	// Static Files
	if c.PublicDir != "" {
		mux.Handle("/", http.FileServer(http.Dir(c.PublicDir)))
	} else {
		// Serve embedded web assets
		mux.Handle("/", http.FileServer(http.FS(web.FS)))
	}

	// Default to localhost for security (DNS rebinding protection)
	listenAddr := c.Listen
	if listenAddr == "" {
		listenAddr = fmt.Sprintf("127.0.0.1:%d", c.Port)
	} else if !strings.Contains(listenAddr, ":") {
		// If only port specified (e.g., "8889"), prepend localhost
		listenAddr = "127.0.0.1:" + listenAddr
	}

	slog.Info("Syncweb server starting", "addr", listenAddr)
	slog.Debug("API token", "token", c.APIToken)

	server := &http.Server{
		Addr:         listenAddr,
		Handler:      mux,
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 0,
		IdleTimeout:  120 * time.Second,
	}

	return server.ListenAndServe()
}

func (c *ServeCmd) AuthMiddleware(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Set basic security headers
		w.Header().Set("X-Frame-Options", "SAMEORIGIN")
		w.Header().Set("X-Content-Type-Options", "nosniff")

		remoteHost, _, _ := net.SplitHostPort(r.RemoteAddr)
		isLocal := IsLocalhost(remoteHost)

		// Host header validation (DNS rebinding protection)
		// If connection is from localhost, only allow localhost-related Host headers
		if isLocal {
			host, _, _ := net.SplitHostPort(r.Host)
			if host == "" {
				host = r.Host // No port in Host header
			}
			if !IsLocalhost(host) {
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
				// Extract host from origin URL
				originHost := strings.TrimPrefix(origin, "http://")
				originHost = strings.TrimPrefix(originHost, "https://")
				originHost, _, _ = strings.Cut(originHost, "/")
				if !IsLocalhost(originHost) {
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

	// Validate path
	if err := validatePath(path); err != nil {
		http.Error(w, "Invalid path: "+err.Error(), http.StatusBadRequest)
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
	// Normalize the path to resolve any .. or . components
	cleanPath := filepath.Clean(path)

	// Block directory traversal attempts
	if strings.Contains(cleanPath, "..") {
		return true
	}

	// Block absolute paths to sensitive system directories
	sensitivePaths := []string{
		"/etc",
		"/proc",
		"/sys",
		"/dev",
		"/root",
		"/boot",
	}

	for _, sensitive := range sensitivePaths {
		if cleanPath == sensitive || strings.HasPrefix(cleanPath, sensitive+"/") {
			return true
		}
	}

	// Block paths with null bytes or other dangerous characters
	if strings.ContainsAny(cleanPath, "\x00") {
		return true
	}

	return false
}

func (c *ServeCmd) handleMounts(w http.ResponseWriter, _ *http.Request) {
	devices, err := utils.GetBlockDevices()
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeOK(w, devices)
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

	// Validate path
	if err := validatePath(path); err != nil {
		http.Error(w, "Invalid path: "+err.Error(), http.StatusBadRequest)
		return
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

	writeOK(w, results)
}

// IsLocalhost checks if a host is a localhost variant
// This includes 127.0.0.0/8 range, ::1, and "localhost" hostname
func IsLocalhost(host string) bool {
	// Strip port if present
	hostOnly, _, err := net.SplitHostPort(host)
	if err != nil {
		hostOnly = host
	}

	// Check common localhost variations
	if hostOnly == "localhost" || hostOnly == "127.0.0.1" || hostOnly == "::1" {
		return true
	}

	// Check if it's an IP address in the loopback range
	if ip := net.ParseIP(hostOnly); ip != nil {
		if ip.IsLoopback() {
			return true
		}
	}

	return false
}
