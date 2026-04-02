package commands

import (
	"fmt"
	"path/filepath"
	"slices"
	"strconv"
	"strings"

	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
)

// Stat command examples
const statExamples = `
Examples:
  # Show detailed file information
  syncweb stat file.txt

  # Terse output format
  syncweb stat -t file.txt

  # Custom format output
  syncweb stat -c '%n %s %y' file.txt    # name, size, mtime

  # Stat multiple files
  syncweb stat file1.txt file2.txt

  # Stat directory
  syncweb stat music/
`

// SyncwebStatCmd displays detailed file status information
type SyncwebStatCmd struct {
	Paths       []string `help:"Files or directories to stat"    required:"true" arg:""`
	Terse       bool     `help:"Print information in terse form"                        short:"t"`
	Format      string   `help:"Use custom format"                                      short:"c"`
	Dereference bool     `help:"Follow symbolic links"                                  short:"L"`
}

// Help displays examples for the stat command
func (c *SyncwebStatCmd) Help() string {
	return statExamples
}

func (c *SyncwebStatCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s syncweb.Engine) error {
		cfg := s.RawConfig()
		deviceNames := buildDeviceNamesMap(cfg.Devices)

		for _, p := range c.Paths {
			c.processPath(s, p, &cfg, deviceNames, g.JSON)
		}

		return nil
	})
}

// buildDeviceNamesMap builds a map of device ID to name
func buildDeviceNamesMap(devices []config.DeviceConfiguration) map[string]string {
	deviceNames := make(map[string]string)
	for i := range devices {
		d := &devices[i]
		name := d.Name
		if name == "" {
			name = d.DeviceID.String()[:7]
		}
		deviceNames[d.DeviceID.String()] = name
	}
	return deviceNames
}

// processPath processes a single path for stat command
func (c *SyncwebStatCmd) processPath(
	s syncweb.Engine,
	path string,
	cfg *config.Configuration,
	deviceNames map[string]string,
	_ bool,
) {
	absPath, err := filepath.Abs(path)
	if err != nil {
		fmt.Printf("Error: %s: %v\n", path, err)
		return
	}

	folderID, relPath, found := c.findFolderAndPath(absPath, cfg)
	if !found {
		fmt.Printf("Error: %s is not inside of a Syncweb folder\n", path)
		return
	}

	info, ok, err := s.GetGlobalFileInfo(folderID, relPath)
	if err != nil {
		fmt.Printf("Error: %v\n", err)
		return
	}
	if !ok {
		fmt.Printf("%s: Not found in cluster\n", path)
		return
	}

	availability := getDeviceAvailability(s, folderID, &info)
	c.printStatInfo(path, &info, availability, deviceNames)
}

