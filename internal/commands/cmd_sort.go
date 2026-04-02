package commands

import (
	"fmt"
	"log/slog"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/syncthing/syncthing/lib/config"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// Sort command examples
const sortExamples = `
Examples:
  # Sort by multiple criteria
  syncweb sort "balanced,frecency" < files.txt
  syncweb sort --sort=date,-seeds < files.txt    # Old and popular first
  syncweb sort --sort=-size,name < files.txt     # Large to small, then by name

  # Filter by seeders
  syncweb sort --min-seeders=2 < files.txt       # Only files with 2+ seeders
  syncweb sort --max-seeders=5 < files.txt       # Only files with ≤5 seeders

  # Niche sorting (files closer to ideal peer count rank higher)
  syncweb sort --niche=3 < files.txt             # Ideal is 3 peers

  # Frecency sorting (recent + popular)
  syncweb sort --frecency-weight=3 < files.txt   # Lower weight = more recency
`

// SyncwebSortCmd sorts Syncthing files by multiple criteria
type SyncwebSortCmd struct {
	Paths          []string `help:"File paths to sort (or read from stdin)"               arg:"" optional:""`
	Sort           []string `help:"Sort by: name, size, seeds, niche, frecency, modified"                    default:"name"`
	LimitSize      string   `help:"Stop after printing N bytes"                                                             short:"S"`
	MinSeeders     int      `help:"Filter files with fewer than N seeders"`
	MaxSeeders     int      `help:"Filter files with more than N seeders"`
	Niche          int      `help:"Ideal peer count for niche sorting"                                       default:"3"`
	FrecencyWeight int      `help:"Recency weight for frecency (lower=more recency)"                         default:"3"`
	Depth          []string `help:"Depth constraints"                                                                       short:"d"`
	MinDepth       int      `help:"Minimum depth"`
	MaxDepth       int      `help:"Maximum depth"`
}

// Help displays examples for the sort command
func (c *SyncwebSortCmd) Help() string {
	return sortExamples
}

type fileWithInfo struct {
	Path       string
	Size       int64
	FolderID   string
	RelPath    string
	Seeders    int
	Modified   int64
	AccessTime int64
}

func (c *SyncwebSortCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s syncweb.Engine) error {
		limitBytes := int64(0)
		if c.LimitSize != "" {
			limitBytes, _ = utils.HumanToBytes(c.LimitSize)
		}

		files := c.collectFiles(s)
		files = c.applySeederFilters(files)
		files = c.sortFiles(files)
		c.printFiles(files, limitBytes)

		return nil
	})
}

// collectFiles collects files from syncweb folders
func (c *SyncwebSortCmd) collectFiles(s syncweb.Engine) []fileWithInfo {
	cfg := s.RawConfig()
	files := make([]fileWithInfo, 0, len(c.Paths))

	for _, p := range c.Paths {
		files = append(files, c.collectFilesForPath(s, cfg, p)...)
	}

	return files
}

// collectFilesForPath collects files for a single path
func (c *SyncwebSortCmd) collectFilesForPath(s syncweb.Engine, cfg config.Configuration, path string) []fileWithInfo {
	absPath, err := filepath.Abs(path)
	if err != nil {
		fmt.Printf("Error: %s: %v\n", path, err)
		return nil
	}

	folderID, relPath, found := c.findFolderAndPath(absPath, cfg)
	if !found {
		return nil
	}

	info, ok, err := s.GetGlobalFileInfo(folderID, relPath)
	if err != nil || !ok {
		return nil
	}

	seeders, err := s.CountSeeders(folderID, relPath)
	if err != nil {
		logger := slog.Default()
		logger.Warn("Failed to count seeders", "path", relPath, "error", err)
		seeders = 0
	}

	return []fileWithInfo{{
		Path:       path,
		Size:       info.Size,
		FolderID:   folderID,
		RelPath:    relPath,
		Seeders:    seeders,
		Modified:   info.ModifiedS,
		AccessTime: 0,
	}}
}

// findFolderAndPath finds the folder ID and relative path for a given absolute path
func (c *SyncwebSortCmd) findFolderAndPath(
	absPath string,
	cfg config.Configuration,
) (folderID, relPath string, found bool) {
	for _, f := range cfg.Folders {
		if strings.HasPrefix(absPath, f.Path) {
			folderID = f.ID
			relPath, relErr := filepath.Rel(f.Path, absPath)
			if relErr != nil {
				return "", "", false
			}
			return folderID, relPath, true
		}
	}
	return "", "", false
}

