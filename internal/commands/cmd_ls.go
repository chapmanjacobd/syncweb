package commands

import (
	"encoding/json"
	"fmt"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/syncthing/syncthing/lib/protocol"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// Ls command examples
const lsExamples = `
Examples:
  # List files in current directory
  syncweb ls

  # Long listing format
  syncweb ls -l

  # List with human-readable sizes
  syncweb ls -l --human-readable

  # Include hidden files
  syncweb ls -a

  # Descend 2 directory levels
  syncweb ls -D2

  # List specific paths
  syncweb ls music/ documents/
`

// SyncwebLsCmd lists files at the current directory level
type SyncwebLsCmd struct {
	Paths         []string `help:"Path relative to the root"             default:"."    arg:"" optional:""`
	Long          bool     `help:"Use long listing format"                                                 short:"l"`
	HumanReadable bool     `help:"Print sizes in human readable format"  default:"true"`
	FolderSize    bool     `help:"Include accurate subfolder size"       default:"true"`
	ShowAll       bool     `help:"Do not ignore entries starting with ."                                   short:"a"`
	Depth         int      `help:"Descend N directory levels deep"       default:"0"                       short:"D"`
	NoHeader      bool     `help:"Suppress header in long format"`
}

// Help displays examples for the ls command
func (c *SyncwebLsCmd) Help() string {
	return lsExamples
}

func (c *SyncwebLsCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		headerPrinted := !c.Long || c.NoHeader
		allEntries := []*fileEntry{}

		printHeader := func() {
			if headerPrinted {
				return
			}
			fmt.Printf("%-4s %10s  %12s  %s\n", "Type", "Size", "Modified", "Name")
			fmt.Println(strings.Repeat("-", 45))
			headerPrinted = true
		}

		for _, p := range c.Paths {
			ctx := &lsPrintHeaderContext{
				allEntries:    &allEntries,
				headerPrinted: &headerPrinted,
				printHeader:   printHeader,
			}
			if err := c.processPath(p, s, g, ctx); err != nil {
				return err
			}
		}

		if g.JSON {
			return c.outputJSON(allEntries)
		}

		return nil
	})
}

func (c *SyncwebLsCmd) processPath(
	p string,
	s *syncweb.Syncweb,
	g *SyncwebCmd,
	ctx *lsPrintHeaderContext,
) error {
	folderID, prefix, ok := c.findFolderForPath(p, s)
	if !ok {
		if !g.JSON {
			fmt.Printf("Error: %s is not inside of a Syncweb folder\n", p)
		}
		return nil
	}

	// Wait for Syncthing to index local files
	_ = s.WaitUntilIdle(folderID, 5*time.Second)

	// Get files from Syncthing
	files := c.getFiles(s, folderID, prefix)

	if len(files) == 0 {
		c.handleEmptyFilesWithContext(
			s,
			folderID,
			prefix,
			g,
			ctx,
		)
		return nil
	}

	if g.JSON {
		*ctx.allEntries = append(*ctx.allEntries, files...)
	} else {
		if !*ctx.headerPrinted {
			ctx.printHeader()
		}
		// Print files
		c.printDirectory(files, 0, ctx.printHeader)
	}

	return nil
}

func (c *SyncwebLsCmd) outputJSON(allEntries []*fileEntry) error {
	data, err := json.MarshalIndent(allEntries, "", "  ")
	if err != nil {
		return err
	}
	fmt.Println(string(data))
	return nil
}

// lsPrintHeaderContext holds context for printing header
type lsPrintHeaderContext struct {
	allEntries    *[]*fileEntry
	headerPrinted *bool
	printHeader   func()
}

func (c *SyncwebLsCmd) handleEmptyFilesWithContext(
	s *syncweb.Syncweb,
	folderID, prefix string,
	g *SyncwebCmd,
	ctx *lsPrintHeaderContext,
) {
	// Might be a file, not a directory
	if prefix != "" {
		fileInfo, ok := c.getFile(s, folderID, prefix)
		if ok {
			if g.JSON {
				*ctx.allEntries = append(*ctx.allEntries, &fileInfo)
			} else {
				if !*ctx.headerPrinted {
					ctx.printHeader()
				}
				c.printEntry(&fileInfo, ctx.printHeader)
			}
		}
	}
}