// findFolderAndPath finds the folder ID and relative path for a given absolute path
func (c *SyncwebStatCmd) findFolderAndPath(
	absPath string,
	cfg *config.Configuration,
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

// printStatInfo prints file stat information in the appropriate format
func (c *SyncwebStatCmd) printStatInfo(
	path string,
	info *protocol.FileInfo,
	availability []string,
	deviceNames map[string]string,
) {
	if c.Terse {
		c.printTerseStat(info, availability)
	} else if c.Format != "" {
		c.printCustomStat(info)
	} else {
		c.printFullStat(path, info, availability, deviceNames)
	}
}

// printTerseStat prints terse format stat output
func (c *SyncwebStatCmd) printTerseStat(info *protocol.FileInfo, availability []string) {
	fmt.Printf("%s|%d|%d|%o|file|%d|%d|0\n",
		info.Name,
		info.Size,
		len(info.Blocks),
		info.Permissions,
		info.ModTime().Unix(),
		len(availability),
	)
}

// printCustomStat prints custom format stat output
func (c *SyncwebStatCmd) printCustomStat(info *protocol.FileInfo) {
	output := c.Format
	output = strings.ReplaceAll(output, "%n", info.Name)
	output = strings.ReplaceAll(output, "%s", strconv.FormatInt(info.Size, 10))
	output = strings.ReplaceAll(output, "%b", strconv.Itoa(len(info.Blocks)))
	output = strings.ReplaceAll(output, "%f", fmt.Sprintf("%o", info.Permissions))
	output = strings.ReplaceAll(output, "%y", info.ModTime().Format("2006-01-02 15:04:05"))
	fmt.Println(output)
}

// printFullStat prints full format stat output
func (c *SyncwebStatCmd) printFullStat(
	path string,
	info *protocol.FileInfo,
	availability []string,
	deviceNames map[string]string,
) {
	fileType := GetFileType(info)
	fmt.Printf("  Path: %s\n", path)
	fmt.Printf("  Size: %-15d Blocks: %-10d %s\n", info.Size, len(info.Blocks), fileType)

	deviceStr := c.formatDeviceAvailability(availability, deviceNames)
	versionStr := FormatVersion(info.Version)
	fmt.Printf("Device: %-15s Version: %s\n", deviceStr, versionStr)

	flags := c.buildFlagsList(info)
	flagStr := ""
	if len(flags) > 0 {
		flagStr = fmt.Sprintf(" [%s]", strings.Join(flags, ", "))
	}
	fmt.Printf("Access: (%o/---------)%s\n", info.Permissions, flagStr)
	fmt.Printf("Modify: %s\n", info.ModTime().Format("2006-01-02 15:04:05"))

	if info.ModifiedBy != 0 {
		c.printModifiedBy(info.ModifiedBy, deviceNames)
	}
}

// formatDeviceAvailability formats the device availability string
func (c *SyncwebStatCmd) formatDeviceAvailability(availability []string, deviceNames map[string]string) string {
	if len(availability) == 0 {
		return "none"
	}
	if len(availability) <= 3 {
		names := make([]string, 0, len(availability))
		for _, devID := range availability {
			if name, ok := deviceNames[devID]; ok {
				names = append(names, name)
			} else {
				names = append(names, devID[:7])
			}
		}
		return strings.Join(names, ", ")
	}
	return fmt.Sprintf("%d devices", len(availability))
}

// buildFlagsList builds a list of flags for the file
func (c *SyncwebStatCmd) buildFlagsList(info *protocol.FileInfo) []string {
	var flags []string
	if info.Deleted {
		flags = append(flags, "deleted")
	}
	if info.LocalFlags.IsInvalid() {
		flags = append(flags, "invalid")
	}
	if info.LocalFlags&protocol.FlagLocalIgnored != 0 {
		flags = append(flags, "ignored")
	}
	return flags
}

// printModifiedBy prints the modified by information
func (c *SyncwebStatCmd) printModifiedBy(modifiedBy protocol.ShortID, deviceNames map[string]string) {
	modifiedByID := modifiedBy.String()
	modifiedByName := deviceNames[modifiedByID]
	if modifiedByName == "" {
		modifiedByName = modifiedByID[:7]
	}
	fmt.Printf("Modified by: %s\n", modifiedByName)
}

// getDeviceAvailability returns a list of device IDs that have the file
func getDeviceAvailability(s syncweb.Engine, folderID string, info *protocol.FileInfo) []string {
	deviceSet := make(map[string]bool)
	for _, block := range info.Blocks {
		availables, err := s.BlockAvailability(folderID, info, block)
		if err != nil {
			continue
		}
		for _, av := range availables {
			deviceSet[av.ID.String()] = true
		}
	}

	// Also include the local device
	deviceSet[s.MyID().String()] = true

	devices := make([]string, 0, len(deviceSet))
	for devID := range deviceSet {
		devices = append(devices, devID)
	}
	slices.Sort(devices)
	return devices
}

// GetFileType returns a human-readable file type string
func GetFileType(info *protocol.FileInfo) string {
	switch info.Type {
	case protocol.FileInfoTypeDirectory:
		return "directory"
	case protocol.FileInfoTypeFile:
		return "regular file"
	case protocol.FileInfoTypeSymlink:
		return "symbolic link"
	default:
		return "unknown"
	}
}

// FormatVersion formats the version vector for display
func FormatVersion(version protocol.Vector) string {
	if len(version.Counters) == 0 {
		return "none"
	}

	var parts []string
	for _, c := range version.Counters {
		parts = append(parts, fmt.Sprintf("%d:%d", c.ID, c.Value))
	}

	if len(parts) <= 3 {
		return strings.Join(parts, ", ")
	}
	return fmt.Sprintf("%s, ... (%d total)", strings.Join(parts[:3], ", "), len(parts))
}
