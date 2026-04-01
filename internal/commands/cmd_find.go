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

//nolint:maintidx // CLI command with many flags is inherently complex
func (c *SyncwebFindCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		type findResult struct {
			Name     string    `json:"name"`
			Path     string    `json:"path"`
			Size     int64     `json:"size"`
			Modified time.Time `json:"modified"`
			IsDir    bool      `json:"is_dir"`
		}
		results := []findResult{}

		// Build search pattern based on mode
		var matchFunc func(string) bool

		if c.Exact {
			// Exact match mode
			pattern := c.Pattern
			if c.IgnoreCase || (!c.CaseSensitive && pattern == strings.ToLower(pattern)) {
				matchFunc = func(target string) bool {
					return strings.EqualFold(target, pattern)
				}
			} else {
				matchFunc = func(target string) bool {
					return target == pattern
				}
			}
		} else if c.Glob {
			// Glob match mode
			pattern := globToRegex(c.Pattern)
			if c.IgnoreCase || (!c.CaseSensitive && c.Pattern == strings.ToLower(c.Pattern)) {
				pattern = "(?i)" + pattern
			}
			re, err := regexp.Compile(pattern)
			if err != nil {
				return fmt.Errorf("invalid glob pattern: %w", err)
			}
			matchFunc = func(target string) bool {
				return re.MatchString(target)
			}
		} else if c.FixedStrings {
			// Literal string match mode
			pattern := regexp.QuoteMeta(c.Pattern)
			if c.IgnoreCase || (!c.CaseSensitive && pattern == strings.ToLower(pattern)) {
				pattern = "(?i)" + pattern
			}
			re, err := regexp.Compile(pattern)
			if err != nil {
				return fmt.Errorf("invalid pattern: %w", err)
			}
			matchFunc = func(target string) bool {
				return re.MatchString(target)
			}
		} else {
			// Regex mode (default)
			pattern := c.Pattern
			if !c.CaseSensitive && (c.IgnoreCase || pattern == strings.ToLower(pattern)) {
				pattern = "(?i)" + pattern
			}
			re, err := regexp.Compile(pattern)
			if err != nil {
				return fmt.Errorf("invalid regex: %w", err)
			}
			matchFunc = func(target string) bool {
				return re.MatchString(target)
			}
		}

		// Parse size constraints
		var sizeMin, sizeMax *int64
		if len(c.Size) > 0 {
			sizeRange, err := utils.ParseRange(strings.Join(c.Size, ","), utils.HumanToBytes)
			if err != nil {
				return fmt.Errorf("invalid size constraint: %w", err)
			}
			sizeMin = sizeRange.Min
			sizeMax = sizeRange.Max
		}

		// Parse time constraints
		var modifiedAfterTS, modifiedBeforeTS *int64
		now := time.Now().Unix()

		if c.ModifiedWithin != "" {
			// Modified within duration (e.g., 1d, 2h, 30m)
			seconds, err := utils.HumanToSeconds(c.ModifiedWithin)
			if err != nil {
				return fmt.Errorf("invalid modified-within duration: %w", err)
			}
			ts := now - seconds
			modifiedAfterTS = &ts
		}

		if c.ModifiedBefore != "" {
			// Try parsing as duration first
			seconds, err := utils.HumanToSeconds(c.ModifiedBefore)
			if err == nil {
				ts := now - seconds
				modifiedBeforeTS = &ts
			} else {
				// Try parsing as date
				ts := utils.ParseDateOrRelative(c.ModifiedBefore)
				if ts > 0 {
					modifiedBeforeTS = &ts
				} else {
					return fmt.Errorf("invalid modified-before: %s", c.ModifiedBefore)
				}
			}
		}

		if len(c.TimeModified) > 0 {
			// Handle alternative time-modified syntax
			for _, tm := range c.TimeModified {
				// Check if it starts with + (older than) or - (newer than)
				if after, ok := strings.CutPrefix(tm, "-"); ok {
					// Newer than (e.g., -3 days)
					duration := after
					seconds, err := utils.HumanToSeconds(duration)
					if err != nil {
						return fmt.Errorf("invalid time-modified duration: %s", tm)
					}
					ts := now - seconds
					modifiedAfterTS = &ts
				} else if after, ok := strings.CutPrefix(tm, "+"); ok {
					// Older than (e.g., +3 days)
					duration := after
					seconds, err := utils.HumanToSeconds(duration)
					if err != nil {
						return fmt.Errorf("invalid time-modified duration: %s", tm)
					}
					ts := now - seconds
					modifiedBeforeTS = &ts
				} else {
					// Try parsing as date or duration
					seconds, err := utils.HumanToSeconds(tm)
					if err == nil {
						ts := now - seconds
						modifiedAfterTS = &ts
					} else {
						ts := utils.ParseDateOrRelative(tm)
						if ts > 0 {
							modifiedAfterTS = &ts
						} else {
							return fmt.Errorf("invalid time-modified: %s", tm)
						}
					}
				}
			}
		}

		// Parse depth constraints
		var depthMin, depthMax *int
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
				_, err := fmt.Sscanf(d, "%d", &val)
				if err == nil {
					depthMin = &val
					depthMax = &val
				} else {
					// Try range parsing
					depthRange, err := utils.ParseRange(d, func(s string) (int64, error) {
						v, err := strconv.ParseInt(s, 10, 64)
						return v, err
					})
					if err == nil {
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

				// Size filter (files only)
				if !isDir && (sizeMin != nil || sizeMax != nil) {
					if sizeMin != nil && meta.Size < *sizeMin {
						continue
					}
					if sizeMax != nil && meta.Size > *sizeMax {
						continue
					}
				}

				// Time filter
				if modifiedAfterTS != nil || modifiedBeforeTS != nil {
					modifiedTS := meta.ModTime().Unix()
					if modifiedAfterTS != nil && modifiedTS < *modifiedAfterTS {
						continue
					}
					if modifiedBeforeTS != nil && modifiedTS > *modifiedBeforeTS {
						continue
					}
				}

				// Depth filter
				if depthMin != nil || depthMax != nil {
					depth := strings.Count(meta.Name, "/") + strings.Count(meta.Name, "\\")
					if depthMin != nil && depth < *depthMin {
						continue
					}
					if depthMax != nil && depth > *depthMax {
						continue
					}
				}

				// Search target
				searchTarget := meta.Name
				if !c.FullPath {
					searchTarget = filepath.Base(meta.Name)
				}

				if matchFunc(searchTarget) {
					// Build output path
					var path string
					if c.AbsolutePath {
						// Get folder path from config
						folderPath := f.Path
						path = filepath.Join(folderPath, meta.Name)
					} else {
						path = fmt.Sprintf("sync://%s/%s", f.ID, meta.Name)
					}

					if g.JSON {
						results = append(results, findResult{
							Name:     filepath.Base(meta.Name),
							Path:     path,
							Size:     meta.Size,
							Modified: meta.ModTime(),
							IsDir:    isDir,
						})
					} else {
						fmt.Println(path)
					}
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

// globToRegex converts a glob pattern to a regex pattern
func globToRegex(glob string) string {
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
