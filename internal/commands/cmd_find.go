package commands

import (
	"encoding/json"
	"fmt"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"time"

	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// Find command examples (displayed in help)
const findExamples = `
Examples:
  # Search by depth
  syncweb find -d 2              # Show only items at depth 2
  syncweb find -d=+2             # Show items at depth 2 and deeper
  syncweb find -d=-2             # Show items up to depth 2
  syncweb find -d=+1 -d=-3       # Show items from depth 1 to 3

  # Search by size
  syncweb find -S 6              # 6 MB exactly
  syncweb find -S-6              # Less than 6 MB
  syncweb find -S+6              # More than 6 MB
  syncweb find -S 6%10           # 6 MB ±10 percent
  syncweb find -S+5GB -S-7GB     # Between 5 and 7 GB

  # Search by modification time
  syncweb find --modified-within '3 days'    # Modified in last 3 days
  syncweb find --modified-before '3 years'   # Modified more than 3 years ago
  syncweb find --time-modified='-3 days'     # Newer than 3 days ago
  syncweb find --time-modified='+3 days'     # Older than 3 days ago
`

// SyncwebFindCmd searches for files by filename, size, and modified date
type SyncwebFindCmd struct {
	Pattern        string   `help:"Search patterns (default: all files)"                  default:".*" arg:"" optional:""`
	Type           string   `help:"Filter by type: f=file, d=directory"                                                   short:"t"`
	FullPath       bool     `help:"Search full abs. path (default: filename only)"                                        short:"p"`
	IgnoreCase     bool     `help:"Case insensitive search"                                                               short:"i"`
	CaseSensitive  bool     `help:"Case sensitive search"                                                                 short:"s"`
	FixedStrings   bool     `help:"Treat all patterns as literals"                                                        short:"F"`
	Glob           bool     `help:"Glob-based search"                                                                     short:"g"`
	Exact          bool     `help:"Exact match search"                                                                    short:"x"`
	Hidden         bool     `help:"Search hidden files and directories"                                                   short:"H"`
	FollowLinks    bool     `help:"Follow symbolic links"                                                                 short:"L"`
	AbsolutePath   bool     `help:"Print absolute paths"                                                                  short:"a"`
	Downloadable   bool     `help:"Exclude sendonly folders"`
	Depth          []string `help:"Constrain files by file depth"                                                         short:"d"`
	MinDepth       int      `help:"Alternative depth notation (default: 0)"`
	MaxDepth       int      `help:"Alternative depth notation"`
	Size           []string `help:"Constrain files by file size"                                                          short:"S"`
	ModifiedWithin string   `help:"Constrain files by time_modified (newer than)"`
	ModifiedBefore string   `help:"Constrain files by time_modified (older than)"`
	TimeModified   []string `help:"Constrain media by time_modified (alternative syntax)"`
	Ext            []string `help:"Include only specific file extensions"                                                 short:"e"`
	Paths          []string `help:"Root directories to search"                                         arg:"" optional:""`
}

// Help displays examples for the find command
func (c *SyncwebFindCmd) Help() string {
	return findExamples
}

func (c *SyncwebFindCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		results := []findResult{}

		// Build search pattern based on mode
		matchFunc, err := c.buildMatchFunc()
		if err != nil {
			return err
		}

		// Parse size constraints
		sizeMin, sizeMax, err := c.parseSizeConstraints()
		if err != nil {
			return err
		}

		// Parse time constraints
		modifiedAfterTS, modifiedBeforeTS, err := c.parseTimeConstraints()
		if err != nil {
			return err
		}

		// Parse depth constraints
		depthMin, depthMax := c.parseDepthConstraints()

		cfg := s.Node.Cfg.RawCopy()
		for _, f := range cfg.Folders {
			// Skip sendonly folders if downloadable flag is set
			if c.Downloadable && f.Type == config.FolderTypeSendOnly {
				continue
			}

			// Wait for Syncthing to index local files
			time.Sleep(1 * time.Second)

			seq, cancel := s.Node.App.Internals.AllGlobalFiles(f.ID)
			for meta := range seq {
				result, include := c.processFile(
					meta,
					matchFunc,
					sizeMin,
					sizeMax,
					modifiedAfterTS,
					modifiedBeforeTS,
					depthMin,
					depthMax,
					f,
					g,
				)
				if include {
					results = append(results, result)
				}
			}
			cancel() //nolint:errcheck // cancel function never returns an error
		}

		if g.JSON {
			data, err := json.MarshalIndent(results, "", "  ")
			if err != nil {
				return err
			}
			fmt.Println(string(data))
		}

		return nil
	})
}

