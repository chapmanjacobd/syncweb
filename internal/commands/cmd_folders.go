package commands

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"syscall"

	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// Folders command examples
const foldersExamples = `
Examples:
  # Show all folders
  syncweb folders

  # Show only pending folders and join them
  syncweb folders --pending --join

  # Search for folders by label or ID
  syncweb folders -s music,audio

  # Exclude folders by pattern
  syncweb folders -E backup,temp

  # Filter by folder type
  syncweb folders -t sendonly,receiveonly

  # Pause matching folders
  syncweb folders -s test --pause

  # Delete metadata and files for test folders
  syncweb folders -s test --delete --delete-files
`

// SyncwebFoldersCmd lists Syncthing folders
type SyncwebFoldersCmd struct {
	Joined      bool     `help:"Only show joined folders"`
	Pending     bool     `help:"Only show pending folders"`
	Discovered  bool     `help:"Only show discovered folders"`
	Join        bool     `help:"Join pending folders"`
	Missing     bool     `help:"Only show orphaned folders"`
	LocalOnly   bool     `help:"Only include local devices"`
	Include     []string `help:"Search for folders by label, ID, or path"        short:"s"`
	Exclude     []string `help:"Exclude folders by label, ID, or path"           short:"E"`
	FolderTypes []string `help:"Filter by folder type"                           short:"t"`
	Introduce   bool     `help:"Introduce devices to all local folders"`
	Delete      bool     `help:"Delete Syncweb metadata for filtered folders"`
	DeleteFiles bool     `help:"Delete actual folders/files in filtered folders"`
	Pause       bool     `help:"Pause matching folders"`
	Resume      bool     `help:"Resume matching folders"`
	Print       bool     `help:"Print only folder IDs"`
}

// Help displays examples for the folders command
func (c *SyncwebFoldersCmd) Help() string {
	return foldersExamples
}

func (c *SyncwebFoldersCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		// If no filter specified, show all
		if !c.Joined && !c.Pending && !c.Discovered {
			c.Joined = true
			c.Pending = true
			c.Discovered = true
		}

		cfg := s.Node.Cfg.RawCopy()
		localDeviceID := s.Node.MyID().String()

		// Collect all folders
		folders := c.collectFolders(s, cfg)

		// Apply filters
		filtered := c.filterFolders(folders)

		// Sort by ID
		sort.Slice(filtered, func(i, j int) bool {
			return filtered[i].ID < filtered[j].ID
		})

		// Output
		if g.JSON {
			return c.outputJSON(filtered)
		}

		if c.Print {
			return c.outputPrint(filtered, localDeviceID)
		}

		return c.outputTable(s, filtered, localDeviceID, cfg)
	})
}

type folderEntry struct {
	ID             string   `json:"id"`
	Label          string   `json:"label"`
	Path           string   `json:"path"`
	LocalFiles     int      `json:"local_files"`
	LocalBytes     int64    `json:"local_bytes"`
	NeededFiles    int      `json:"needed_files"`
	NeededBytes    int64    `json:"needed_bytes"`
	GlobalFiles    int      `json:"global_files"`
	GlobalBytes    int64    `json:"global_bytes"`
	FreeSpace      string   `json:"free_space"`
	SyncStatus     string   `json:"sync_status"`
	SyncPct        float64  `json:"sync_pct"`
	Peers          int      `json:"peers"`
	PendingPeers   int      `json:"pending_peers"`
	Errors         string   `json:"errors"`
	Type           string   `json:"type"`
	Paused         bool     `json:"paused"`
	Devices        []string `json:"devices"`
	PendingDevices []string `json:"pending_devices"`
	Discovered     bool     `json:"discovered"`
	State          string   `json:"state"`
	Completed      int64    `json:"completed"`
}

func (c *SyncwebFoldersCmd) collectFolders(s *syncweb.Syncweb, cfg config.Configuration) []folderEntry {
	var folders []folderEntry
	seenIDs := make(map[string]bool)

	// Get joined folders
	if c.Joined {
		for _, f := range cfg.Folders {
			if seenIDs[f.ID] {
				continue
			}
			seenIDs[f.ID] = true
			folders = append(folders, c.buildJoinedFolderEntry(s, f))
		}
	}

	// Get pending/discovered folders
	if c.Pending || c.Discovered {
		pending := s.GetPendingFolders()
		for folderID, info := range pending {
			offeredBy, _ := info["offeredBy"].(map[string]map[string]any)
			if len(offeredBy) == 0 {
				continue
			}

			var pendingDevices []string
			for devID := range offeredBy {
				pendingDevices = append(pendingDevices, devID)
			}

			// Check if already joined
			alreadyJoined := false
			for i, f := range folders {
				if f.ID == folderID {
					folders[i].PendingDevices = pendingDevices
					alreadyJoined = true
					break
				}
			}

			if !alreadyJoined {
				if seenIDs[folderID] {
					continue
				}
				seenIDs[folderID] = true

				// Get label from first offer
				label := folderID
				folders = append(folders, folderEntry{
					ID:             folderID,
					Label:          label,
					Path:           "(not joined)",
					Discovered:     true,
					PendingDevices: pendingDevices,
				})
			}
		}
	}

	return folders
}

