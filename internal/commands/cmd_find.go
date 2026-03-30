package commands

import (
	"encoding/json"
	"fmt"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"
)

// SyncwebFindCmd searches for files by filename, size, and modified date
type SyncwebFindCmd struct {
	Pattern        string   `arg:"" optional:"" default:".*" help:"Search patterns"`
	Type           string   `short:"t" help:"Filter by type (f=file, d=directory)"`
	FullPath       bool     `short:"p" help:"Search full path (default: filename only)"`
	IgnoreCase     bool     `short:"i" help:"Case insensitive search"`
	CaseSensitive  bool     `short:"s" help:"Case sensitive search"`
	FixedStrings   bool     `short:"F" help:"Treat all patterns as literals"`
	Glob           bool     `short:"g" help:"Glob-based search"`
	Exact          bool     `short:"x" help:"Exact match search"`
	Hidden         bool     `short:"H" help:"Search hidden files and directories"`
	AbsolutePath   bool     `short:"a" help:"Print absolute paths"`
	Downloadable   bool     `help:"Exclude sendonly folders"`
	Depth          []string `short:"d" help:"Depth constraints (e.g., +2, -3, 2)"`
	MinDepth       int      `help:"Minimum depth"`
	MaxDepth       int      `help:"Maximum depth"`
	Sizes          []string `short:"S" help:"Size constraints"`
	ModifiedWithin string   `help:"Modified within duration (e.g., 1d, 2h, 30m)"`
	ModifiedBefore string   `help:"Modified before duration or date (e.g., 1d, 2024-01-01)"`
	Ext            []string `short:"e" help:"File extensions to include"`
	Paths          []string `arg:"" optional:"" help:"Root directories to search"`
}

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
		if len(c.Sizes) > 0 {
			sizeRange, err := utils.ParseRange(strings.Join(c.Sizes, ","), utils.HumanToBytes)
			if err != nil {
				return fmt.Errorf("invalid size constraint: %w", err)
			}
			sizeMin = sizeRange.Min
			sizeMax = sizeRange.Max
		}

		// Parse time constraints
		var modifiedAfterTs, modifiedBeforeTs *int64
		now := time.Now().Unix()

		if c.ModifiedWithin != "" {
			// Modified within duration (e.g., 1d, 2h, 30m)
			seconds, err := utils.HumanToSeconds(c.ModifiedWithin)
			if err != nil {
				return fmt.Errorf("invalid modified-within duration: %w", err)
			}
			ts := now - seconds
			modifiedAfterTs = &ts
		}

		if c.ModifiedBefore != "" {
			// Try parsing as duration first
			seconds, err := utils.HumanToSeconds(c.ModifiedBefore)
			if err == nil {
				ts := now - seconds
				modifiedBeforeTs = &ts
			} else {
				// Try parsing as date
				ts := utils.ParseDateOrRelative(c.ModifiedBefore)
				if ts > 0 {
					modifiedBeforeTs = &ts
				} else {
					return fmt.Errorf("invalid modified-before: %s", c.ModifiedBefore)
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
				if modifiedAfterTs != nil || modifiedBeforeTs != nil {
					modifiedTs := meta.ModTime().Unix()
					if modifiedAfterTs != nil && modifiedTs < *modifiedAfterTs {
						continue
					}
					if modifiedBeforeTs != nil && modifiedTs > *modifiedBeforeTs {
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
						path = fmt.Sprintf("syncweb://%s/%s", f.ID, meta.Name)
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
			cancel()
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
