package commands

import (
	"fmt"
	"log/slog"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// SyncwebSortCmd sorts Syncthing files by multiple criteria
type SyncwebSortCmd struct {
	Paths          []string `arg:"" optional:"" help:"File paths to sort"`
	Sort           []string `help:"Sort criteria" default:"name"`
	LimitSize      string   `short:"S" help:"Stop after printing N bytes"`
	MinSeeders     int      `help:"Filter files with fewer than N seeders"`
	MaxSeeders     int      `help:"Filter files with more than N seeders"`
	Niche          int      `default:"3" help:"Ideal popularity for niche sort"`
	FrecencyWeight int      `default:"3" help:"Weight for frecency calculation"`
	Depth          []string `short:"d" help:"Depth constraints"`
	MinDepth       int      `help:"Minimum depth"`
	MaxDepth       int      `help:"Maximum depth"`
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
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		limitBytes := int64(0)
		if c.LimitSize != "" {
			limitBytes, _ = utils.HumanToBytes(c.LimitSize)
		}

		var files []fileWithInfo

		cfg := s.Node.Cfg.RawCopy()
		for _, p := range c.Paths {
			absPath, err := filepath.Abs(p)
			if err != nil {
				fmt.Printf("Error: %s: %v\n", p, err)
				continue
			}

			// Find folder
			var folderID string
			var relPath string

			for _, f := range cfg.Folders {
				if strings.HasPrefix(absPath, f.Path) {
					folderID = f.ID
					var err error
					relPath, err = filepath.Rel(f.Path, absPath)
					if err != nil {
						fmt.Printf("Error: Failed to compute relative path for %s: %v\n", p, err)
						continue
					}
					break
				}
			}

			if folderID == "" {
				continue
			}

			info, ok, err := s.GetGlobalFileInfo(folderID, relPath)
			if err == nil && ok {
				// Count seeders for this file
				seeders, err := s.CountSeeders(folderID, relPath)
				if err != nil {
					slog.Warn("Failed to count seeders", "path", relPath, "error", err)
					seeders = 0
				}

				files = append(files, fileWithInfo{
					Path:       p,
					Size:       info.Size,
					FolderID:   folderID,
					RelPath:    relPath,
					Seeders:    seeders,
					Modified:   info.ModifiedS,
					AccessTime: 0, // AccessTime is not available in protocol.FileInfo
				})
			}
		}

		// Apply seeder filters
		if c.MinSeeders > 0 {
			filtered := make([]fileWithInfo, 0, len(files))
			for _, f := range files {
				if f.Seeders >= c.MinSeeders {
					filtered = append(filtered, f)
				}
			}
			files = filtered
		}

		if c.MaxSeeders > 0 {
			filtered := make([]fileWithInfo, 0, len(files))
			for _, f := range files {
				if f.Seeders <= c.MaxSeeders {
					filtered = append(filtered, f)
				}
			}
			files = filtered
		}

		// Sort files
		sort.Slice(files, func(i, j int) bool {
			for _, criterion := range c.Sort {
				reverse := strings.HasPrefix(criterion, "-")
				if reverse {
					criterion = criterion[1:]
				}

				var less bool
				switch criterion {
				case "size":
					less = files[i].Size < files[j].Size
				case "name", "path":
					less = files[i].Path < files[j].Path
				case "seeds", "peers":
					less = files[i].Seeders < files[j].Seeders
				case "niche":
					// Niche score: closer to ideal = better
					nicheI := files[i].Seeders - c.Niche
					nicheJ := files[j].Seeders - c.Niche
					if nicheI < 0 {
						nicheI = -nicheI
					}
					if nicheJ < 0 {
						nicheJ = -nicheJ
					}
					less = nicheI < nicheJ
				case "frecency":
					// Frecency: combination of frequency (seeders) and recency
					frecencyI := calculateFrecency(files[i], c.FrecencyWeight)
					frecencyJ := calculateFrecency(files[j], c.FrecencyWeight)
					less = frecencyI < frecencyJ
				case "modified", "time":
					less = files[i].Modified < files[j].Modified
				default:
					continue
				}

				if reverse {
					return !less
				}
				return less
			}
			return false
		})

		currentSize := int64(0)
		for _, f := range files {
			if limitBytes > 0 && currentSize+f.Size > limitBytes {
				break
			}
			fmt.Println(f.Path)
			currentSize += f.Size
		}
		return nil
	})
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
func FrecencyScore(modified int64, seeders int, weight int) float64 {
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
