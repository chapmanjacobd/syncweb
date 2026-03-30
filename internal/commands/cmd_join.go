package commands

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
	"github.com/syncthing/syncthing/lib/config"
)

// SyncwebJoinCmd joins sync folders/devices
type SyncwebJoinCmd struct {
	URLs   []string `arg:"" required:"" name:"urls" help:"Sync URLs (sync://folder-id#device-id)"`
	Prefix string   `help:"Path to parent folder" env:"SYNCWEB_HOME" default:"."`
}

func (c *SyncwebJoinCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		deviceCount := 0
		folderCount := 0

		for _, url := range c.URLs {
			// Parse syncweb URL
			ref, err := utils.ParseSyncwebPath(url, true)
			if err != nil {
				fmt.Printf("Invalid URL format %s: %v\n", url, err)
				continue
			}

			// Add device if specified
			if ref.DeviceID != "" {
				if err := s.AddDevice(ref.DeviceID, "", false); err != nil {
					fmt.Printf("Failed to add device %s: %v\n", ref.DeviceID, err)
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

				absPath, _ := filepath.Abs(path)
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
				if err := os.MkdirAll(path, 0o755); err != nil {
					fmt.Printf("Failed to create directory %s: %v\n", path, err)
					continue
				}

				// Add folder as receiveonly, paused
				if _, exists := existingFolders[folderID]; !exists {
					if err := s.AddFolder(folderID, ref.FolderID, path, config.FolderTypeReceiveOnly); err != nil {
						fmt.Printf("Failed to add folder %s: %v\n", folderID, err)
						continue
					}

					// Set empty ignores and resume
					if err := s.SetIgnores(folderID, []string{}); err != nil {
						fmt.Printf("Failed to set ignores: %v\n", err)
						continue
					}

					if err := s.ResumeFolder(folderID); err != nil {
						fmt.Printf("Failed to resume folder: %v\n", err)
						continue
					}

					folderCount++
				}

				// Share folder with device
				if ref.DeviceID != "" {
					if err := s.AddFolderDevice(folderID, ref.DeviceID); err != nil {
						fmt.Printf("Failed to share folder with device: %v\n", err)
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

		fmt.Printf("Added %d %s\n", deviceCount, utils.Pluralize(deviceCount, "device", "devices"))
		fmt.Printf("Added %d %s\n", folderCount, utils.Pluralize(folderCount, "folder", "folders"))
		return nil
	})
}
