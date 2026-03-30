package commands

import (
	"fmt"
	"math"
	"path/filepath"
	"slices"
	"strings"
	"syscall"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"
)

// SyncwebDownloadCmd marks file paths for download/sync
type SyncwebDownloadCmd struct {
	Paths []string `arg:"" optional:"" help:"File or directory paths to download"`
	Depth int      `help:"Maximum depth for directory traversal"`
}

type folderSpaceInfo struct {
	Free            int64
	Total           int64
	MinFree         int64
	Usable          int64
	MinFreeConfig   minDiskFreeConfig
	Mountpoint      string
	DeviceID        uint64
	PendingDownload int64
}

type minDiskFreeConfig struct {
	Value float64
	Unit  string
}

type downloadItem struct {
	folderID string
	relPath  string
	size     int64
}

func (c *SyncwebDownloadCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		// Build download plan
		var items []downloadItem
		var totalSize int64

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
					rel, _ := filepath.Rel(f.Path, absPath)
					relPath = rel
					break
				}
			}

			if folderID == "" {
				fmt.Printf("Warning: %s is not inside a Syncweb folder\n", p)
				continue
			}

			info, ok, err := s.GetGlobalFileInfo(folderID, relPath)
			if err != nil || !ok {
				fmt.Printf("Warning: %s not found in cluster\n", p)
				continue
			}

			items = append(items, downloadItem{folderID, relPath, info.Size})
			totalSize += info.Size
		}

		if len(items) == 0 {
			fmt.Println("No files found to download")
			return nil
		}

		// Group items by folder
		itemsByFolder := make(map[string][]downloadItem)
		for _, item := range items {
			itemsByFolder[item.folderID] = append(itemsByFolder[item.folderID], item)
		}

		// Get space info for each folder
		folderSpaceInfos := make(map[string]*folderSpaceInfo)
		for folderID := range itemsByFolder {
			spaceInfo := getFolderSpaceInfo(cfg, s, folderID)
			if spaceInfo != nil {
				folderSpaceInfos[folderID] = spaceInfo
			}
		}

		// Group folders by mountpoint
		mountpointGroups := groupFoldersByMountpoint(folderSpaceInfos)

		// Calculate mountpoint-level pending downloads and usable space
		mountpointUsage := calculateMountpointUsage(mountpointGroups, folderSpaceInfos, itemsByFolder)

		// Print summary
		printDownloadSummary(itemsByFolder, folderSpaceInfos, mountpointUsage, mountpointGroups)

		// Check for space warnings
		warnings := generateWarnings(mountpointUsage, folderSpaceInfos)
		if len(warnings) > 0 {
			fmt.Println("\nWARNING: Insufficient space!")
			for _, w := range warnings {
				fmt.Printf("  %s\n", w)
			}
			fmt.Println()
		}

		// Confirm
		if !g.NoConfirm && !g.Yes {
			var response string
			fmt.Printf("\nMark %d files (%s) for download? [y/N]: ", len(items), utils.FormatSize(totalSize))
			fmt.Scanln(&response)
			if strings.ToLower(response) != "y" && strings.ToLower(response) != "yes" {
				fmt.Println("Download cancelled")
				return nil
			}
		}

		// Trigger downloads
		for _, item := range items {
			if err := s.Unignore(item.folderID, item.relPath); err != nil {
				fmt.Printf("Error: Failed to trigger download for %s: %v\n", item.relPath, err)
			} else {
				fmt.Printf("Queued: %s\n", item.relPath)
			}
		}

		return nil
	})
}

// getFolderSpaceInfo gets disk space information for a folder
func getFolderSpaceInfo(cfg config.Configuration, s *syncweb.Syncweb, folderID string) *folderSpaceInfo {
	var folderPath string
	var minFreeCfg minDiskFreeConfig

	for _, f := range cfg.Folders {
		if f.ID == folderID {
			folderPath = f.Path
			minFreeCfg = minDiskFreeConfig{
				Value: f.MinDiskFree.Value,
				Unit:  f.MinDiskFree.Unit,
			}
			break
		}
	}

	if folderPath == "" {
		return nil
	}

	// Get disk space using syscall.Statfs
	var stat syscall.Statfs_t
	if err := syscall.Statfs(folderPath, &stat); err != nil {
		return nil
	}

	// Calculate free and total space with overflow protection
	// stat.Bavail = free blocks available to non-super user
	// stat.Blocks = total data blocks in filesystem
	// stat.Bsize = block size
	free := safeMulUint64(uint64(stat.Bavail), uint64(stat.Bsize))
	total := safeMulUint64(uint64(stat.Blocks), uint64(stat.Bsize))

	// Calculate minimum free space to preserve
	minFree := calculateMinDiskFree(total, minFreeCfg)

	// Get pending download size from NeedSize
	needSize, _ := s.Node.App.Internals.NeedSize(folderID, protocol.LocalDeviceID)
	pendingDownload := needSize.Bytes

	// Usable space = free - min_free - pending_downloads
	us := max(free-minFree-pendingDownload, 0)

	// Try to get mountpoint
	mountpoint := getMountpoint(folderPath)

	return &folderSpaceInfo{
		Free:            free,
		Total:           total,
		MinFree:         minFree,
		Usable:          us,
		MinFreeConfig:   minFreeCfg,
		Mountpoint:      mountpoint,
		PendingDownload: pendingDownload,
	}
}

