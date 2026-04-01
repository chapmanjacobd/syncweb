package commands

import (
	"errors"
	"fmt"
	"log/slog"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/models"
	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
	"github.com/chapmanjacobd/syncweb/internal/version"
	"github.com/sevlyar/go-daemon"
	"github.com/syncthing/syncthing/lib/config"
)

// Constants for automatic sync operations.
const (
	// AutoSyncInterval is the default interval for automatic sync operations.
	AutoSyncInterval = 30 * time.Second
)

type SyncwebCmd struct {
	models.CoreFlags    `embed:""`
	models.SyncwebFlags `embed:""`

	Create    SyncwebCreateCmd    `aliases:"init,in,share"           cmd:""                                                         help:"Create a syncweb folder"`
	Join      SyncwebJoinCmd      `aliases:"import,clone"            cmd:""                                                         help:"Join syncweb folders/devices"`
	Accept    SyncwebAcceptCmd    `aliases:"add"                     cmd:""                                                         help:"Add a device to syncweb"`
	Drop      SyncwebDropCmd      `aliases:"remove,reject"           cmd:""                                                         help:"Remove a device from syncweb"`
	Folders   SyncwebFoldersCmd   `aliases:"list-folders,lsf"        cmd:""                                                         help:"List Syncthing folders"`
	Devices   SyncwebDevicesCmd   `aliases:"list-devices,lsd"        cmd:""                                                         help:"List Syncthing devices"`
	Ls        SyncwebLsCmd        `aliases:"list"                    cmd:""                                                         help:"List files at the current directory level"`
	Find      SyncwebFindCmd      `aliases:"fd,search"               cmd:""                                                         help:"Search for files by filename, size, and modified date"`
	Scan      SyncwebScanCmd      `cmd:""                            help:"Trigger a scan on all folders"`
	Stat      SyncwebStatCmd      `cmd:""                            help:"Display detailed file status information from Syncthing"`
	Sort      SyncwebSortCmd      `cmd:""                            help:"Sort Syncthing files by multiple criteria"`
	Download  SyncwebDownloadCmd  `aliases:"dl,upload,unignore,sync" cmd:""                                                         help:"Mark file paths for download/sync"`
	Automatic SyncwebAutomaticCmd `cmd:""                            help:"Start syncweb-automatic daemon"`
	Serve     ServeCmd            `cmd:""                            help:"Start the Syncweb Web UI server"`
	Start     SyncwebStartCmd     `aliases:"restart"                 cmd:""                                                         help:"Start Syncweb daemon"`
	Stop      SyncwebStopCmd      `aliases:"shutdown,quit"           cmd:""                                                         help:"Stop Syncweb daemon"`
	Version   SyncwebVersionCmd   `cmd:""                            help:"Show Syncweb version"`
	Repl      SyncwebReplCmd      `aliases:"debug"                   cmd:""                                                         help:"Interactive REPL for debugging"`
}

func (c *SyncwebCmd) AfterApply() error {
	if c.SyncwebHome == "" {
		c.SyncwebHome = utils.GetConfigDir()
	}
	return nil
}

func (c *SyncwebCmd) WithSyncweb(fn func(s *syncweb.Syncweb) error) error {
	s, err := syncweb.NewSyncweb(c.SyncwebHome, "syncweb", "")
	if err != nil {
		return err
	}
	if err := s.Start(); err != nil {
		return err
	}
	defer s.Stop()
	return fn(s)
}

// SyncwebAutomaticCmd starts the syncweb-automatic daemon.
type SyncwebAutomaticCmd struct {
	Devices        bool     `help:"Auto-accept devices"`
	Folders        bool     `help:"Auto-join folders"`
	Local          bool     `default:"true"                                              help:"Only auto-accept local devices"`
	FoldersInclude []string `help:"Search for folders which match by label, ID, or path"`
	FoldersExclude []string `help:"Exclude folders which match by label, ID, or path"`
	FolderTypes    []string `help:"Filter folders by type"`
	DevicesInclude []string `help:"Search for devices which match by name or ID"`
	DevicesExclude []string `help:"Exclude devices which match by name or ID"`
	JoinNewFolders bool     `help:"Join non-existing folders from other devices"`
	Sort           string   `default:"-niche,-frecency"                                  help:"Sort criteria for download prioritization"`
}

