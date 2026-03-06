package commands

import (
	"fmt"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
	"github.com/syncthing/syncthing/lib/protocol"
)

// SyncwebLsCmd lists files at the current directory level
type SyncwebLsCmd struct {
	Paths         []string `arg:"" optional:"" default:"." help:"Path relative to the root"`
	Long          bool     `short:"l" help:"Use long listing format"`
	HumanReadable bool     `help:"Print sizes in human readable format" default:"true"`
	FolderSize    bool     `help:"Include accurate subfolder size" default:"true"`
	ShowAll       bool     `short:"a" help:"Do not ignore entries starting with ."`
	Depth         int      `short:"D" help:"Descend N directory levels deep" default:"0"`
	NoHeader      bool     `help:"Suppress header in long format"`
}

func (c *SyncwebLsCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		headerPrinted := !(c.Long && !c.NoHeader)

		printHeader := func() {
			if headerPrinted {
				return
			}
			fmt.Printf("%-4s %10s  %12s  %s\n", "Type", "Size", "Modified", "Name")
			fmt.Println(strings.Repeat("-", 45))
			headerPrinted = true
		}

		for _, p := range c.Paths {
			absPath, _ := filepath.Abs(p)

			// Find which folder this path belongs to
			var folderID string
			var prefix string

			if after, ok := strings.CutPrefix(p, "syncweb://"); ok {
				// Parse syncweb:// URL
				parts := strings.SplitN(after, "/", 2)
				folderID = parts[0]
				if len(parts) > 1 {
					prefix = parts[1]
				}
			} else {
				// Find folder by path
				cfg := s.Node.Cfg.RawCopy()
				for _, f := range cfg.Folders {
					if strings.HasPrefix(absPath, f.Path) {
						folderID = f.ID
						rel, _ := filepath.Rel(f.Path, absPath)
						if rel != "." {
							prefix = rel
						}
						break
					}
				}
			}

			if folderID == "" {
				fmt.Printf("Error: %s is not inside of a Syncweb folder\n", p)
				continue
			}

			// Get files from Syncthing
			files := c.getFiles(s, folderID, prefix)

			if len(files) == 0 {
				// Might be a file, not a directory
				if prefix != "" {
					fileInfo, ok := c.getFile(s, folderID, prefix)
					if ok {
						if !headerPrinted {
							printHeader()
						}
						c.printEntry(&fileInfo, printHeader)
						continue
					}
				}
				continue
			}

			if !headerPrinted {
				printHeader()
			}

			// Print files
			c.printDirectory(files, 0, printHeader)
		}
		return nil
	})
}

func (c *SyncwebLsCmd) getFiles(s *syncweb.Syncweb, folderID, prefix string) []*fileEntry {
	seq, cancel := s.Node.App.Internals.AllGlobalFiles(folderID)
	defer cancel()

	// Build a tree structure
	tree := make(map[string]*fileEntry)
	var rootItems []*fileEntry

	for meta := range seq {
		name := meta.Name

		// Filter by prefix
		if prefix != "" {
			if name == prefix {
				continue
			}
			if !strings.HasPrefix(name, prefix+"/") {
				continue
			}
			// Remove prefix
			name = strings.TrimPrefix(name, prefix+"/")
		}

		// Split into parts
		parts := strings.Split(name, "/")
		if len(parts) == 0 {
			continue
		}

		// Check depth
		if c.Depth > 0 && len(parts) > c.Depth {
			continue
		}

		// Build tree
		var currentMap map[string]*fileEntry = tree
		var currentPath string

		for i, part := range parts {
			isLast := i == len(parts)-1
			isDir := !isLast

			entryPath := part
			if currentPath != "" {
				entryPath = currentPath + "/" + part
			}

			if _, exists := currentMap[part]; !exists {
				entry := &fileEntry{
					Name:     part,
					Path:     entryPath,
					IsDir:    isDir,
					Size:     0,
					ModTime:  time.Time{},
					Children: make(map[string]*fileEntry),
				}

				if isLast {
					entry.Size = meta.Size
					entry.ModTime = meta.ModTime()
				}

				currentMap[part] = entry
				if currentPath == "" {
					rootItems = append(rootItems, entry)
				}
			}

			currentMap = currentMap[part].Children
			if currentPath == "" {
				currentPath = part
			} else {
				currentPath = currentPath + "/" + part
			}
		}
	}

	// Calculate folder sizes if needed
	if c.FolderSize && c.Long {
		c.calculateFolderSizes(rootItems)
	}

	return rootItems
}

func (c *SyncwebLsCmd) getFile(s *syncweb.Syncweb, folderID, path string) (fileEntry, bool) {
	info, ok, err := s.GetGlobalFileInfo(folderID, path)
	if err != nil || !ok {
		return fileEntry{}, false
	}

	return fileEntry{
		Name:    filepath.Base(path),
		Path:    path,
		IsDir:   info.Type == protocol.FileInfoTypeDirectory,
		Size:    info.Size,
		ModTime: info.ModTime(),
	}, true
}

func (c *SyncwebLsCmd) calculateFolderSizes(items []*fileEntry) {
	for _, item := range items {
		if item.IsDir {
			item.Size = c.calculateDirSize(item)
		}
	}
}

func (c *SyncwebLsCmd) calculateDirSize(item *fileEntry) int64 {
	if !item.IsDir {
		return item.Size
	}

	var total int64
	for _, child := range item.Children {
		if child.IsDir {
			total += c.calculateDirSize(child)
		} else {
			total += child.Size
		}
	}
	return total
}

func (c *SyncwebLsCmd) printDirectory(items []*fileEntry, indent int, printHeader func()) {
	// Sort items: directories first, then files, alphabetically
	sort.Slice(items, func(i, j int) bool {
		if items[i].IsDir != items[j].IsDir {
			return items[i].IsDir // directories first
		}
		return items[i].Name < items[j].Name
	})

	for _, item := range items {
		// Skip hidden files unless ShowAll is true
		if !c.ShowAll && strings.HasPrefix(item.Name, ".") {
			continue
		}

		// Print indentation
		for range indent {
			fmt.Print("  ")
		}

		c.printEntry(item, printHeader)

		// Recurse into directories
		if item.IsDir && len(item.Children) > 0 && indent < c.Depth {
			if indent == 0 {
				fmt.Printf("\n\x1b[4m%s\x1b[0m:\n", item.Name)
			}
			// Convert map to slice
			children := make([]*fileEntry, 0, len(item.Children))
			for _, child := range item.Children {
				children = append(children, child)
			}
			c.printDirectory(children, indent+1, printHeader)
			if indent == 0 {
				fmt.Println()
			}
		}
	}
}

func (c *SyncwebLsCmd) printEntry(item *fileEntry, printHeader func()) {
	if c.Long {
		typeChar := "d"
		if !item.IsDir {
			typeChar = "-"
		}

		sizeStr := "-"
		if !item.IsDir || c.FolderSize {
			if c.HumanReadable {
				sizeStr = utils.FormatSize(item.Size)
			} else {
				sizeStr = fmt.Sprintf("%d", item.Size)
			}
		}

		timeStr := utils.FormatTimeLong(item.ModTime.Unix())

		name := item.Name
		if item.IsDir {
			name += "/"
		}

		fmt.Printf("%-4s %10s  %12s  %s\n", typeChar, sizeStr, timeStr, name)
	} else {
		name := item.Name
		if item.IsDir {
			name += "/"
		}
		fmt.Println(name)
	}
}

type fileEntry struct {
	Name     string
	Path     string
	IsDir    bool
	Size     int64
	ModTime  time.Time
	Children map[string]*fileEntry
}
