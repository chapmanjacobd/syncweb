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
	DeviceIDs  []string `help:"Syncthing device IDs (space or comma-separated)" required:"true" name:"device-ids" arg:""`
	FolderIDs  []string `help:"Add devices to folders"                                                                   short:"f"`
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
		deviceIDs := parseDeviceIDs(c.DeviceIDs)

		result := c.acceptDevices(s, deviceIDs, g.JSON)

		// Add devices to folders if specified
		if len(c.FolderIDs) > 0 && len(result.Devices) > 0 {
			c.addDevicesToFolders(s, c.FolderIDs, result.Devices, g.JSON, &result)
		}

		c.printResult(result, g.JSON)

		// Exit with error if all device IDs were invalid
		if len(deviceIDs) > 0 && result.DeviceCount == 0 {
			return errors.New("no valid devices were added")
		}

		return nil
	})
}

// parseDeviceIDs parses comma-separated device IDs
func parseDeviceIDs(deviceIDs []string) []string {
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

// acceptDevices validates and adds devices to syncweb
func (c *SyncwebAcceptCmd) acceptDevices(s *syncweb.Syncweb, deviceIDs []string, isJSON bool) AcceptResult {
	result := AcceptResult{
		Devices: []string{},
		Errors:  []string{},
	}

	for _, devID := range deviceIDs {
		if !c.validateAndAddDevice(s, devID, isJSON, &result) {
			continue
		}
		result.Devices = append(result.Devices, devID)
		result.DeviceCount++
	}

	return result
}

// validateAndAddDevice validates a device ID and adds it to syncweb
func (c *SyncwebAcceptCmd) validateAndAddDevice(
	s *syncweb.Syncweb,
	devID string,
	isJSON bool,
	result *AcceptResult,
) bool {
	if _, err := utils.ExtractDeviceID(devID); err != nil {
		errMsg := fmt.Sprintf("Invalid Device ID %s: %v", devID, err)
		result.Errors = append(result.Errors, errMsg)
		if !isJSON {
			fmt.Println(errMsg)
		}
		return false
	}

	if err := s.AddDevice(devID, "", c.Introducer); err != nil {
		errMsg := fmt.Sprintf("Failed to add device %s: %v", devID, err)
		result.Errors = append(result.Errors, errMsg)
		if !isJSON {
			fmt.Println(errMsg)
		}
		return false
	}

	return true
}

// addDevicesToFolders adds accepted devices to specified folders
func (c *SyncwebAcceptCmd) addDevicesToFolders(
	s *syncweb.Syncweb,
	folderIDs, deviceIDs []string,
	isJSON bool,
	result *AcceptResult,
) {
	for _, fldID := range folderIDs {
		if err := s.AddFolderDevices(fldID, deviceIDs); err != nil {
			errMsg := fmt.Sprintf("Failed to add devices to folder %s: %v", fldID, err)
			result.Errors = append(result.Errors, errMsg)
			if !isJSON {
				fmt.Println(errMsg)
			}
			continue
		}

		// Pause and resume devices to unstuck connections
		c.pauseAndResumeDevices(s, deviceIDs)
	}
}

// pauseAndResumeDevices pauses and resumes devices to refresh connections
func (c *SyncwebAcceptCmd) pauseAndResumeDevices(s *syncweb.Syncweb, deviceIDs []string) {
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

// printResult prints the accept result
func (c *SyncwebAcceptCmd) printResult(result AcceptResult, isJSON bool) {
	if isJSON {
		jsonData, err := json.MarshalIndent(result, "", "  ")
		if err != nil {
			fmt.Printf("Error marshaling result: %v\n", err)
			return
		}
		fmt.Println(string(jsonData))
	} else {
		fmt.Printf("Added %d %s\n", result.DeviceCount, utils.Pluralize(result.DeviceCount, "device", "devices"))
	}
}