func (c *SyncwebFindCmd) buildMatchFunc() (func(string) bool, error) {
	if c.Exact {
		// Exact match mode
		pattern := c.Pattern
		if c.IgnoreCase || (!c.CaseSensitive && pattern == strings.ToLower(pattern)) {
			return func(target string) bool {
				return strings.EqualFold(target, pattern)
			}, nil
		}
		return func(target string) bool {
			return target == pattern
		}, nil
	}

	if c.Glob {
		// Glob match mode
		pattern := GlobToRegex(c.Pattern)
		if c.IgnoreCase || (!c.CaseSensitive && c.Pattern == strings.ToLower(c.Pattern)) {
			pattern = "(?i)" + pattern
		}
		re, err := regexp.Compile(pattern)
		if err != nil {
			return nil, fmt.Errorf("invalid glob pattern: %w", err)
		}
		return func(target string) bool {
			return re.MatchString(target)
		}, nil
	}

	if c.FixedStrings {
		// Literal string match mode
		pattern := regexp.QuoteMeta(c.Pattern)
		if c.IgnoreCase || (!c.CaseSensitive && pattern == strings.ToLower(pattern)) {
			pattern = "(?i)" + pattern
		}
		re, err := regexp.Compile(pattern)
		if err != nil {
			return nil, fmt.Errorf("invalid pattern: %w", err)
		}
		return func(target string) bool {
			return re.MatchString(target)
		}, nil
	}

	// Regex mode (default)
	pattern := c.Pattern
	if !c.CaseSensitive && (c.IgnoreCase || pattern == strings.ToLower(pattern)) {
		pattern = "(?i)" + pattern
	}
	re, err := regexp.Compile(pattern)
	if err != nil {
		return nil, fmt.Errorf("invalid regex: %w", err)
	}
	return func(target string) bool {
		return re.MatchString(target)
	}, nil
}

func (c *SyncwebFindCmd) parseSizeConstraints() (sizeMin *int64, sizeMax *int64, err error) {
	if len(c.Size) > 0 {
		sizeRange, parseErr := utils.ParseRange(strings.Join(c.Size, ","), utils.HumanToBytes)
		if parseErr != nil {
			return nil, nil, fmt.Errorf("invalid size constraint: %w", parseErr)
		}
		sizeMin = sizeRange.Min
		sizeMax = sizeRange.Max
	}
	return sizeMin, sizeMax, nil
}

func (c *SyncwebFindCmd) parseTimeConstraints() (modifiedAfter *int64, modifiedBefore *int64, err error) {
	now := time.Now().Unix()

	if c.ModifiedWithin != "" {
		// Modified within duration (e.g., 1d, 2h, 30m)
		seconds, err := utils.HumanToSeconds(c.ModifiedWithin)
		if err != nil {
			return nil, nil, fmt.Errorf("invalid modified-within duration: %w", err)
		}
		ts := now - seconds
		modifiedAfter = &ts
	}

	if c.ModifiedBefore != "" {
		// Try parsing as duration first
		seconds, parseErr := utils.HumanToSeconds(c.ModifiedBefore)
		if parseErr == nil {
			ts := now - seconds
			modifiedBefore = &ts
		} else {
			// Try parsing as date
			ts := utils.ParseDateOrRelative(c.ModifiedBefore)
			if ts <= 0 {
				return nil, nil, fmt.Errorf("invalid modified-before: %s", c.ModifiedBefore)
			}
			modifiedBefore = &ts
		}
	}

	if len(c.TimeModified) > 0 {
		// Handle alternative time-modified syntax
		for _, tm := range c.TimeModified {
			// Check if it starts with + (older than) or - (newer than)
			if newerThan, ok := strings.CutPrefix(tm, "-"); ok {
				// Newer than (e.g., -3 days)
				duration := newerThan
				seconds, parseErr := utils.HumanToSeconds(duration)
				if parseErr != nil {
					return nil, nil, fmt.Errorf("invalid time-modified duration: %s", tm)
				}
				ts := now - seconds
				modifiedAfter = &ts
			} else if olderThan, ok2 := strings.CutPrefix(tm, "+"); ok2 {
				// Older than (e.g., +3 days)
				duration := olderThan
				seconds, parseErr := utils.HumanToSeconds(duration)
				if parseErr != nil {
					return nil, nil, fmt.Errorf("invalid time-modified duration: %s", tm)
				}
				ts := now - seconds
				modifiedBefore = &ts
			} else {
				// Try parsing as date or duration
				seconds, parseErr := utils.HumanToSeconds(tm)
				if parseErr == nil {
					ts := now - seconds
					modifiedAfter = &ts
				} else {
					ts := utils.ParseDateOrRelative(tm)
					if ts <= 0 {
						return nil, nil, fmt.Errorf("invalid time-modified: %s", tm)
					}
					modifiedAfter = &ts
				}
			}
		}
	}

	return modifiedAfter, modifiedBefore, nil
}

func (c *SyncwebFindCmd) parseDepthConstraints() (depthMin *int, depthMax *int) {
	if c.MinDepth > 0 {
		depthMin = &c.MinDepth
	}
	if c.MaxDepth > 0 {
		depthMax = &c.MaxDepth
	}
	if len(c.Depth) > 0 {
		for _, d := range c.Depth {
			// Try parsing as plain int first
			var val int
			_, scanErr := fmt.Sscanf(d, "%d", &val)
			if scanErr == nil {
				depthMin = &val
				depthMax = &val
			} else {
				// Try range parsing
				depthRange, rangeErr := utils.ParseRange(d, func(s string) (int64, error) {
					v, parseErr := strconv.ParseInt(s, 10, 64)
					return v, parseErr
				})
				if rangeErr == nil {
					if depthRange.Min != nil {
						v := int(*depthRange.Min)
						depthMin = &v
					}
					if depthRange.Max != nil {
						v := int(*depthRange.Max)
						depthMax = &v
					}
				}
			}
		}
	}
	return depthMin, depthMax
}