// calculateMinDiskFree calculates the minimum free space to preserve
func calculateMinDiskFree(totalSpace int64, cfg minDiskFreeConfig) int64 {
	value := cfg.Value
	unit := strings.ToLower(cfg.Unit)

	if unit == "%" {
		return int64(float64(totalSpace) * value / 100.0)
	}

	multiplier := int64(1)
	switch unit {
	case "kb", "kib", "k":
		multiplier = 1024
	case "mb", "mib", "m":
		multiplier = 1024 * 1024
	case "gb", "gib", "g":
		multiplier = 1024 * 1024 * 1024
	case "tb", "tib", "t":
		multiplier = 1024 * 1024 * 1024 * 1024
	}

	return int64(value) * multiplier
}

// getMountpoint returns the mountpoint for a path
func getMountpoint(path string) string {
	// Simplified: just return the directory for now
	// A full implementation would check /proc/mounts or use stat.st_dev
	return filepath.Dir(path)
}

// groupFoldersByMountpoint groups folders by their mountpoint
func groupFoldersByMountpoint(folderSpaceInfos map[string]*folderSpaceInfo) map[string][]string {
	groups := make(map[string][]string)

	for folderID, info := range folderSpaceInfos {
		key := info.Mountpoint
		if key == "" {
			key = fmt.Sprintf("unknown_%s", folderID)
		}
		groups[key] = append(groups[key], folderID)
	}

	return groups
}

// mountpointUsageInfo holds calculated usage info for a mountpoint
type mountpointUsageInfo struct {
	TotalDownload int64
	Usable        int64
	Free          int64
	MaxMinFree    int64
	TotalPending  int64
	FolderIDs     []string
	Shared        bool
}

// calculateMountpointUsage calculates usage per mountpoint
func calculateMountpointUsage(
	mountpointGroups map[string][]string,
	folderSpaceInfos map[string]*folderSpaceInfo,
	itemsByFolder map[string][]downloadItem,
) map[string]*mountpointUsageInfo {
	result := make(map[string]*mountpointUsageInfo)

	for mountpoint, folderIDs := range mountpointGroups {
		var totalDownload int64
		var maxMinFree int64
		var totalPending int64
		var free int64

		for _, fid := range folderIDs {
			info := folderSpaceInfos[fid]
			if info == nil {
				continue
			}

			// Sum up download sizes for this folder
			for _, item := range itemsByFolder[fid] {
				totalDownload += item.size
			}

			// Track max min_free config
			if info.MinFree > maxMinFree {
				maxMinFree = info.MinFree
			}

			// Sum pending downloads
			totalPending += info.PendingDownload

			// Use first folder's free space (they share mountpoint)
			if free == 0 {
				free = info.Free
			}
		}

		// Usable = free - max_buffer - pending
		us := max(free-maxMinFree-totalPending, 0)

		result[mountpoint] = &mountpointUsageInfo{
			TotalDownload: totalDownload,
			Usable:        us,
			Free:          free,
			MaxMinFree:    maxMinFree,
			TotalPending:  totalPending,
			FolderIDs:     folderIDs,
			Shared:        len(folderIDs) > 1,
		}
	}

	return result
}