func (c *SyncwebFoldersCmd) buildJoinedFolderEntry(s *syncweb.Syncweb, f config.FolderConfiguration) folderEntry {
	devices := make([]string, 0, len(f.Devices))
	for _, d := range f.Devices {
		devices = append(devices, d.DeviceID.String())
	}

	entry := folderEntry{
		ID:      f.ID,
		Label:   f.Label,
		Path:    f.Path,
		Type:    f.Type.String(),
		Paused:  f.Paused,
		Devices: devices,
	}

	// Get free space
	if f.Path != "" {
		if info, statErr := os.Stat(f.Path); statErr == nil && info.IsDir() {
			var stat syscall.Statfs_t
			if statfsErr := syscall.Statfs(f.Path, &stat); statfsErr == nil {
				entry.FreeSpace = utils.FormatSize(
					int64(stat.Bavail) * stat.Bsize,
				)
			}
		}
	}

	// Get live sync progress from Internals
	globalSize, _ := s.Node.App.Internals.GlobalSize(f.ID)
	localSize, _ := s.Node.App.Internals.LocalSize(f.ID)
	needSize, _ := s.Node.App.Internals.NeedSize(f.ID, protocol.LocalDeviceID)

	entry.LocalFiles = int(localSize.Files) //nolint:unconvert // Files is int64, struct field is int
	entry.LocalBytes = localSize.Bytes
	entry.NeededFiles = int(needSize.Files) //nolint:unconvert // Files is int64, struct field is int
	entry.NeededBytes = needSize.Bytes
	entry.GlobalFiles = int(globalSize.Files) //nolint:unconvert // Files is int64, struct field is int
	entry.GlobalBytes = globalSize.Bytes

	// Calculate sync percentage from live progress
	if globalSize.Bytes > 0 {
		entry.SyncPct = float64(localSize.Bytes) / float64(globalSize.Bytes) * 100
	}

	// Get folder state (idle, scanning, syncing)
	state, _, _ := s.Node.App.Internals.FolderState(f.ID)
	entry.State = state

	// Get bytes completed for real-time progress
	completed := s.Node.App.Internals.FolderProgressBytesCompleted(f.ID)
	entry.Completed = completed

	return entry
}

func (c *SyncwebFoldersCmd) filterFolders(folders []folderEntry) []folderEntry {
	filtered := make([]folderEntry, 0, len(folders))

	for _, f := range folders {
		if !c.shouldIncludeFolder(f) {
			continue
		}
		filtered = append(filtered, f)
	}

	return filtered
}

func (c *SyncwebFoldersCmd) shouldIncludeFolder(f folderEntry) bool {
	// Missing filter
	if c.Missing && !c.isFolderMissing(f) {
		return false
	}

	// Type filter
	if len(c.FolderTypes) > 0 && !c.matchesFolderType(f) {
		return false
	}

	// Include filter
	if len(c.Include) > 0 && !c.matchesIncludeFilter(f) {
		return false
	}

	// Exclude filter
	if len(c.Exclude) > 0 && c.matchesExcludeFilter(f) {
		return false
	}

	return true
}

func (c *SyncwebFoldersCmd) isFolderMissing(f folderEntry) bool {
	if f.Path != "" && f.Path != "(not joined)" {
		if _, err := os.Stat(f.Path); err == nil {
			return false // Path exists, not missing
		}
	}
	return true
}

func (c *SyncwebFoldersCmd) matchesFolderType(f folderEntry) bool {
	for _, t := range c.FolderTypes {
		if strings.EqualFold(f.Type, t) {
			return true
		}
	}
	return false
}

func (c *SyncwebFoldersCmd) matchesIncludeFilter(f folderEntry) bool {
	for _, s := range c.Include {
		if strings.Contains(f.Label, s) || strings.Contains(f.ID, s) || strings.Contains(f.Path, s) {
			return true
		}
	}
	return false
}

func (c *SyncwebFoldersCmd) matchesExcludeFilter(f folderEntry) bool {
	for _, s := range c.Exclude {
		if strings.Contains(f.Label, s) || strings.Contains(f.ID, s) || strings.Contains(f.Path, s) {
			return true
		}
	}
	return false
}

