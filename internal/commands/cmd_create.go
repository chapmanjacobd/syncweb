package commands

import (
	"fmt"
	"path/filepath"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
	"github.com/syncthing/syncthing/lib/config"
)

// SyncwebCreateCmd creates a new syncweb folder
type SyncwebCreateCmd struct {
	Paths []string `arg:"" optional:"" default:"." help:"Path to folder"`
}

func (c *SyncwebCreateCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		// Get existing folder IDs to avoid collisions
		existingFolders := make(map[string]bool)
		for _, f := range s.GetFolders() {
			existingFolders[f.ID] = true
		}

		for _, p := range c.Paths {
			absPath, err := filepath.Abs(p)
			if err != nil {
				return fmt.Errorf("failed to resolve path %s: %w", p, err)
			}

			// Generate folder ID
			folderID := utils.CreateFolderID(absPath, existingFolders)
			existingFolders[folderID] = true

			// Create directory if it doesn't exist
			if err := utils.EnsureDir(absPath); err != nil {
				return fmt.Errorf("failed to create directory %s: %w", absPath, err)
			}

			// Add folder as sendonly
			if err := s.AddFolder(folderID, filepath.Base(absPath), absPath, config.FolderTypeSendOnly); err != nil {
				return fmt.Errorf("failed to add folder %s: %w", folderID, err)
			}

			// Set empty ignore patterns
			if err := s.SetIgnores(folderID, []string{}); err != nil {
				return fmt.Errorf("failed to set ignores for %s: %w", folderID, err)
			}

			// Trigger scan to index files immediately
			s.ScanFolderSubdirs(folderID, []string{""})

			// Print syncweb URL
			fmt.Printf("sync://%s#%s\n", folderID, s.Node.MyID())
		}
		return nil
	})
}