func (c *SyncwebAutomaticCmd) Run(g *SyncwebCmd) error {
	slog.Info("Starting syncweb-automatic",
		"devices", c.Devices,
		"folders", c.Folders,
		"localOnly", c.Local,
		"joinNewFolders", c.JoinNewFolders)

	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		ticker := time.NewTicker(AutoSyncInterval)
		defer ticker.Stop()

		for {
			// 1. Auto-accept devices
			if c.Devices {
				pending := s.GetPendingDevices()
				for id := range pending {
					// Apply include/exclude filters
					if !matchesFilters(id, c.DevicesInclude, c.DevicesExclude) {
						continue
					}

					slog.Info("Auto-accepting device", "id", id)
					if err := s.AddDevice(id, "", false); err != nil {
						slog.Error("Failed to auto-accept device", "id", id, "error", err)
					}
				}
			}

			// 2. Auto-join folders
			if c.Folders {
				cfg := s.Node.Cfg.RawCopy()
				for _, dev := range cfg.Devices {
					pending, _ := s.Node.App.Internals.PendingFolders(dev.DeviceID)
					for folderID := range pending {
						// Apply filters
						if !matchesFilters(folderID, c.FoldersInclude, c.FoldersExclude) {
							continue
						}

						// Check folder type filter
						if len(c.FolderTypes) > 0 { //nolint:staticcheck // Empty branch: folder type filtering not yet implemented
							// Would need to get folder type from pending info
							// For now, skip type filtering
						}

						// Check if folder already exists
						exists := false
						for _, f := range cfg.Folders {
							if f.ID == folderID {
								exists = true
								break
							}
						}

						if !exists && !c.JoinNewFolders {
							continue
						}

						slog.Info("Auto-joining folder", "id", folderID, "from", dev.DeviceID)
						path := filepath.Join(g.SyncwebHome, folderID)

						if !exists {
							if err := s.AddFolder(folderID, folderID, path, config.FolderTypeReceiveOnly); err != nil {
								slog.Error("Failed to create folder", "id", folderID, "error", err)
								continue
							}
							if err := s.SetIgnores(folderID, []string{}); err != nil {
								slog.Error("Failed to set ignores", "id", folderID, "error", err)
								continue
							}
							if err := s.ResumeFolder(folderID); err != nil {
								slog.Error("Failed to resume folder", "id", folderID, "error", err)
								continue
							}
						}

						if err := s.AddFolderDevice(folderID, dev.DeviceID.String()); err != nil {
							slog.Error("Failed to share folder with device", "folder", folderID, "device", dev.DeviceID, "error", err)
						}
					}
				}
			}

			<-ticker.C
		}
	})
}

// matchesFilters checks if a string matches include/exclude filters.
func matchesFilters(s string, include, exclude []string) bool {
	// Check include filters
	if len(include) > 0 {
		matched := false
		for _, pattern := range include {
			if strings.Contains(s, pattern) {
				matched = true
				break
			}
		}
		if !matched {
			return false
		}
	}

	// Check exclude filters
	if len(exclude) > 0 {
		for _, pattern := range exclude {
			if strings.Contains(s, pattern) {
				return false
			}
		}
	}

	return true
}

// SyncwebStartCmd starts the Syncweb daemon.
type SyncwebStartCmd struct{}

func (c *SyncwebStartCmd) Run(g *SyncwebCmd) error {
	models.SetupLogging(g.Verbose)
	home := g.SyncwebHome
	if home == "" {
		home = utils.GetConfigDir()
	}

	cntxt := &daemon.Context{
		PidFileName: filepath.Join(home, "syncweb.pid"),
		PidFilePerm: 0o644,
		LogFileName: filepath.Join(home, "syncweb.log"),
		LogFilePerm: 0o640,
		WorkDir:     home,
		Umask:       0o27,
		Args:        []string{"syncweb", "serve", "--home", home},
	}

	d, err := cntxt.Reborn()
	if err != nil {
		return fmt.Errorf("unable to run: %w", err)
	}
	if d != nil {
		slog.Info("Syncweb daemon started", "pid", d.Pid)
		return nil
	}
	defer func() { _ = cntxt.Release() }()

	slog.Info("Syncweb daemon process starting")
	return nil
}

// SyncwebStopCmd stops the Syncweb daemon.
type SyncwebStopCmd struct{}

func (c *SyncwebStopCmd) Run(g *SyncwebCmd) error {
	models.SetupLogging(g.Verbose)
	home := g.SyncwebHome
	if home == "" {
		home = utils.GetConfigDir()
	}

	pidFile := filepath.Join(home, "syncweb.pid")
	if _, err := os.Stat(pidFile); os.IsNotExist(err) {
		return errors.New("syncweb daemon is not running (PID file not found)")
	}

	cntxt := &daemon.Context{
		PidFileName: pidFile,
	}

	d, err := cntxt.Search()
	if err != nil {
		return fmt.Errorf("unable to find daemon process: %w", err)
	}

	if err := d.Signal(syscall.SIGTERM); err != nil {
		return fmt.Errorf("unable to send signal to daemon: %w", err)
	}

	slog.Info("Syncweb daemon stop signal sent")
	return nil
}

// SyncwebVersionCmd shows the Syncweb version.
type SyncwebVersionCmd struct{}

func (c *SyncwebVersionCmd) Run(g *SyncwebCmd) error {
	fmt.Println(version.FullInfo())
	return nil
}