func (c *SyncwebFoldersCmd) outputJSON(filtered []folderEntry) error {
	data, err := json.MarshalIndent(filtered, "", "  ")
	if err != nil {
		return err
	}
	fmt.Println(string(data))
	return nil
}

func (c *SyncwebFoldersCmd) outputPrint(filtered []folderEntry, localDeviceID string) error {
	for _, f := range filtered {
		if f.Discovered && len(f.PendingDevices) > 0 {
			fmt.Printf("sync://%s#%s\n", f.ID, f.PendingDevices[0])
		} else {
			fmt.Printf("sync://%s#%s\n", f.ID, localDeviceID)
		}
	}
	return nil
}

func (c *SyncwebFoldersCmd) outputTable(
	s *syncweb.Syncweb,
	filtered []folderEntry,
	_ string,
	cfg config.Configuration,
) error {
	// Print header
	fmt.Printf(
		"%-15s  %-8s  %-28s  %-18s  %-18s  %-20s  %-8s  %-15s  %-6s  %-10s  %s\n",
		"Folder ID",
		"Label",
		"Path",
		"Local",
		"Needed",
		"Global",
		"Free",
		"Sync Status",
		"Peers",
		"State",
		"Errors",
	)
	fmt.Println(strings.Repeat("-", 170))

	// Print rows
	for _, f := range filtered {
		c.printFolderRow(f)
	}
	fmt.Println()

	// Actions
	c.executeActions(filtered, cfg, s)

	return nil
}

func (c *SyncwebFoldersCmd) printFolderRow(f folderEntry) {
	local := "-"
	if f.LocalFiles > 0 || f.LocalBytes > 0 {
		local = fmt.Sprintf("%d files (%s)", f.LocalFiles, utils.FormatSize(f.LocalBytes))
	}

	needed := "-"
	if f.NeededFiles > 0 || f.NeededBytes > 0 {
		needed = fmt.Sprintf("%d files (%s)", f.NeededFiles, utils.FormatSize(f.NeededBytes))
	}

	global := "-"
	if f.GlobalFiles > 0 || f.GlobalBytes > 0 {
		global = fmt.Sprintf("%d files (%s)", f.GlobalFiles, utils.FormatSize(f.GlobalBytes))
	}

	syncStatus := fmt.Sprintf("%.0f%%", f.SyncPct)
	if f.Paused {
		syncStatus = "⏸️ " + syncStatus
	}

	// Show progress bar for syncing folders
	if f.State == "syncing" && f.GlobalBytes > 0 {
		progress := float64(f.Completed) / float64(f.GlobalBytes) * 100
		syncStatus = fmt.Sprintf("%.0f%% (%s)", progress, utils.FormatSize(f.Completed))
	}

	peers := strconv.Itoa(f.Peers)
	if f.PendingPeers > 0 {
		peers += fmt.Sprintf(" (%d)", f.PendingPeers)
	}

	path := f.Path
	if f.Discovered && len(f.PendingDevices) > 0 {
		path = f.PendingDevices[0]
	}

	state := f.State
	if state == "" {
		state = "idle"
	}
	if f.Paused {
		state = "paused"
	}

	errors := "-"
	if f.Errors != "" {
		errors = f.Errors
	}

	fmt.Printf("%-15s  %-8s  %-28s  %-18s  %-18s  %-20s  %-8s  %-15s  %-6s  %-10s  %s\n",
		f.ID, f.Label, path, local, needed, global, f.FreeSpace, syncStatus, peers, state, errors)
}

func (c *SyncwebFoldersCmd) executeActions(filtered []folderEntry, cfg config.Configuration, s *syncweb.Syncweb) {
	if c.Join {
		c.actionJoin(filtered, cfg, s)
	}

	if c.Pause {
		c.actionPause(s, filtered)
	}

	if c.Resume {
		c.actionResume(s, filtered)
	}

	if c.Delete {
		c.actionDelete(s, filtered)
	}

	if c.DeleteFiles {
		c.actionDeleteFiles(filtered)
	}
}

func (c *SyncwebFoldersCmd) actionJoin(filtered []folderEntry, cfg config.Configuration, s *syncweb.Syncweb) {
	var toJoin []folderEntry
	for _, f := range filtered {
		if len(f.PendingDevices) > 0 {
			toJoin = append(toJoin, f)
		}
	}

	if len(toJoin) == 0 {
		fmt.Println("No pending folders to join")
		return
	}

	for _, f := range toJoin {
		c.joinFolder(f, cfg, s)
	}
}

