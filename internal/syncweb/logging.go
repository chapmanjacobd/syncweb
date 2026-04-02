package syncweb

import (
	"context"
	"log/slog"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
)

type syncthingRedirectHandler struct {
	fileHandler slog.Handler
	next        slog.Handler
	isSyncthing bool
}

func (h *syncthingRedirectHandler) Enabled(ctx context.Context, level slog.Level) bool {
	return h.fileHandler.Enabled(ctx, level) || h.next.Enabled(ctx, level)
}

func (h *syncthingRedirectHandler) Handle(
	ctx context.Context,
	r slog.Record, //nolint:gocritic // slog.Handler interface requires value receiver
) error {
	isSyncthing := h.isSyncthing
	if !isSyncthing && r.PC != 0 {
		fs := runtime.CallersFrames([]uintptr{r.PC})
		frame, _ := fs.Next()
		if strings.Contains(frame.Function, "github.com/syncthing/syncthing") ||
			strings.Contains(frame.Function, "github.com/chapmanjacobd/syncweb/internal/syncweb") {

			isSyncthing = true
		}
	}

	if !isSyncthing {
		r.Attrs(func(a slog.Attr) bool {
			if a.Key == "pkg" || a.Key == "log.pkg" {
				isSyncthing = true
				return false
			}
			return true
		})
	}

	if isSyncthing {
		return h.fileHandler.Handle(ctx, r)
	}
	return h.next.Handle(ctx, r)
}

func (h *syncthingRedirectHandler) WithAttrs(attrs []slog.Attr) slog.Handler {
	isSyncthing := h.isSyncthing
	for _, a := range attrs {
		if a.Key == "pkg" || a.Key == "log.pkg" {
			isSyncthing = true
			break
		}
	}
	return &syncthingRedirectHandler{
		fileHandler: h.fileHandler.WithAttrs(attrs),
		next:        h.next.WithAttrs(attrs),
		isSyncthing: isSyncthing,
	}
}

func (h *syncthingRedirectHandler) WithGroup(name string) slog.Handler {
	return &syncthingRedirectHandler{
		fileHandler: h.fileHandler.WithGroup(name),
		next:        h.next.WithGroup(name),
		isSyncthing: h.isSyncthing,
	}
}

var once sync.Once

// setupLogging redirects Syncthing-related logs to a file in the home directory
func setupLogging(homeDir string) {
	once.Do(func() {
		logPath := filepath.Join(homeDir, "syncthing.log")
		f, err := os.OpenFile(logPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644)
		if err != nil {
			slog.Warn("Failed to open syncthing log file", "path", logPath, "error", err)
			return
		}

		fileHandler := slog.NewTextHandler(f, &slog.HandlerOptions{
			Level: slog.LevelInfo,
		})

		currentDefault := slog.Default().Handler()

		newDefault := &syncthingRedirectHandler{
			fileHandler: fileHandler,
			next:        currentDefault,
		}

		slog.SetDefault(slog.New(newDefault))
		slog.Info("Syncthing logging redirected to file", "path", logPath, "pkg", "syncweb")
	})
}
