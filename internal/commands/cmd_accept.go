package commands

import (
	"fmt"
	"strings"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// SyncwebAcceptCmd accepts devices and optionally adds them to folders
type SyncwebAcceptCmd struct {
	DeviceIDs  []string `arg:"" required:"" name:"device-ids" help:"Syncthing device IDs (space or comma-separated)"`
	FolderIDs  []string `short:"f" help:"Add devices to folders"`
	Introducer bool     `help:"Configure devices as introducers"`
}

func (c *SyncwebAcceptCmd) Run(g *SyncwebCmd) error {
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
		for _, devID := range deviceIDs {
			// Validate device ID format
			if _, err := utils.ExtractDeviceID(devID); err != nil {
				fmt.Printf("Invalid Device ID %s: %v\n", devID, err)
				continue
			}

			if err := s.AddDevice(devID, "", c.Introducer); err != nil {
				fmt.Printf("Failed to add device %s: %v\n", devID, err)
				continue
			}
			deviceCount++
		}

		// Add devices to folders if specified
		if len(c.FolderIDs) > 0 {
			for _, fldID := range c.FolderIDs {
				if err := s.AddFolderDevices(fldID, deviceIDs); err != nil {
					fmt.Printf("Failed to add devices to folder %s: %v\n", fldID, err)
					continue
				}

				// Pause and resume devices to unstuck connections
				for _, devID := range deviceIDs {
					_ = s.PauseDevice(devID)
				}
				for _, devID := range deviceIDs {
					_ = s.ResumeDevice(devID)
				}
			}
		}

		fmt.Printf("Added %d %s\n", deviceCount, pluralize(deviceCount, "device", "devices"))
		return nil
	})
}