func (c *SyncwebFoldersCmd) joinFolder(f folderEntry, cfg config.Configuration, s *syncweb.Syncweb) {
	folderID := f.ID
	deviceIDs := f.PendingDevices

	// Check if folder exists
	exists := c.folderExists(cfg, folderID)

	if exists {
		c.addDevicesToFolder(folderID, deviceIDs, s)
	} else {
		c.createAndJoinFolder(folderID, deviceIDs, s)
	}
}

func (c *SyncwebFoldersCmd) folderExists(cfg config.Configuration, folderID string) bool {
	for _, ef := range cfg.Folders {
		if ef.ID == folderID {
			return true
		}
	}
	return false
}

func (c *SyncwebFoldersCmd) addDevicesToFolder(folderID string, deviceIDs []string, s *syncweb.Syncweb) {
	if addErr := s.AddFolderDevices(folderID, deviceIDs); addErr != nil {
		fmt.Printf("Error adding devices to %s: %v\n", folderID, addErr)
		return
	}

	// Pause/resume to unstuck
	for _, devID := range deviceIDs {
		if pauseErr := s.PauseDevice(devID); pauseErr != nil {
			fmt.Fprintf(os.Stderr, "Warning: Failed to pause device %s: %v\n", devID, pauseErr)
		}
	}
	for _, devID := range deviceIDs {
		if resumeErr := s.ResumeDevice(devID); resumeErr != nil {
			fmt.Fprintf(os.Stderr, "Warning: Failed to resume device %s: %v\n", devID, resumeErr)
		}
	}
	fmt.Printf("Joined folder %s with %d devices\n", folderID, len(deviceIDs))
}

func (c *SyncwebFoldersCmd) createAndJoinFolder(folderID string, deviceIDs []string, s *syncweb.Syncweb) {
	dest := filepath.Join(os.Getenv("HOME"), "Syncweb", folderID)
	if mkdirErr := os.MkdirAll(dest, 0o755); mkdirErr != nil {
		fmt.Printf("Error creating directory for %s: %v\n", folderID, mkdirErr)
		return
	}

	if addErr := s.AddFolder(
		folderID,
		folderID,
		dest,
		config.FolderTypeReceiveOnly,
	); addErr != nil {
		fmt.Printf("Error creating folder %s: %v\n", folderID, addErr)
		return
	}

	if err := s.SetIgnores(folderID, []string{}); err != nil {
		fmt.Printf("Error setting ignores for %s: %v\n", folderID, err)
		return
	}

	if err := s.ResumeFolder(folderID); err != nil {
		fmt.Printf("Error resuming folder %s: %v\n", folderID, err)
		return
	}

	if err := s.AddFolderDevices(folderID, deviceIDs); err != nil {
		fmt.Printf("Error sharing folder with devices: %v\n", err)
		return
	}

	fmt.Printf("Created and joined folder %s\n", folderID)
}

func (c *SyncwebFoldersCmd) actionPause(s *syncweb.Syncweb, filtered []folderEntry) {
	count := 0
	for _, f := range filtered {
		if !f.Paused {
			if err := s.PauseFolder(f.ID); err == nil {
				count++
			}
		}
	}
	fmt.Printf("Paused %d %s\n", count, utils.Pluralize(count, "folder", "folders"))
}

func (c *SyncwebFoldersCmd) actionResume(s *syncweb.Syncweb, filtered []folderEntry) {
	count := 0
	for _, f := range filtered {
		if f.Paused {
			if err := s.ResumeFolder(f.ID); err == nil {
				count++
			}
		}
	}
	fmt.Printf("Resumed %d %s\n", count, utils.Pluralize(count, "folder", "folders"))
}

func (c *SyncwebFoldersCmd) actionDelete(s *syncweb.Syncweb, filtered []folderEntry) {
	count := 0
	for _, f := range filtered {
		if len(f.Devices) > 0 {
			if err := s.DeleteFolder(f.ID); err == nil {
				count++
			}
		}
	}
	fmt.Printf("Deleted %d %s\n", count, utils.Pluralize(count, "folder", "folders"))
}

func (c *SyncwebFoldersCmd) actionDeleteFiles(filtered []folderEntry) {
	count := 0
	for _, f := range filtered {
		if len(f.Devices) > 0 && f.Path != "" && f.Path != "(not joined)" {
			if _, statErr := os.Stat(f.Path); statErr == nil {
				if removeErr := os.RemoveAll(f.Path); removeErr == nil {
					count++
				}
			}
		}
	}
	fmt.Printf("Deleted files from %d %s\n", count, utils.Pluralize(count, "folder", "folders"))
}
