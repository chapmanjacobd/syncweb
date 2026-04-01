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

// Accept command examples
const acceptExamples = `
Examples:
  # Accept a device
  syncweb accept NXL7XBL-VPNDOSR-QXU7WI7-NEUI65A-TWN7YGT-WS2U457-NTZNGB4-J6IYDQH

  # Accept multiple devices (comma-separated)
  syncweb accept DEV1,DEV2,DEV3

  # Accept device and add to specific folders
  syncweb accept --folders=audio,video DEVICE-ID

  # Accept device as introducer
  syncweb accept --introducer DEVICE-ID

  # Accept multiple devices (space-separated)
  syncweb accept DEV1 DEV2 DEV3
`

// SyncwebAcceptCmd accepts devices and optionally adds them to folders
type SyncwebAcceptCmd struct {
	DeviceIDs  []string `help:"Syncthing device IDs (space or comma-separated)" required:"" name:"device-ids" arg:""`
	FolderIDs  []string `help:"Add devices to folders"                                                               short:"f"`
	Introducer bool     `help:"Configure devices as introducers"`
}

// Help displays examples for the accept command
func (c *SyncwebAcceptCmd) Help() string {
	return acceptExamples
}

type AcceptResult struct {
	DeviceCount int      `json:"device_count"`
	Devices     []string `json:"devices"`
	Errors      []string `json:"errors,omitempty"`
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

		result := AcceptResult{
			Devices: []string{},
			Errors:  []string{},
		}

		for _, devID := range deviceIDs {
			// Validate device ID format
			if _, err := utils.ExtractDeviceID(devID); err != nil {
				errMsg := fmt.Sprintf("Invalid Device ID %s: %v", devID, err)
				result.Errors = append(result.Errors, errMsg)
				if !g.JSON {
					fmt.Println(errMsg)
				}
				continue
			}

			if err := s.AddDevice(devID, "", c.Introducer); err != nil {
				errMsg := fmt.Sprintf("Failed to add device %s: %v", devID, err)
				result.Errors = append(result.Errors, errMsg)
				if !g.JSON {
					fmt.Println(errMsg)
				}
				continue
			}
			result.Devices = append(result.Devices, devID)
			result.DeviceCount++
		}

		// Add devices to folders if specified
		if len(c.FolderIDs) > 0 {
			for _, fldID := range c.FolderIDs {
				if err := s.AddFolderDevices(fldID, result.Devices); err != nil {
					errMsg := fmt.Sprintf("Failed to add devices to folder %s: %v", fldID, err)
					result.Errors = append(result.Errors, errMsg)
					if !g.JSON {
						fmt.Println(errMsg)
					}
					continue
				}

				// Pause and resume devices to unstuck connections
				for _, devID := range result.Devices {
					if err := s.PauseDevice(devID); err != nil {
						slog.Warn("Failed to pause device", "device", devID, "error", err)
					}
				}
				for _, devID := range result.Devices {
					if err := s.ResumeDevice(devID); err != nil {
						slog.Warn("Failed to resume device", "device", devID, "error", err)
					}
				}
			}
		}

		if g.JSON {
			jsonData, _ := json.MarshalIndent(result, "", "  ")
			fmt.Println(string(jsonData))
		} else {
			fmt.Printf("Added %d %s\n", result.DeviceCount, utils.Pluralize(result.DeviceCount, "device", "devices"))
		}

		// Exit with error if all device IDs were invalid
		if len(deviceIDs) > 0 && result.DeviceCount == 0 {
			return errors.New("no valid devices were added")
		}

		return nil
	})
}