// findFolderForPath finds the folder ID and prefix for a given path
func (c *SyncwebLsCmd) findFolderForPath(path string, s *syncweb.Syncweb) (folderID, prefix string, ok bool) {
	absPath, err := filepath.Abs(path)
	if err != nil {
		return "", "", false
	}
	absPath = filepath.Clean(absPath)

	var after string
	var hasPrefix bool
	if after, hasPrefix = strings.CutPrefix(path, "sync://"); !hasPrefix {
		after, hasPrefix = strings.CutPrefix(path, "syncweb://")
	}

	if hasPrefix {
		// Parse sync:// or syncweb:// URL
		parts := strings.SplitN(after, "/", 2)
		folderID = parts[0]
		if len(parts) > 1 {
			prefix = parts[1]
		}
		return folderID, prefix, true
	}

	// Find folder by path
	cfg := s.Node.Cfg.RawCopy()
	for _, f := range cfg.Folders {
		fPath := filepath.Clean(f.Path)
		if absPath == fPath || strings.HasPrefix(absPath, fPath+string(filepath.Separator)) {
			folderID = f.ID
			rel, err := filepath.Rel(fPath, absPath)
			if err != nil {
				return "", "", false
			}
			if rel != "." {
				prefix = rel
			}
			return folderID, prefix, true
		}
	}

	return "", "", false
}

func (c *SyncwebLsCmd) getFiles(s *syncweb.Syncweb, folderID, prefix string) []*fileEntry {
	seq, cancel := s.Node.App.Internals.AllGlobalFiles(folderID)
	defer func() { _ = cancel() }()

	// Build a tree structure
	tree := make(map[string]*fileEntry)
	var rootItems []*fileEntry

	for meta := range seq {
		c.processFile(meta, prefix, tree, &rootItems)
	}

	// Calculate folder sizes if needed
	if c.FolderSize && c.Long {
		c.calculateFolderSizes(rootItems)
	}

	return rootItems
}

func (c *SyncwebLsCmd) processFile(meta any, prefix string, tree map[string]*fileEntry, rootItems *[]*fileEntry) {
	// Access meta fields using helper functions
	name := getMetaName(meta)
	_ = getMetaType(meta)
	sizeVal := getMetaSize(meta)
	modTimeVal := getMetaModTime(meta)

	// Filter by prefix
	if prefix != "" {
		if name == prefix || !strings.HasPrefix(name, prefix+"/") {
			return
		}
		name = strings.TrimPrefix(name, prefix+"/")
	}

	// Split into parts
	parts := strings.Split(name, "/")
	if len(parts) == 0 {
		return
	}

	// Check depth
	if c.Depth > 0 && len(parts) > c.Depth {
		return
	}

	// Build tree
	currentMap := tree
	var currentPath string
	var isNewRootItem bool

	for i, part := range parts {
		isLast := i == len(parts)-1

		entryPath := part
		if currentPath != "" {
			entryPath = currentPath + "/" + part
		}

		if _, exists := currentMap[part]; !exists {
			entry := &fileEntry{
				Name:     part,
				Path:     entryPath,
				IsDir:    !isLast,
				Size:     0,
				ModTime:  time.Time{},
				Children: make(map[string]*fileEntry),
			}

			if isLast {
				entry.Size = sizeVal
				entry.ModTime = modTimeVal
			}

			currentMap[part] = entry
			isNewRootItem = (currentPath == "")
		} else if !isLast {
			currentMap[part].IsDir = true
		}

		currentMap = currentMap[part].Children
		if currentPath == "" {
			currentPath = part
		} else {
			currentPath = currentPath + "/" + part
		}
	}

	if isNewRootItem {
		if entry, ok := currentMap[parts[0]]; ok {
			*rootItems = append(*rootItems, entry)
		}
	}
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

func (c *SyncwebLsCmd) printEntry(item *fileEntry, _ func()) {
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
				sizeStr = strconv.FormatInt(item.Size, 10)
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
	Name     string                `json:"name"`
	Path     string                `json:"path"`
	IsDir    bool                  `json:"is_dir"`
	Size     int64                 `json:"size"`
	ModTime  time.Time             `json:"mod_time"`
	Children map[string]*fileEntry `json:"children,omitempty"`
}
