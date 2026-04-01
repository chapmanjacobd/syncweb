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
	URLs   []string `help:"Sync URLs (sync://folder-id#device-id)" required:"" name:"urls" arg:""`
	Prefix string   `help:"Path to parent folder"                                                 default:"." env:"SYNCWEB_HOME"`
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
			// Parse syncweb URL
			ref, parseErr := utils.ParseSyncwebPath(url, true)
			if parseErr != nil {
				fmt.Printf("Invalid URL format %s: %v\n", url, parseErr)
				continue
			}

			// Add device if specified
			if ref.DeviceID != "" {
				if addDevErr := s.AddDevice(ref.DeviceID, "", false); addDevErr != nil {
					fmt.Printf("Failed to add device %s: %v\n", ref.DeviceID, addDevErr)
					continue
				}
				deviceCount++
			}

			// Add folder if specified
			if ref.FolderID != "" {
				prefix := c.Prefix
				if prefix == "" {
					prefix = "."
				}
				path := filepath.Join(prefix, ref.FolderID)

				// Check if path already exists as a folder
				folderID := ref.FolderID
				existingFolders := make(map[string]bool)
				for _, f := range s.GetFolders() {
					existingFolders[f.ID] = true
				}

				absPath, absErr := filepath.Abs(path)
				if absErr != nil {
					fmt.Printf("Error resolving path %s: %v\n", path, absErr)
					continue
				}
				if _, exists := existingFolders[ref.FolderID]; !exists {
					// Check if path is already a folder root
					for _, f := range s.GetFolders() {
						if f.Path == absPath {
							folderID = f.ID
							break
						}
					}
				}

				// Create directory
				if mkdirErr := os.MkdirAll(path, 0o755); mkdirErr != nil {
					fmt.Printf("Failed to create directory %s: %v\n", path, mkdirErr)
					continue
				}

				// Add folder as receiveonly, paused
				if _, exists := existingFolders[folderID]; !exists {
					if addFldErr := s.AddFolder(
						folderID,
						ref.FolderID,
						path,
						config.FolderTypeReceiveOnly,
					); addFldErr != nil {
						fmt.Printf("Failed to add folder %s: %v\n", folderID, addFldErr)
						continue
					}

					// Set empty ignores and resume
					if ignoreErr := s.SetIgnores(folderID, []string{}); ignoreErr != nil {
						fmt.Printf("Failed to set ignores: %v\n", ignoreErr)
						continue
					}

					if resumeErr := s.ResumeFolder(folderID); resumeErr != nil {
						fmt.Printf("Failed to resume folder: %v\n", resumeErr)
						continue
					}

					folderCount++
				}

				// Share folder with device
				if ref.DeviceID != "" {
					if shareErr := s.AddFolderDevice(folderID, ref.DeviceID); shareErr != nil {
						fmt.Printf("Failed to share folder with device: %v\n", shareErr)
						continue
					}
				}

				// Add subpath for download if specified
				if ref.Subpath != "" {
					if err := s.AddIgnores(folderID, []string{ref.Subpath}); err != nil {
						fmt.Printf("Failed to add subpath for download: %v\n", err)
						continue
					}
				}
			}
		}

		fmt.Printf("Local Device ID: %s\n", s.Node.MyID())
		fmt.Printf("Added %d %s\n", deviceCount, utils.Pluralize(deviceCount, "device", "devices"))
		fmt.Printf("Added %d %s\n", folderCount, utils.Pluralize(folderCount, "folder", "folders"))
		return nil
	})
}
