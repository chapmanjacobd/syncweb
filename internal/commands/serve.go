package commands

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"time"

	"github.com/alecthomas/kong"
	"github.com/chapmanjacobd/syncweb/internal/models"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

type LsEntry struct {
	Name  string `json:"name"`
	Path  string `json:"path"`
	IsDir bool   `json:"is_dir"`
	Type  string `json:"type,omitempty"`
	Local bool   `json:"local"`
	Size  int64  `json:"size"`
}

type ServeCmd struct {
	models.CoreFlags    `embed:""`
	models.SyncwebFlags `embed:""`

	Port      int    `short:"p" default:"8888" help:"Port to listen on"`
	PublicDir string `help:"Local directory for static assets"`
	ReadOnly  bool   `help:"Disable file modifications"`

	APIToken string `kong:"-"`
}

func (c *ServeCmd) Run(ctx *kong.Context) error {
	models.SetupLogging(c.Verbose)
	c.APIToken = utils.RandomString(32)

	c.setupSyncweb()

	mux := http.NewServeMux()

	// API Routes
	mux.HandleFunc("/api/syncweb/folders", c.authMiddleware(c.handleSyncwebFolders))
	mux.HandleFunc("/api/syncweb/ls", c.authMiddleware(c.handleSyncwebLs))
	mux.HandleFunc("/api/syncweb/download", c.authMiddleware(c.handleSyncwebDownload))
	mux.HandleFunc("/api/syncweb/toggle", c.authMiddleware(c.handleSyncwebToggle))
	mux.HandleFunc("/api/syncweb/status", c.authMiddleware(c.handleSyncwebStatus))

	// File Management Routes
	mux.HandleFunc("/api/file/move", c.authMiddleware(c.handleFileMove))
	mux.HandleFunc("/api/file/copy", c.authMiddleware(c.handleFileCopy))
	mux.HandleFunc("/api/file/delete", c.authMiddleware(c.handleFileDelete))

	// Static Files
	if c.PublicDir != "" {
		mux.Handle("/", http.FileServer(http.Dir(c.PublicDir)))
	} else {
		mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
			// Try to serve web/index.html if it exists relative to the binary
			// This is a simple fallback for development
			if _, err := os.Stat("web/index.html"); err == nil {
				http.ServeFile(w, r, "web/index.html")
				return
			}
			fmt.Fprintf(w, "Syncweb Server Running. (No PublicDir configured and web/index.html not found)")
		})
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

		if token != c.APIToken {
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
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
	// Implementation for copying files
	http.Error(w, "Not implemented", http.StatusNotImplemented)
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

func (c *ServeCmd) isPathBlacklisted(path string) bool {
	// Add implementation for path blacklisting
	return false
}
