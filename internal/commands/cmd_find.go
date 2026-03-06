package commands

import (
	"fmt"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"
)

// SyncwebFindCmd searches for files by filename, size, and modified date
type SyncwebFindCmd struct {
	Pattern     string   `arg:"" optional:"" default:".*" help:"Search patterns"`
	Type        string   `short:"t" help:"Filter by type (f=file, d=directory)"`
	FullPath    bool     `short:"p" help:"Search full path (default: filename only)"`
	IgnoreCase  bool     `short:"i" help:"Case insensitive search"`
	CaseSensitive bool   `short:"s" help:"Case sensitive search"`
	FixedStrings bool    `short:"F" help:"Treat all patterns as literals"`
	Glob        bool     `short:"g" help:"Glob-based search"`
	Hidden      bool     `short:"H" help:"Search hidden files and directories"`
	FollowLinks bool     `short:"L" help:"Follow symbolic links"`
	AbsolutePath bool    `short:"a" help:"Print absolute paths"`
	Downloadable bool    `help:"Exclude sendonly folders"`
	Depth       []string `short:"d" help:"Depth constraints (e.g., +2, -3, 2)"`
	MinDepth    int      `help:"Minimum depth"`
	MaxDepth    int      `help:"Maximum depth"`
	Sizes       []string `short:"S" help:"Size constraints"`
	Ext         []string `short:"e" help:"File extensions to include"`
	Paths       []string `arg:"" optional:"" help:"Root directories to search"`
}

func (c *SyncwebFindCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		// Build regex pattern
		pattern := c.Pattern
		if !c.CaseSensitive && (c.IgnoreCase || pattern == strings.ToLower(pattern)) {
			pattern = "(?i)" + pattern
		}

		re, err := regexp.Compile(pattern)
		if err != nil {
			return fmt.Errorf("invalid regex: %w", err)
		}

		cfg := s.Node.Cfg.RawCopy()
		for _, f := range cfg.Folders {
			// Skip sendonly folders if downloadable flag is set
			if c.Downloadable && f.Type == config.FolderTypeSendOnly {
				continue
			}

			seq, cancel := s.Node.App.Internals.AllGlobalFiles(f.ID)
			for meta := range seq {
				isDir := meta.Type == protocol.FileInfoTypeDirectory

				// Type filter
				if c.Type == "f" && isDir {
					continue
				}
				if c.Type == "d" && !isDir {
					continue
				}

				// Hidden file filter
				if !c.Hidden && strings.HasPrefix(meta.Name, ".") {
					continue
				}

				// Extension filter
				if len(c.Ext) > 0 {
					matched := false
					for _, ext := range c.Ext {
						if strings.HasSuffix(strings.ToLower(meta.Name), strings.ToLower(ext)) {
							matched = true
							break
						}
					}
					if !matched {
						continue
					}
				}

				// Search target
				searchTarget := meta.Name
				if !c.FullPath {
					searchTarget = filepath.Base(meta.Name)
				}

				if re.MatchString(searchTarget) {
					path := fmt.Sprintf("syncweb://%s/%s", f.ID, meta.Name)
					fmt.Println(path)
				}
			}
			cancel()
		}
		return nil
	})
}