// applySeederFilters applies seeder count filters
func (c *SyncwebSortCmd) applySeederFilters(files []fileWithInfo) []fileWithInfo {
	if c.MinSeeders > 0 {
		files = filterMinSeeders(files, c.MinSeeders)
	}
	if c.MaxSeeders > 0 {
		files = filterMaxSeeders(files, c.MaxSeeders)
	}
	return files
}

// filterMinSeeders filters files with fewer than minSeeders seeders
func filterMinSeeders(files []fileWithInfo, minSeeders int) []fileWithInfo {
	filtered := make([]fileWithInfo, 0, len(files))
	for _, f := range files {
		if f.Seeders >= minSeeders {
			filtered = append(filtered, f)
		}
	}
	return filtered
}

// filterMaxSeeders filters files with more than maxSeeders seeders
func filterMaxSeeders(files []fileWithInfo, maxSeeders int) []fileWithInfo {
	filtered := make([]fileWithInfo, 0, len(files))
	for _, f := range files {
		if f.Seeders <= maxSeeders {
			filtered = append(filtered, f)
		}
	}
	return filtered
}

// sortFiles sorts files according to sort criteria
func (c *SyncwebSortCmd) sortFiles(files []fileWithInfo) []fileWithInfo {
	sort.Slice(files, func(i, j int) bool {
		return c.compareFiles(files[i], files[j])
	})
	return files
}

// compareFiles compares two files based on sort criteria
func (c *SyncwebSortCmd) compareFiles(a, b fileWithInfo) bool {
	for _, criterion := range c.Sort {
		reverse := strings.HasPrefix(criterion, "-")
		if reverse {
			criterion = criterion[1:]
		}

		less := c.compareByCriterion(a, b, criterion)

		if reverse {
			return !less
		}
		if less {
			return true
		}
	}
	return false
}

// compareByCriterion compares two files by a single criterion
func (c *SyncwebSortCmd) compareByCriterion(a, b fileWithInfo, criterion string) bool {
	switch criterion {
	case "size":
		return a.Size < b.Size
	case "name", "path":
		return a.Path < b.Path
	case "seeds", "peers":
		return a.Seeders < b.Seeders
	case "niche":
		nicheA := abs(a.Seeders - c.Niche)
		nicheB := abs(b.Seeders - c.Niche)
		return nicheA < nicheB
	case "frecency":
		frecencyA := calculateFrecency(a, c.FrecencyWeight)
		frecencyB := calculateFrecency(b, c.FrecencyWeight)
		return frecencyA < frecencyB
	case "modified", "time":
		return a.Modified < b.Modified
	}
	return false
}

// abs returns the absolute value of an integer
func abs(x int) int {
	if x < 0 {
		return -x
	}
	return x
}

// printFiles prints the sorted files
func (c *SyncwebSortCmd) printFiles(files []fileWithInfo, limitBytes int64) {
	currentSize := int64(0)
	for _, f := range files {
		if limitBytes > 0 && currentSize+f.Size > limitBytes {
			break
		}
		fmt.Println(f.Path)
		currentSize += f.Size
	}
}

// calculateFrecency computes a score based on recency and popularity (seed count)
// Higher score = more recently modified and/or more popular (more seeders)
func calculateFrecency(f fileWithInfo, weight int) float64 {
	now := time.Now().Unix()

	// Recency component: more recent = higher score
	age := float64(now - f.Modified)
	recencyScore := 1.0 / (1.0 + age/86400) // Decay over days

	// Frequency/popularity component: more seeders = higher score
	// Normalize seed count to avoid overflow and give diminishing returns
	freqScore := float64(f.Seeders) / (1.0 + float64(f.Seeders))

	// Combine recency and frequency with weight balancing them
	// Higher weight gives more importance to recency
	return recencyScore*float64(weight) + freqScore
}

// FrecencyScore calculates the popularity score for a file using recency and seed count
// modified: Unix timestamp of last modification
// seeders: number of devices that have the file (popularity measure)
// weight: importance of recency vs frequency (higher = more weight on recency)
func FrecencyScore(modified int64, seeders, weight int) float64 {
	now := time.Now().Unix()

	age := float64(now - modified)
	recencyScore := 1.0 / (1.0 + age/86400)

	// Frequency/popularity component: more seeders = higher score
	freqScore := float64(seeders) / (1.0 + float64(seeders))

	return recencyScore*float64(weight) + freqScore
}

// NicheScore calculates how close a file is to the ideal seeder count
func NicheScore(seeders, ideal int) int {
	diff := seeders - ideal
	if diff < 0 {
		diff = -diff
	}
	return diff
}
