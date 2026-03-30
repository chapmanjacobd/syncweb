package commands

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"syscall"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"
)

// SyncwebFoldersCmd lists Syncthing folders
type SyncwebFoldersCmd struct {
	Joined      bool     `help:"Only show joined folders"`
	Pending     bool     `help:"Only show pending folders"`
	Discovered  bool     `help:"Only show discovered folders"`
	Join        bool     `help:"Join pending folders"`
	Missing     bool     `help:"Only show orphaned folders"`
	LocalOnly   bool     `help:"Only include local devices"`
	Include     []string `short:"s" help:"Search for folders by label, ID, or path"`
	Exclude     []string `short:"E" help:"Exclude folders by label, ID, or path"`
	FolderTypes []string `short:"t" help:"Filter by folder type"`
	Introduce   bool     `help:"Introduce devices to all local folders"`
	Delete      bool     `help:"Delete Syncweb metadata for filtered folders"`
	DeleteFiles bool     `help:"Delete actual folders/files in filtered folders"`
	Pause       bool     `help:"Pause matching folders"`
	Resume      bool     `help:"Resume matching folders"`
	Print       bool     `help:"Print only folder IDs"`
}

func (c *SyncwebFoldersCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		// If no filter specified, show all
		if !c.Joined && !c.Pending && !c.Discovered {
			c.Joined = true
			c.Pending = true
			c.Discovered = true
		}

		type folderEntry struct {
			ID             string
			Label          string
			Path           string
			LocalFiles     int
			LocalBytes     int64
			NeededFiles    int
			NeededBytes    int64
			GlobalFiles    int
			GlobalBytes    int64
			FreeSpace      string
			SyncStatus     string
			SyncPct        float64
			Peers          int
			PendingPeers   int
			Errors         string
			Type           string
			Paused         bool
			Devices        []string
			PendingDevices []string
			Discovered     bool
			State          string
			Completed      int64
		}

		var folders []folderEntry
		seenIDs := make(map[string]bool)

		cfg := s.Node.Cfg.RawCopy()
		localDeviceID := s.Node.MyID().String()

		// Get folder statistics for live sync progress
		// Note: FolderStatistics is not available on Internals, so we use GlobalSize, LocalSize, NeedSize

		// Get joined folders
		if c.Joined {
			for _, f := range cfg.Folders {
				if seenIDs[f.ID] {
					continue
				}
				seenIDs[f.ID] = true

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
					if info, err := os.Stat(f.Path); err == nil && info.IsDir() {
						var stat syscall.Statfs_t
						if err := syscall.Statfs(f.Path, &stat); err == nil {
							entry.FreeSpace = utils.FormatSize(int64(stat.Bavail) * int64(stat.Bsize))
						}
					}
				}

				// Get live sync progress from Internals
				globalSize, _ := s.Node.App.Internals.GlobalSize(f.ID)
				localSize, _ := s.Node.App.Internals.LocalSize(f.ID)
				needSize, _ := s.Node.App.Internals.NeedSize(f.ID, protocol.LocalDeviceID)

				entry.LocalFiles = int(localSize.Files)
				entry.LocalBytes = localSize.Bytes
				entry.NeededFiles = int(needSize.Files)
				entry.NeededBytes = needSize.Bytes
				entry.GlobalFiles = int(globalSize.Files)
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

				folders = append(folders, entry)
			}
		}

		// Get pending folders
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

		// Apply filters
		var filtered []folderEntry
		for _, f := range folders {
			// Missing filter
			if c.Missing {
				// Check if folder path is missing
				if f.Path != "" && f.Path != "(not joined)" {
					if _, err := os.Stat(f.Path); err == nil {
						continue // Path exists, not missing
					}
				}
			}

			// Type filter
			if len(c.FolderTypes) > 0 {
				matched := false
				for _, t := range c.FolderTypes {
					if strings.EqualFold(f.Type, t) {
						matched = true
						break
					}
				}
				if !matched {
					continue
				}
			}

			// Include filter
			if len(c.Include) > 0 {
				matched := false
				for _, s := range c.Include {
					if strings.Contains(f.Label, s) || strings.Contains(f.ID, s) || strings.Contains(f.Path, s) {
						matched = true
						break
					}
				}
				if !matched {
					continue
				}
			}

			// Exclude filter
			if len(c.Exclude) > 0 {
				excluded := false
				for _, s := range c.Exclude {
					if strings.Contains(f.Label, s) || strings.Contains(f.ID, s) || strings.Contains(f.Path, s) {
						excluded = true
						break
					}
				}
				if excluded {
					continue
				}
			}

			filtered = append(filtered, f)
		}

		// Sort by ID
		sort.Slice(filtered, func(i, j int) bool {
			return filtered[i].ID < filtered[j].ID
		})

		if g.JSON {
			data, err := json.MarshalIndent(filtered, "", "  ")
			if err != nil {
				return err
			}
			fmt.Println(string(data))
			return nil
		}

		if c.Print {
			for _, f := range filtered {
				if f.Discovered && len(f.PendingDevices) > 0 {
					fmt.Printf("sync://%s#%s\n", f.ID, f.PendingDevices[0])
				} else {
					fmt.Printf("sync://%s#%s\n", f.ID, localDeviceID)
				}
			}
			return nil
		}

		// Print table
		fmt.Printf("%-15s  %-8s  %-28s  %-18s  %-18s  %-20s  %-8s  %-15s  %-6s  %-10s  %s\n",
			"Folder ID", "Label", "Path", "Local", "Needed", "Global", "Free", "Sync Status", "Peers", "State", "Errors")
		fmt.Println(strings.Repeat("-", 170))

		for _, f := range filtered {
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

			peers := fmt.Sprintf("%d", f.Peers)
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
		fmt.Println()

		// Actions
		if c.Join {
			var toJoin []folderEntry
			for _, f := range filtered {
				if len(f.PendingDevices) > 0 {
					toJoin = append(toJoin, f)
				}
			}

			if len(toJoin) == 0 {
				fmt.Println("No pending folders to join")
			} else {
				for _, f := range toJoin {
					folderID := f.ID
					deviceIDs := f.PendingDevices

					// Check if folder exists
					exists := false
					for _, ef := range cfg.Folders {
						if ef.ID == folderID {
							exists = true
							break
						}
					}

					if exists {
						// Just add devices
						if err := s.AddFolderDevices(folderID, deviceIDs); err != nil {
							fmt.Printf("Error adding devices to %s: %v\n", folderID, err)
						} else {
							// Pause/resume to unstuck
							for _, devID := range deviceIDs {
								_ = s.PauseDevice(devID)
							}
							for _, devID := range deviceIDs {
								_ = s.ResumeDevice(devID)
							}
							fmt.Printf("Joined folder %s with %d devices\n", folderID, len(deviceIDs))
						}
					} else {
						// Create folder
						dest := filepath.Join(os.Getenv("HOME"), "Syncweb", folderID)
						if err := os.MkdirAll(dest, 0o755); err != nil {
							fmt.Printf("Error creating directory for %s: %v\n", folderID, err)
							continue
						}

						if err := s.AddFolder(folderID, folderID, dest, config.FolderTypeReceiveOnly); err != nil {
							fmt.Printf("Error creating folder %s: %v\n", folderID, err)
							continue
						}

						if err := s.SetIgnores(folderID, []string{}); err != nil {
							fmt.Printf("Error setting ignores for %s: %v\n", folderID, err)
							continue
						}

						if err := s.ResumeFolder(folderID); err != nil {
							fmt.Printf("Error resuming folder %s: %v\n", folderID, err)
							continue
						}

						if err := s.AddFolderDevices(folderID, deviceIDs); err != nil {
							fmt.Printf("Error sharing folder with devices: %v\n", err)
							continue
						}

						fmt.Printf("Created and joined folder %s\n", folderID)
					}
				}
			}
		}

		if c.Pause {
			count := 0
			for _, f := range filtered {
				if !f.Paused {
					if err := s.PauseFolder(f.ID); err == nil {
						count++
					}
				}
			}
			fmt.Printf("Paused %d %s\n", count, pluralize(count, "folder", "folders"))
		}

		if c.Resume {
			count := 0
			for _, f := range filtered {
				if f.Paused {
					if err := s.ResumeFolder(f.ID); err == nil {
						count++
					}
				}
			}
			fmt.Printf("Resumed %d %s\n", count, pluralize(count, "folder", "folders"))
		}

		if c.Delete {
			count := 0
			for _, f := range filtered {
				for _, devID := range f.PendingDevices {
					// Delete pending folder
					_ = devID // Would need API call to delete pending
				}
				if len(f.Devices) > 0 {
					if err := s.DeleteFolder(f.ID); err == nil {
						count++
					}
				}
			}
			fmt.Printf("Deleted %d %s\n", count, pluralize(count, "folder", "folders"))
		}

		if c.DeleteFiles {
			count := 0
			for _, f := range filtered {
				if len(f.Devices) > 0 && f.Path != "" && f.Path != "(not joined)" {
					if _, err := os.Stat(f.Path); err == nil {
						if err := os.RemoveAll(f.Path); err == nil {
							count++
						}
					}
				}
			}
			fmt.Printf("Deleted files from %d %s\n", count, pluralize(count, "folder", "folders"))
		}

		return nil
	})
}
