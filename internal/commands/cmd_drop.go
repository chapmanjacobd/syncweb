package commands

import (
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"strings"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// Drop command examples
const dropExamples = `
Examples:
  # Drop a device
  syncweb drop NXL7XBL-VPNDOSR-QXU7WI7-NEUI65A-TWN7YGT-WS2U457-NTZNGB4-J6IYDQH

  # Drop multiple devices (comma-separated)
  syncweb drop DEV1,DEV2,DEV3

  # Drop device from specific folders
  syncweb drop --folders=audio,video DEVICE-ID

  # Drop multiple devices (space-separated)
  syncweb drop DEV1 DEV2 DEV3
`

// SyncwebDropCmd removes devices from syncweb
type SyncwebDropCmd struct {
	DeviceIDs []string `help:"Syncthing device IDs (space or comma-separated)" required:"true" name:"device-ids" arg:""`
	FolderIDs []string `help:"Remove devices from folders"                                                              short:"f"`
}

// Help displays examples for the drop command
func (c *SyncwebDropCmd) Help() string {
	return dropExamples
}

type DropResult struct {
	DeviceCount int      `json:"device_count"`
	Devices     []string `json:"devices"`
	Errors      []string `json:"errors,omitempty"`
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

		result := DropResult{
			Devices: []string{},
			Errors:  []string{},
		}

		// If folder IDs specified, remove devices from folders
		if len(c.FolderIDs) > 0 {
			for _, fldID := range c.FolderIDs {
				if err := s.RemoveFolderDevices(fldID, deviceIDs); err != nil {
					errMsg := fmt.Sprintf("Failed to remove devices from folder %s: %v", fldID, err)
					result.Errors = append(result.Errors, errMsg)
					if !g.JSON {
						fmt.Println(errMsg)
					}
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

			if g.JSON {
				jsonData, _ := json.MarshalIndent(result, "", "  ")
				fmt.Println(string(jsonData))
			} else {
				fmt.Printf("Removed from %d folder(s)\n", len(c.FolderIDs))
			}
			return nil
		}

		// Remove devices entirely
		for _, devID := range deviceIDs {
			if err := s.DeleteDevice(devID); err != nil {
				errMsg := fmt.Sprintf("Failed to remove device %s: %v", devID, err)
				result.Errors = append(result.Errors, errMsg)
				if !g.JSON {
					fmt.Println(errMsg)
				}
				continue
			}
			result.Devices = append(result.Devices, devID)
			result.DeviceCount++
		}

		if g.JSON {
			jsonData, _ := json.MarshalIndent(result, "", "  ")
			fmt.Println(string(jsonData))
		} else {
			fmt.Printf("Removed %d %s\n", result.DeviceCount, utils.Pluralize(result.DeviceCount, "device", "devices"))
		}

		// Exit with error if all device IDs were invalid
		if len(deviceIDs) > 0 && result.DeviceCount == 0 {
			return errors.New("no valid devices were removed")
		}

		return nil
	})
}
