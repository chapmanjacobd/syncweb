package commands

import (
	"fmt"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// SyncwebSortCmd sorts Syncthing files by multiple criteria
type SyncwebSortCmd struct {
	Paths        []string `arg:"" optional:"" help:"File paths to sort"`
	Sort         []string `help:"Sort criteria" default:"name"`
	LimitSize    string   `short:"S" help:"Stop after printing N bytes"`
	MinSeeders   int      `help:"Filter files with fewer than N seeders"`
	MaxSeeders   int      `help:"Filter files with more than N seeders"`
	Niche        int      `default:"3" help:"Ideal popularity for niche sort"`
	FrecencyWeight int    `default:"3" help:"Weight for frecency calculation"`
	Depth        []string `short:"d" help:"Depth constraints"`
	MinDepth     int      `help:"Minimum depth"`
	MaxDepth     int      `help:"Maximum depth"`
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
				continue
			}

			info, ok, err := s.GetGlobalFileInfo(folderID, relPath)
			if err == nil && ok {
				// Count seeders for this file
				seeders, _ := s.CountSeeders(folderID, relPath)

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
					// Frecency: combination of frequency and recency
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

// calculateFrecency computes a score based on recency and access frequency
// Higher score = more recently/frequently accessed
// Note: Since AccessTime is not available, we use only recency
func calculateFrecency(f fileWithInfo, weight int) float64 {
	now := time.Now().Unix()
	
	// Recency component: more recent = higher score
	age := float64(now - f.Modified)
	recencyScore := 1.0 / (1.0 + age/86400) // Decay over days

	// Without access time, we can only use recency
	// Weight affects how much recency matters
	return recencyScore * float64(weight)
}

// FrecencyScore calculates the popularity score for a file
func FrecencyScore(modified, accessTime int64, weight int) float64 {
	now := time.Now().Unix()
	
	age := float64(now - modified)
	recencyScore := 1.0 / (1.0 + age/86400)
	
	freqScore := float64(accessTime) / float64(now)
	
	return recencyScore * float64(weight) + freqScore
}

// NicheScore calculates how close a file is to the ideal seeder count
func NicheScore(seeders, ideal int) int {
	diff := seeders - ideal
	if diff < 0 {
		diff = -diff
	}
	return diff
}
