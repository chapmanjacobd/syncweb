package commands

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/syncthing/syncthing/lib/config"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// Join command examples
const joinExamples = `
Examples:
  # Join a syncweb folder/device
  syncweb join sync://audio#CKTVWGQ-XBRFFRH-YTRPQ5G-YDA5YXI-N66GA5J-XVBGEZ3-PD56G6Y-N7TEAQC

  # Join with custom prefix path
  syncweb join --prefix=/data sync://music#DEVICE-ID

  # Join multiple URLs
  syncweb join sync://folder1#DEV1 sync://folder2#DEV2

  # Join specific subfolder for immediate download
  syncweb join sync://music/albums#DEVICE-ID
`

// SyncwebJoinCmd joins sync folders/devices
type SyncwebJoinCmd struct {
	URLs   []string `help:"Sync URLs (sync://folder-id#device-id)" required:"true" name:"urls" arg:""`
	Prefix string   `help:"Path to parent folder"                                                     default:"." env:"SYNCWEB_HOME"`
}

// Help displays examples for the join command
func (c *SyncwebJoinCmd) Help() string {
	return joinExamples
}

func (c *SyncwebJoinCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		deviceCount := 0
		folderCount := 0

		for _, url := range c.URLs {
			dc, fc := c.processURL(s, url)
			deviceCount += dc
			folderCount += fc
		}

		fmt.Printf("Local Device ID: %s\n", s.Node.MyID())
		fmt.Printf("Added %d %s\n", deviceCount, utils.Pluralize(deviceCount, "device", "devices"))
		fmt.Printf("Added %d %s\n", folderCount, utils.Pluralize(folderCount, "folder", "folders"))
		return nil
	})
}

// processURL processes a single syncweb URL and returns device and folder counts
func (c *SyncwebJoinCmd) processURL(s *syncweb.Syncweb, url string) (deviceCount, folderCount int) {
	ref, parseErr := utils.ParseSyncwebPath(url, true)
	if parseErr != nil {
		fmt.Printf("Invalid URL format %s: %v\n", url, parseErr)
		return 0, 0
	}

	// Add device if specified
	if ref.DeviceID != "" {
		if addDevErr := s.AddDevice(ref.DeviceID, "", false); addDevErr != nil {
			fmt.Printf("Failed to add device %s: %v\n", ref.DeviceID, addDevErr)
		} else {
			deviceCount = 1
		}
	}

	// Add folder if specified
	if ref.FolderID != "" {
		if c.addFolderAndDevice(s, ref) {
			folderCount = 1
		}
	}

	return deviceCount, folderCount
}

// addFolderAndDevice adds a folder and optionally shares it with a device
func (c *SyncwebJoinCmd) addFolderAndDevice(s *syncweb.Syncweb, ref *utils.SyncwebRef) bool {
	prefix := c.Prefix
	if prefix == "" {
		prefix = "."
	}
	path := filepath.Join(prefix, ref.FolderID)

	folderID, _, ok := c.resolveFolderID(s, ref.FolderID, path)
	if !ok {
		return false
	}

	// Create directory
	if mkdirErr := os.MkdirAll(path, 0o755); mkdirErr != nil {
		fmt.Printf("Failed to create directory %s: %v\n", path, mkdirErr)
		return false
	}

	// Add folder as receiveonly, paused
	if !c.createOrUseFolder(s, folderID, ref.FolderID, path) {
		return false
	}

	// Share folder with device
	if ref.DeviceID != "" {
		if shareErr := s.AddFolderDevice(folderID, ref.DeviceID); shareErr != nil {
			fmt.Printf("Failed to share folder with device: %v\n", shareErr)
			return false
		}
	}

	// Add subpath for download if specified
	if ref.Subpath != "" {
		if err := s.AddIgnores(folderID, []string{ref.Subpath}); err != nil {
			fmt.Printf("Failed to add subpath for download: %v\n", err)
			return false
		}
	}

	return true
}

// resolveFolderID resolves the folder ID and absolute path
func (c *SyncwebJoinCmd) resolveFolderID(
	s *syncweb.Syncweb,
	folderID, path string,
) (resolvedFolderID, absPath string, ok bool) {
	existingFolders := make(map[string]bool)
	for _, f := range s.GetFolders() {
		existingFolders[f.ID] = true
	}

	absPath, absErr := filepath.Abs(path)
	if absErr != nil {
		fmt.Printf("Error resolving path %s: %v\n", path, absErr)
		return "", "", false
	}

	if _, exists := existingFolders[folderID]; !exists {
		// Check if path is already a folder root
		for _, f := range s.GetFolders() {
			if f.Path == absPath {
				resolvedFolderID = f.ID
				break
			}
		}
	}

	return resolvedFolderID, absPath, true
}

// createOrUseFolder creates a new folder or uses existing one
func (c *SyncwebJoinCmd) createOrUseFolder(s *syncweb.Syncweb, folderID, label, path string) bool {
	existingFolders := make(map[string]bool)
	for _, f := range s.GetFolders() {
		existingFolders[f.ID] = true
	}

	if _, exists := existingFolders[folderID]; exists {
		return true
	}

	if addFldErr := s.AddFolder(folderID, label, path, config.FolderTypeReceiveOnly); addFldErr != nil {
		fmt.Printf("Failed to add folder %s: %v\n", folderID, addFldErr)
		return false
	}

	// Set empty ignores and resume
	if ignoreErr := s.SetIgnores(folderID, []string{}); ignoreErr != nil {
		fmt.Printf("Failed to set ignores: %v\n", ignoreErr)
		return false
	}

	if resumeErr := s.ResumeFolder(folderID); resumeErr != nil {
		fmt.Printf("Failed to resume folder: %v\n", resumeErr)
		return false
	}

	return true
}
