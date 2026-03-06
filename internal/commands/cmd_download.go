package commands

import (
	"fmt"
	"path/filepath"
	"strings"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// SyncwebDownloadCmd marks file paths for download/sync
type SyncwebDownloadCmd struct {
	Paths  []string `arg:"" optional:"" help:"File or directory paths to download"`
	Depth  int      `help:"Maximum depth for directory traversal"`
}

func (c *SyncwebDownloadCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		type downloadItem struct {
			folderID string
			relPath  string
			size     int64
		}
		var items []downloadItem
		var totalSize int64

		for _, p := range c.Paths {
			absPath, _ := filepath.Abs(p)

			// Find folder
			var folderID string
			var relPath string

			cfg := s.Node.Cfg.RawCopy()
			for _, f := range cfg.Folders {
				if strings.HasPrefix(absPath, f.Path) {
					folderID = f.ID
					rel, _ := filepath.Rel(f.Path, absPath)
					relPath = rel
					break
				}
			}

			if folderID == "" {
				fmt.Printf("Warning: %s is not inside a Syncweb folder\n", p)
				continue
			}

			info, ok, err := s.GetGlobalFileInfo(folderID, relPath)
			if err != nil || !ok {
				fmt.Printf("Warning: %s not found in cluster\n", p)
				continue
			}

			items = append(items, downloadItem{folderID, relPath, info.Size})
			totalSize += info.Size
		}

		if len(items) == 0 {
			fmt.Println("No files found to download")
			return nil
		}

		// Show summary
		fmt.Printf("\nDownload Summary:\n")
		fmt.Println(strings.Repeat("-", 60))
		fmt.Printf("%-20s %-30s %10s\n", "Folder ID", "Path", "Size")
		fmt.Println(strings.Repeat("-", 60))
		for _, item := range items {
			fmt.Printf("%-20s %-30s %10s\n", item.folderID, item.relPath, utils.FormatSize(item.size))
		}
		fmt.Println(strings.Repeat("-", 60))
		fmt.Printf("TOTAL: %d files (%s)\n", len(items), utils.FormatSize(totalSize))

		// Confirm
		if !g.NoConfirm && !g.Yes {
			var response string
			fmt.Printf("\nMark %d files for download? [y/N]: ", len(items))
			fmt.Scanln(&response)
			if strings.ToLower(response) != "y" && strings.ToLower(response) != "yes" {
				fmt.Println("Download cancelled")
				return nil
			}
		}

		// Trigger downloads
		for _, item := range items {
			if err := s.Unignore(item.folderID, item.relPath); err != nil {
				fmt.Printf("Error: Failed to trigger download for %s: %v\n", item.relPath, err)
			} else {
				fmt.Printf("Queued: %s\n", item.relPath)
			}
		}

		return nil
	})
}