type findResult struct {
	Name     string    `json:"name"`
	Path     string    `json:"path"`
	Size     int64     `json:"size"`
	Modified time.Time `json:"modified"`
	IsDir    bool      `json:"is_dir"`
}

func (c *SyncwebFindCmd) processFile(
	meta any,
	matchFunc func(string) bool,
	sizeMin, sizeMax, modifiedAfterTS, modifiedBeforeTS *int64,
	depthMin, depthMax *int,
	f config.FolderConfiguration,
	g *SyncwebCmd,
) (findResult, bool) {
	isDir := getMetaType(meta) == protocol.FileInfoTypeDirectory

	// Type filter
	if c.Type == "f" && isDir {
		return findResult{}, false
	}
	if c.Type == "d" && !isDir {
		return findResult{}, false
	}

	// Hidden file filter
	if !c.Hidden && strings.HasPrefix(getMetaName(meta), ".") {
		return findResult{}, false
	}

	// Extension filter
	if len(c.Ext) > 0 {
		matched := false
		for _, ext := range c.Ext {
			if strings.HasSuffix(strings.ToLower(getMetaName(meta)), strings.ToLower(ext)) {
				matched = true
				break
			}
		}
		if !matched {
			return findResult{}, false
		}
	}

	// Size filter (files only)
	if !isDir && (sizeMin != nil || sizeMax != nil) {
		if sizeMin != nil && getMetaSize(meta) < *sizeMin {
			return findResult{}, false
		}
		if sizeMax != nil && getMetaSize(meta) > *sizeMax {
			return findResult{}, false
		}
	}

	// Time filter
	if modifiedAfterTS != nil || modifiedBeforeTS != nil {
		modifiedTS := getMetaModTime(meta).Unix()
		if modifiedAfterTS != nil && modifiedTS < *modifiedAfterTS {
			return findResult{}, false
		}
		if modifiedBeforeTS != nil && modifiedTS > *modifiedBeforeTS {
			return findResult{}, false
		}
	}

	// Depth filter
	if depthMin != nil || depthMax != nil {
		depth := strings.Count(getMetaName(meta), "/") + strings.Count(getMetaName(meta), "\\")
		if depthMin != nil && depth < *depthMin {
			return findResult{}, false
		}
		if depthMax != nil && depth > *depthMax {
			return findResult{}, false
		}
	}

	// Search target
	searchTarget := getMetaName(meta)
	if !c.FullPath {
		searchTarget = filepath.Base(getMetaName(meta))
	}

	if !matchFunc(searchTarget) {
		return findResult{}, false
	}

	// Build output path
	var path string
	if c.AbsolutePath {
		// Get folder path from config
		folderPath := f.Path
		path = filepath.Join(folderPath, getMetaName(meta))
	} else {
		path = fmt.Sprintf("sync://%s/%s", f.ID, getMetaName(meta))
	}

	if g.JSON {
		return findResult{
			Name:     filepath.Base(getMetaName(meta)),
			Path:     path,
			Size:     getMetaSize(meta),
			Modified: getMetaModTime(meta),
			IsDir:    isDir,
		}, true
	}

	fmt.Println(path)
	return findResult{}, false
}

// Helper functions to access db.FileMetadata fields
func getMetaName(meta any) string {
	if m, ok := meta.(interface{ Name() string }); ok {
		return m.Name()
	}
	return ""
}

func getMetaType(meta any) protocol.FileInfoType {
	if m, ok := meta.(interface{ Type() protocol.FileInfoType }); ok {
		return m.Type()
	}
	return protocol.FileInfoTypeFile
}

func getMetaSize(meta any) int64 {
	if m, ok := meta.(interface{ Size() int64 }); ok {
		return m.Size()
	}
	return 0
}

func getMetaModTime(meta any) time.Time {
	if m, ok := meta.(interface{ ModTime() time.Time }); ok {
		return m.ModTime()
	}
	return time.Time{}
}

// GlobToRegex converts a glob pattern to a regex pattern
func GlobToRegex(glob string) string {
	// Escape special regex characters except * and ?
	var result strings.Builder
	result.WriteString("^")
	for _, r := range glob {
		switch r {
		case '*':
			result.WriteString(".*")
		case '?':
			result.WriteString(".")
		case '.', '+', '^', '$', '(', ')', '[', ']', '{', '}', '|', '\\':
			result.WriteRune('\\')
			result.WriteRune(r)
		default:
			result.WriteRune(r)
		}
	}
	result.WriteString("$")
	return result.String()
}
