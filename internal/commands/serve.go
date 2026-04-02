package commands

import (
	"context"
	"encoding/json"
	"errors"
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
	Port      int      `help:"Port to listen on"                                 default:"8889" short:"p"`
	Listen    string   `help:"Address to listen on (default: 127.0.0.1)"`
	PublicDir string   `help:"Override embedded web assets with local directory"`
	ReadOnly  bool     `help:"Disable file modifications"`
	SafeRoots []string `help:"Restrict local filesystem access to these directories"`

	APIToken string `kong:"-"`

	// Syncweb instance (dependency injection for testability)
	sw   syncweb.Engine
	swMu sync.RWMutex
}

// Help displays examples for the serve command
func (c *ServeCmd) Help() string {
	return serveExamples
}

func (c *ServeCmd) Run(g *SyncwebCmd) error {
	models.SetupLogging(g.Verbose)

	// Default safe roots to home directory if not specified
	if len(c.SafeRoots) == 0 {
		home, err := os.UserHomeDir()
		if err == nil {
			c.SafeRoots = []string{home}
		}
	}

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

	// Folder management endpoints
	mux.HandleFunc("/api/syncweb/folders/pause", c.AuthMiddleware(c.handleSyncwebFolderPause))
	mux.HandleFunc("/api/syncweb/folders/resume", c.AuthMiddleware(c.handleSyncwebFolderResume))
	mux.HandleFunc("/api/syncweb/folders/scan-subdirs", c.AuthMiddleware(c.handleSyncwebFolderScanSubdirs))
	mux.HandleFunc("/api/syncweb/folders/remove-devices", c.AuthMiddleware(c.handleSyncwebFolderRemoveDevices))

	// Device management endpoints
	mux.HandleFunc("/api/syncweb/devices/pause", c.AuthMiddleware(c.handleSyncwebDevicePause))
	mux.HandleFunc("/api/syncweb/devices/resume", c.AuthMiddleware(c.handleSyncwebDeviceResume))
	mux.HandleFunc("/api/syncweb/devices/set-addresses", c.AuthMiddleware(c.handleSyncwebDeviceSetAddresses))

	// Ignores management endpoints
	mux.HandleFunc("/api/syncweb/ignores/add", c.AuthMiddleware(c.handleSyncwebIgnoresAdd))

	// Status and health endpoints
	mux.HandleFunc("/api/syncweb/idle", c.AuthMiddleware(c.handleSyncwebIdle))

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

	logger := slog.Default().With("addr", listenAddr)
	logger.Info("Syncweb server starting")
	logger.Debug("API token", "token", c.APIToken)

	// Save address and token for CLI discovery
	addrFile := filepath.Join(g.SyncwebHome, "syncweb.addr")
	if err := os.WriteFile(addrFile, []byte(listenAddr), 0o600); err != nil {
		logger.Warn("Failed to write address file", "error", err)
	}
	defer os.Remove(addrFile)

	tokenFile := filepath.Join(g.SyncwebHome, "syncweb.token")
	if err := os.WriteFile(tokenFile, []byte(c.APIToken), 0o600); err != nil {
		logger.Warn("Failed to write token file", "error", err)
	}
	defer os.Remove(tokenFile)

	server := &http.Server{
		Addr:         listenAddr,
		Handler:      mux,
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 0,
		IdleTimeout:  120 * time.Second,
	}

	go func() {
		if err := server.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			logger.Error("HTTP server failed", "error", err)
		}
	}()

	// Wait for context cancellation
	<-g.Context.Done()
	logger.Info("Stopping Syncweb server")

	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer shutdownCancel()

	if err := server.Shutdown(shutdownCtx); err != nil {
		logger.Error("HTTP server shutdown failed", "error", err)
	}

	c.swMu.Lock()
	if c.sw != nil {
		c.sw.Stop()
	}
	c.swMu.Unlock()

	return nil
}

func (c *ServeCmd) AuthMiddleware(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Set basic security headers
		w.Header().Set("X-Frame-Options", "SAMEORIGIN")
		w.Header().Set("X-Content-Type-Options", "nosniff")

		remoteHost, _, _ := net.SplitHostPort(r.RemoteAddr)
		isLocal := IsLocalhost(remoteHost)

		// Host header validation (DNS rebinding protection)
		if err := validateHostHeader(r, isLocal); err != nil {
			http.Error(w, err.Error(), http.StatusForbidden)
			return
		}

		// Token validation
		if err := validateToken(r, c.APIToken, isLocal); err != nil {
			http.Error(w, err.Error(), http.StatusUnauthorized)
			return
		}

		// Basic CSRF protection for state-changing requests
		if err := validateCSRF(r, isLocal); err != nil {
			http.Error(w, err.Error(), http.StatusForbidden)
			return
		}

		next(w, r)
	}
}

func validateHostHeader(r *http.Request, isLocal bool) error {
	if !isLocal {
		return nil
	}
	host, _, _ := net.SplitHostPort(r.Host)
	if host == "" {
		host = r.Host
	}
	if !IsLocalhost(host) {
		return errors.New("host check failed (DNS rebinding protection)")
	}
	return nil
}

func validateToken(r *http.Request, apiToken string, isLocal bool) error {
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
	if token != apiToken && !isLocal {
		return errors.New("unauthorized")
	}
	return nil
}

func validateCSRF(r *http.Request, isLocal bool) error {
	if r.Method == http.MethodGet || r.Method == http.MethodHead || r.Method == http.MethodOptions {
		return nil
	}
	origin := r.Header.Get("Origin")
	if origin == "" {
		origin = r.Header.Get("Referer")
	}
	if origin != "" && isLocal {
		originHost := strings.TrimPrefix(origin, "http://")
		originHost = strings.TrimPrefix(originHost, "https://")
		originHost, _, _ = strings.Cut(originHost, "/")
		if !IsLocalhost(originHost) {
			return errors.New("CSRF block")
		}
	}
	return nil
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
	cleanPath, err := filepath.Abs(filepath.Clean(path))
	if err != nil {
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
		"/var/lib",
		"/var/log",
	}

	for _, sensitive := range sensitivePaths {
		if cleanPath == sensitive || strings.HasPrefix(cleanPath, sensitive+"/") {
			return true
		}
	}

	// Block sensitive user files in any directory
	sensitiveFiles := []string{
		".ssh",
		".gnupg",
		".config",
		".bash_history",
		".zsh_history",
		".netrc",
		".aws",
		".docker",
		".kube",
		"id_rsa",
		"id_ed25519",
		"id_ecdsa",
		"id_dsa",
	}

	for _, sf := range sensitiveFiles {
		if filepath.Base(cleanPath) == sf || strings.Contains(cleanPath, "/"+sf+"/") {
			return true
		}
	}

	// Check if path is within any of the safe roots
	if len(c.SafeRoots) > 0 {
		isSafe := false
		for _, root := range c.SafeRoots {
			absRoot, err := filepath.Abs(root)
			if err != nil {
				continue
			}
			if cleanPath == absRoot || strings.HasPrefix(cleanPath, absRoot+string(filepath.Separator)) {
				isSafe = true
				break
			}
		}
		if !isSafe {
			return true
		}
	}

	// Block paths with null bytes
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
		if len(c.SafeRoots) > 0 {
			path = c.SafeRoots[0]
		} else {
			path = "/"
		}
	}

	// Validate path
	if err := validatePath(path); err != nil {
		http.Error(w, "Invalid path: "+err.Error(), http.StatusBadRequest)
		return
	}

	// Check blacklist and safe roots
	if c.isPathBlacklisted(path) {
		http.Error(w, "Access denied", http.StatusForbidden)
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
