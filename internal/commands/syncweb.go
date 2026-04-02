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

	"github.com/sevlyar/go-daemon"
	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"

	"github.com/chapmanjacobd/syncweb/internal/models"
	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
	"github.com/chapmanjacobd/syncweb/internal/version"
)

// Constants for automatic sync operations
const (
	// AutoSyncInterval is the default interval for automatic sync operations
	AutoSyncInterval = 30 * time.Second
)

// Automatic command examples
const automaticExamples = `
Examples:
  # Auto-accept local devices only
  syncweb automatic --devices

  # Auto-join folders from local devices
  syncweb automatic --folders

  # Auto-accept devices and auto-join folders
  syncweb automatic --devices --folders

  # Auto-accept all devices (including remote)
  syncweb automatic --devices --global

  # Auto-join only specific folder types
  syncweb automatic --folders --folder-types=sendreceive

  # Auto-accept devices matching pattern
  syncweb automatic --devices --devices-include=server-
`

// Start command examples
const startExamples = `
Examples:
  # Start Syncweb daemon
  syncweb start

  # Start daemon (alias)
  syncweb restart
`

// Stop command examples
const stopExamples = `
Examples:
  # Stop Syncweb daemon
  syncweb stop

  # Stop daemon (aliases)
  syncweb shutdown
  syncweb quit
`

// Version command examples
const versionExamples = `
Examples:
  # Show version information
  syncweb version
`

type SyncwebCmd struct {
	models.CoreFlags    `embed:""`
	models.SyncwebFlags `embed:""`

	Create    SyncwebCreateCmd    `help:"Create a syncweb folder"                                 cmd:"" aliases:"init,in,share"`
	Join      SyncwebJoinCmd      `help:"Join syncweb folders/devices"                            cmd:"" aliases:"import,clone"`
	Accept    SyncwebAcceptCmd    `help:"Add a device to syncweb"                                 cmd:"" aliases:"add"`
	Drop      SyncwebDropCmd      `help:"Remove a device from syncweb"                            cmd:"" aliases:"remove,reject"`
	Folders   SyncwebFoldersCmd   `help:"List Syncthing folders"                                  cmd:"" aliases:"list-folders,lsf"`
	Devices   SyncwebDevicesCmd   `help:"List Syncthing devices"                                  cmd:"" aliases:"list-devices,lsd"`
	Ls        SyncwebLsCmd        `help:"List files at the current directory level"               cmd:"" aliases:"list"`
	Find      SyncwebFindCmd      `help:"Search for files by filename, size, and modified date"   cmd:"" aliases:"fd,search"`
	Scan      SyncwebScanCmd      `help:"Trigger a scan on all folders"                           cmd:""`
	Stat      SyncwebStatCmd      `help:"Display detailed file status information from Syncthing" cmd:""`
	Sort      SyncwebSortCmd      `help:"Sort Syncthing files by multiple criteria"               cmd:""`
	Download  SyncwebDownloadCmd  `help:"Mark file paths for download/sync"                       cmd:"" aliases:"dl,upload,unignore,sync"`
	Automatic SyncwebAutomaticCmd `help:"Start syncweb-automatic daemon"                          cmd:""`
	Serve     ServeCmd            `help:"Start the Syncweb Web UI server"                         cmd:""`
	Start     SyncwebStartCmd     `help:"Start Syncweb daemon"                                    cmd:"" aliases:"restart"`
	Stop      SyncwebStopCmd      `help:"Stop Syncweb daemon"                                     cmd:"" aliases:"shutdown,quit"`
	Version   SyncwebVersionCmd   `help:"Show Syncweb version"                                    cmd:""`
	Repl      SyncwebReplCmd      `help:"Interactive REPL for debugging"                          cmd:"" aliases:"debug"`
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

// SyncwebAutomaticCmd starts the syncweb-automatic daemon
type SyncwebAutomaticCmd struct {
	Devices        bool     `help:"Auto-accept devices"`
	Folders        bool     `help:"Auto-join folders"`
	Local          bool     `help:"Only auto-accept local devices"                       default:"true"`
	FoldersInclude []string `help:"Search for folders which match by label, ID, or path"`
	FoldersExclude []string `help:"Exclude folders which match by label, ID, or path"`
	FolderTypes    []string `help:"Filter folders by type"`
	DevicesInclude []string `help:"Search for devices which match by name or ID"`
	DevicesExclude []string `help:"Exclude devices which match by name or ID"`
	JoinNewFolders bool     `help:"Join non-existing folders from other devices"`
	Sort           string   `help:"Sort criteria for download prioritization"            default:"-niche,-frecency"`
}

// Help displays examples for the automatic command
func (c *SyncwebAutomaticCmd) Help() string {
	return automaticExamples
}

func (c *SyncwebAutomaticCmd) Run(g *SyncwebCmd) error {
	logger := slog.Default().With("component", "automatic")
	logger.Info("Starting syncweb-automatic",
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
				c.autoAcceptDevices(s, logger)
			}

			// 2. Auto-join folders
			if c.Folders {
				c.autoJoinFolders(s, logger, g.SyncwebHome)
			}

			<-ticker.C
		}
	})
}