// printDownloadSummary prints the download summary table
func printDownloadSummary(
	itemsByFolder map[string][]downloadItem,
	folderSpaceInfos map[string]*folderSpaceInfo,
	mountpointUsage map[string]*mountpointUsageInfo,
	mountpointGroups map[string][]string,
) {
	fmt.Println("\nDownload Summary:")
	fmt.Println(strings.Repeat("-", 135))
	fmt.Printf("%-40s %8s %12s %12s %12s %15s %8s\n",
		"Folder ID", "Files", "Total Size", "Usable", "Pending", "Buffer", "Status")
	fmt.Println(strings.Repeat("-", 135))

	var grandTotalSize int64
	var grandTotalFiles int

	for folderID, items := range itemsByFolder {
		count := len(items)
		size := int64(0)
		for _, item := range items {
			size += item.size
		}
		grandTotalSize += size
		grandTotalFiles += count

		spaceInfo := folderSpaceInfos[folderID]
		status := "?"
		usableStr := "Unknown"
		pendingStr := "-"
		bufferStr := "Unknown"

		if spaceInfo != nil {
			// Find mountpoint usage for this folder
			var mpInfo *mountpointUsageInfo
			for mp, info := range mountpointUsage {
				if slices.Contains(info.FolderIDs, folderID) {
					mpInfo = info
					_ = mp
					break
				}
			}

			if mpInfo != nil {
				usableStr = utils.FormatSize(mpInfo.Usable)
				pendingStr = utils.FormatSize(spaceInfo.PendingDownload)
				if spaceInfo.PendingDownload == 0 {
					pendingStr = "-"
				}
				bufferStr = fmt.Sprintf("%s (%.0f%s)",
					utils.FormatSize(spaceInfo.MinFree),
					spaceInfo.MinFreeConfig.Value,
					spaceInfo.MinFreeConfig.Unit)

				if mpInfo.TotalDownload > mpInfo.Usable {
					status = "LOW"
				} else {
					status = "OK"
				}
			}
		}

		fmt.Printf("%-40s %8d %12s %12s %12s %15s %8s\n",
			folderID, count, utils.FormatSize(size), usableStr, pendingStr, bufferStr, status)
	}

	fmt.Println(strings.Repeat("-", 135))
	fmt.Printf("%-40s %8d %12s\n", "TOTAL", grandTotalFiles, utils.FormatSize(grandTotalSize))
	fmt.Println(strings.Repeat("-", 135))

	// Print shared mountpoint summary
	for mp, info := range mountpointUsage {
		if info.Shared {
			fmt.Printf("\nShared Mountpoint (%s):\n", mp)
			fmt.Printf("  Folders: %s\n", strings.Join(info.FolderIDs, ", "))
			fmt.Printf("  Combined download: %s\n", utils.FormatSize(info.TotalDownload))
			fmt.Printf("  Pending downloads: %s\n", utils.FormatSize(info.TotalPending))
			fmt.Printf("  Usable space: %s (after %s buffer and %s pending)\n",
				utils.FormatSize(info.Usable),
				utils.FormatSize(info.MaxMinFree),
				utils.FormatSize(info.TotalPending))
		}
	}
}

// generateWarnings generates warnings for insufficient space
func generateWarnings(
	mountpointUsage map[string]*mountpointUsageInfo,
	folderSpaceInfos map[string]*folderSpaceInfo,
) []string {
	var warnings []string

	for mp, info := range mountpointUsage {
		if info.TotalDownload > info.Usable {
			if info.Shared {
				folderList := strings.Join(info.FolderIDs[:3], ", ")
				if len(info.FolderIDs) > 3 {
					folderList += fmt.Sprintf(", ... (%d total)", len(info.FolderIDs))
				}
				warnings = append(warnings,
					fmt.Sprintf("Shared mountpoint (%s): Combined download size (%s) exceeds usable space (%s) across folders: %s",
						mp, utils.FormatSize(info.TotalDownload), utils.FormatSize(info.Usable), folderList))
			} else {
				folderID := info.FolderIDs[0]
				spaceInfo := folderSpaceInfos[folderID]
				bufferDesc := fmt.Sprintf("%.0f%s", spaceInfo.MinFreeConfig.Value, spaceInfo.MinFreeConfig.Unit)
				warnings = append(warnings,
					fmt.Sprintf("Folder %s: Download size (%s) exceeds usable space (%s) [preserving %s buffer (%s)]",
						folderID, utils.FormatSize(info.TotalDownload), utils.FormatSize(info.Usable),
						utils.FormatSize(spaceInfo.MinFree), bufferDesc))
			}
		}
	}

	return warnings
}

// safeMulUint64 multiplies two uint64 values with overflow protection
// Returns math.MaxInt64 if overflow would occur
func safeMulUint64(a, b uint64) int64 {
	if a == 0 || b == 0 {
		return 0
	}
	// Check for overflow: a * b > MaxInt64
	if a > math.MaxInt64/b {
		return math.MaxInt64
	}
	result := a * b
	if result > math.MaxInt64 {
		return math.MaxInt64
	}
	return int64(result)
}
