package commands

import (
	"fmt"
	"log/slog"
	"strings"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// SyncwebDropCmd removes devices from syncweb
type SyncwebDropCmd struct {
	DeviceIDs []string `arg:""                             help:"Syncthing device IDs (space or comma-separated)" name:"device-ids" required:""`
	FolderIDs []string `help:"Remove devices from folders" short:"f"`
}

func (c *SyncwebDropCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		// Parse device IDs (support comma-separated)
		var deviceIDs []string
		for _, id := range c.DeviceIDs {
			parts := strings.SplitSeq(id, ",")
			for p := range parts {
				p = strings.TrimSpace(p)
				if p != "" {
					deviceIDs = append(deviceIDs, p)
				}
			}
		}

		deviceCount := 0

		// If folder IDs specified, remove devices from folders
		if len(c.FolderIDs) > 0 {
			for _, fldID := range c.FolderIDs {
				if err := s.RemoveFolderDevices(fldID, deviceIDs); err != nil {
					fmt.Printf("Failed to remove devices from folder %s: %v\n", fldID, err)
					continue
				}

				// Pause and resume devices to immediately drop connections
				for _, devID := range deviceIDs {
					if err := s.PauseDevice(devID); err != nil {
						slog.Warn("Failed to pause device", "device", devID, "error", err)
					}
				}
				for _, devID := range deviceIDs {
					if err := s.ResumeDevice(devID); err != nil {
						slog.Warn("Failed to resume device", "device", devID, "error", err)
					}
				}
			}
			return nil
		}

		// Remove devices entirely
		for _, devID := range deviceIDs {
			if err := s.DeleteDevice(devID); err != nil {
				fmt.Printf("Failed to remove device %s: %v\n", devID, err)
				continue
			}
			deviceCount++
		}

		fmt.Printf("Removed %d %s\n", deviceCount, utils.Pluralize(deviceCount, "device", "devices"))
		return nil
	})
}
