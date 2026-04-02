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
		deviceIDs := parseDropDeviceIDs(c.DeviceIDs)

		// If folder IDs specified, remove devices from folders
		if len(c.FolderIDs) > 0 {
			return c.removeDevicesFromFolders(s, c.FolderIDs, deviceIDs, g.JSON)
		}

		// Remove devices entirely
		result := c.removeDevices(s, deviceIDs, g.JSON)

		// Exit with error if all device IDs were invalid
		if len(deviceIDs) > 0 && result.DeviceCount == 0 {
			return errors.New("no valid devices were removed")
		}

		return nil
	})
}

// removeDevicesFromFolders removes devices from specified folders
func (c *SyncwebDropCmd) removeDevicesFromFolders(
	s *syncweb.Syncweb,
	folderIDs, deviceIDs []string,
	isJSON bool,
) error {
	result := DropResult{
		Devices: []string{},
		Errors:  []string{},
	}

	for _, fldID := range folderIDs {
		if err := s.RemoveFolderDevices(fldID, deviceIDs); err != nil {
			errMsg := fmt.Sprintf("Failed to remove devices from folder %s: %v", fldID, err)
			result.Errors = append(result.Errors, errMsg)
			if !isJSON {
				fmt.Println(errMsg)
			}
			continue
		}

		// Pause and resume devices to immediately drop connections
		c.pauseAndResumeDevices(s, deviceIDs)
	}

	if isJSON {
		jsonData, err := json.MarshalIndent(result, "", "  ")
		if err != nil {
			fmt.Printf("Error marshaling result: %v\n", err)
			return nil
		}
		fmt.Println(string(jsonData))
	} else {
		fmt.Printf("Removed from %d folder(s)\n", len(c.FolderIDs))
	}

	return nil
}

// removeDevices removes devices entirely from syncweb
func (c *SyncwebDropCmd) removeDevices(s *syncweb.Syncweb, deviceIDs []string, isJSON bool) DropResult {
	result := DropResult{
		Devices: []string{},
		Errors:  []string{},
	}

	for _, devID := range deviceIDs {
		if err := s.DeleteDevice(devID); err != nil {
			errMsg := fmt.Sprintf("Failed to remove device %s: %v", devID, err)
			result.Errors = append(result.Errors, errMsg)
			if !isJSON {
				fmt.Println(errMsg)
			}
			continue
		}
		result.Devices = append(result.Devices, devID)
		result.DeviceCount++
	}

	if isJSON {
		jsonData, err := json.MarshalIndent(result, "", "  ")
		if err != nil {
			fmt.Printf("Error marshaling result: %v\n", err)
			return result
		}
		fmt.Println(string(jsonData))
	} else {
		fmt.Printf("Removed %d %s\n", result.DeviceCount, utils.Pluralize(result.DeviceCount, "device", "devices"))
	}

	return result
}

// pauseAndResumeDevices pauses and resumes devices to refresh connections
func (c *SyncwebDropCmd) pauseAndResumeDevices(s *syncweb.Syncweb, deviceIDs []string) {
	logger := slog.Default()
	for _, devID := range deviceIDs {
		if err := s.PauseDevice(devID); err != nil {
			logger.Warn("Failed to pause device", "device", devID, "error", err)
		}
	}
	for _, devID := range deviceIDs {
		if err := s.ResumeDevice(devID); err != nil {
			logger.Warn("Failed to resume device", "device", devID, "error", err)
		}
	}
}

// parseDropDeviceIDs parses comma-separated device IDs
func parseDropDeviceIDs(deviceIDs []string) []string {
	var result []string
	for _, id := range deviceIDs {
		parts := strings.SplitSeq(id, ",")
		for p := range parts {
			p = strings.TrimSpace(p)
			if p != "" {
				result = append(result, p)
			}
		}
	}
	return result
}