func (c *SyncwebAutomaticCmd) autoAcceptDevices(s *syncweb.Syncweb, logger *slog.Logger) {
	pending := s.GetPendingDevices()
	for id := range pending {
		// Apply include/exclude filters
		if !matchesFilters(id, c.DevicesInclude, c.DevicesExclude) {
			continue
		}

		logger.Info("Auto-accepting device", "id", id)
		if err := s.AddDevice(id, "", false); err != nil {
			logger.Error("Failed to auto-accept device", "id", id, "error", err)
		}
	}
}

func (c *SyncwebAutomaticCmd) autoJoinFolders(s *syncweb.Syncweb, logger *slog.Logger, syncwebHome string) {
	cfg := s.Node.Cfg.RawCopy()
	for _, dev := range cfg.Devices {
		pending, _ := s.Node.App.Internals.PendingFolders(dev.DeviceID)
		for folderID := range pending {
			ctx := &processPendingFolderContext{
				logger:      logger,
				cfg:         cfg,
				devID:       dev.DeviceID,
				folderID:    folderID,
				syncwebHome: syncwebHome,
			}
			c.processPendingFolder(s, ctx)
		}
	}
}

// processPendingFolderContext holds context for processing a pending folder
type processPendingFolderContext struct {
	logger      *slog.Logger
	cfg         config.Configuration
	devID       protocol.DeviceID
	folderID    string
	syncwebHome string
}

func (c *SyncwebAutomaticCmd) processPendingFolder(
	s *syncweb.Syncweb,
	ctx *processPendingFolderContext,
) {
	// Apply filters
	if !matchesFilters(ctx.folderID, c.FoldersInclude, c.FoldersExclude) {
		return
	}

	// Note: Folder type filtering is not applied to pending folders
	// because the folder type is not available until the folder is joined.

	// Check if folder already exists
	exists := c.folderExists(ctx.cfg, ctx.folderID)

	if !exists && !c.JoinNewFolders {
		return
	}

	ctx.logger.Info("Auto-joining folder", "id", ctx.folderID, "from", ctx.devID)
	path := filepath.Join(ctx.syncwebHome, ctx.folderID)

	if !exists {
		if err := c.createFolder(s, ctx.logger, ctx.folderID, path); err != nil {
			return
		}
	}

	if err := s.AddFolderDevice(ctx.folderID, ctx.devID.String()); err != nil {
		ctx.logger.Error(
			"Failed to share folder with device",
			"folder",
			ctx.folderID,
			"device",
			ctx.devID,
			"error",
			err,
		)
	}
}

func (c *SyncwebAutomaticCmd) folderExists(cfg config.Configuration, folderID string) bool {
	for _, f := range cfg.Folders {
		if f.ID == folderID {
			return true
		}
	}
	return false
}

func (c *SyncwebAutomaticCmd) createFolder(s *syncweb.Syncweb, logger *slog.Logger, folderID, path string) error {
	if err := s.AddFolder(folderID, folderID, path, config.FolderTypeReceiveOnly); err != nil {
		logger.Error("Failed to create folder", "id", folderID, "error", err)
		return err
	}
	if err := s.SetIgnores(folderID, []string{}); err != nil {
		logger.Error("Failed to set ignores", "id", folderID, "error", err)
		return err
	}
	if err := s.ResumeFolder(folderID); err != nil {
		logger.Error("Failed to resume folder", "id", folderID, "error", err)
		return err
	}
	return nil
}

// matchesFilters checks if a string matches include/exclude filters
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

// SyncwebStartCmd starts the Syncweb daemon
type SyncwebStartCmd struct{}

// Help displays examples for the start command
func (c *SyncwebStartCmd) Help() string {
	return startExamples
}

func (c *SyncwebStartCmd) Run(g *SyncwebCmd) error {
	logger := slog.Default().With("component", "daemon")
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
		logger.Info("Syncweb daemon started", "pid", d.Pid)
		return nil
	}
	defer func() { _ = cntxt.Release() }()

	logger.Info("Syncweb daemon process starting")
	return nil
}

// SyncwebStopCmd stops the Syncweb daemon
type SyncwebStopCmd struct{}

// Help displays examples for the stop command
func (c *SyncwebStopCmd) Help() string {
	return stopExamples
}

func (c *SyncwebStopCmd) Run(g *SyncwebCmd) error {
	logger := slog.Default().With("component", "daemon")
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

	logger.Info("Syncweb daemon stop signal sent")
	return nil
}

// SyncwebVersionCmd shows the Syncweb version
type SyncwebVersionCmd struct{}

// Help displays examples for the version command
func (c *SyncwebVersionCmd) Help() string {
	return versionExamples
}

func (c *SyncwebVersionCmd) Run(_ *SyncwebCmd) error {
	fmt.Println(version.FullInfo())
	return nil
}
