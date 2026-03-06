package commands

import (
	"fmt"
	"path/filepath"
	"strings"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
)

// SyncwebStatCmd displays detailed file status information
type SyncwebStatCmd struct {
	Paths       []string `arg:"" required:"" help:"Files or directories to stat"`
	Terse       bool     `short:"t" help:"Print information in terse form"`
	Format      string   `short:"c" help:"Use custom format"`
	Dereference bool     `short:"L" help:"Follow symbolic links"`
}

func (c *SyncwebStatCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		for _, p := range c.Paths {
			absPath, _ := filepath.Abs(p)

			// Find folder and relative path
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
				fmt.Printf("Error: %s is not inside of a Syncweb folder\n", p)
				continue
			}

			info, ok, err := s.GetGlobalFileInfo(folderID, relPath)
			if err != nil {
				fmt.Printf("Error: %v\n", err)
				continue
			}
			if !ok {
				fmt.Printf("%s: Not found in cluster\n", p)
				continue
			}

			if c.Terse {
				// Terse format: name|size|blocks|permissions|type|modified|device_count|has_diffs
				fmt.Printf("%s|%d|%d|%o|file|%d|1|0\n",
					info.Name,
					info.Size,
					len(info.Blocks),
					info.Permissions,
					info.ModTime().Unix(),
				)
			} else if c.Format != "" {
				// Custom format
				output := c.Format
				output = strings.ReplaceAll(output, "%n", info.Name)
				output = strings.ReplaceAll(output, "%s", fmt.Sprintf("%d", info.Size))
				output = strings.ReplaceAll(output, "%b", fmt.Sprintf("%d", len(info.Blocks)))
				output = strings.ReplaceAll(output, "%f", fmt.Sprintf("%o", info.Permissions))
				output = strings.ReplaceAll(output, "%y", info.ModTime().Format("2006-01-02 15:04:05"))
				fmt.Println(output)
			} else {
				// Full format
				fmt.Printf("  File: %s\n", info.Name)
				fmt.Printf("  Size: %-15d Blocks: %-10d %s\n", info.Size, len(info.Blocks), "regular file")
				fmt.Printf("Device: %-15d Version: %v\n", 1, "local")
				fmt.Printf("Access: (%o/---------)\n", info.Permissions)
				fmt.Printf("Modify: %s\n", info.ModTime().Format("2006-01-02 15:04:05"))
			}
		}
		return nil
	})
}
