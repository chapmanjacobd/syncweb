package commands

import (
	"fmt"
	"path/filepath"
	"slices"
	"strconv"
	"strings"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/syncthing/syncthing/lib/protocol"
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
	Paths       []string `help:"Files or directories to stat" required:"" arg:""`
	Terse       bool     `help:"Print information in terse form" short:"t"`
	Format      string   `help:"Use custom format"               short:"c"`
	Dereference bool     `help:"Follow symbolic links"           short:"L"`
}

// Help displays examples for the stat command
func (c *SyncwebStatCmd) Help() string {
	return statExamples
}

func (c *SyncwebStatCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		cfg := s.Node.Cfg.RawCopy()

		// Build device ID to name map
		deviceNames := make(map[string]string)
		for _, d := range cfg.Devices {
			name := d.Name
			if name == "" {
				name = d.DeviceID.String()[:7]
			}
			deviceNames[d.DeviceID.String()] = name
		}

		for _, p := range c.Paths {
			absPath, err := filepath.Abs(p)
			if err != nil {
				fmt.Printf("Error: %s: %v\n", p, err)
				continue
			}

			// Find folder and relative path
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
				fmt.Printf("Error: %s is not inside of a Syncweb folder\n", p)
				continue
			}

			info, ok, err := s.GetGlobalFileInfo(folderID, relPath)
			if err != nil {
				fmt.Printf("Error: %v\n", err)
				continue
			}
			if !ok {
				fmt.Printf("%s: Not found in cluster\n", p)
				continue
			}

			// Get device availability
			availability := getDeviceAvailability(s, folderID, info)

			if c.Terse {
				// Terse format: name|size|blocks|permissions|type|modified|device_count|has_diffs
				fmt.Printf("%s|%d|%d|%o|file|%d|%d|0\n",
					info.Name,
					info.Size,
					len(info.Blocks),
					info.Permissions,
					info.ModTime().Unix(),
					len(availability),
				)
			} else if c.Format != "" {
				// Custom format
				output := c.Format
				output = strings.ReplaceAll(output, "%n", info.Name)
				output = strings.ReplaceAll(output, "%s", strconv.FormatInt(info.Size, 10))
				output = strings.ReplaceAll(output, "%b", strconv.Itoa(len(info.Blocks)))
				output = strings.ReplaceAll(output, "%f", fmt.Sprintf("%o", info.Permissions))
				output = strings.ReplaceAll(output, "%y", info.ModTime().Format("2006-01-02 15:04:05"))
				fmt.Println(output)
			} else {
				// Full format
				fileType := getFileType(info)
				fmt.Printf("  Path: %s\n", p)
				fmt.Printf("  Size: %-15d Blocks: %-10d %s\n", info.Size, len(info.Blocks), fileType)

				// Device availability
				var deviceStr string
				if len(availability) == 0 {
					deviceStr = "none"
				} else if len(availability) <= 3 {
					names := make([]string, 0, len(availability))
					for _, devID := range availability {
						if name, ok := deviceNames[devID]; ok {
							names = append(names, name)
						} else {
							names = append(names, devID[:7])
						}
					}
					deviceStr = strings.Join(names, ", ")
				} else {
					deviceStr = fmt.Sprintf("%d devices", len(availability))
				}

				// Version vector display
				versionStr := formatVersion(info.Version)

				fmt.Printf("Device: %-15s Version: %s\n", deviceStr, versionStr)

				// Flags display
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
				flagStr := ""
				if len(flags) > 0 {
					flagStr = fmt.Sprintf(" [%s]", strings.Join(flags, ", "))
				}

				fmt.Printf("Access: (%o/---------)%s\n", info.Permissions, flagStr)

				// Timestamps
				fmt.Printf("Modify: %s\n", info.ModTime().Format("2006-01-02 15:04:05"))

				// Modified by tracking
				if info.ModifiedBy != 0 {
					modifiedByID := info.ModifiedBy.String()
					modifiedByName := deviceNames[modifiedByID]
					if modifiedByName == "" {
						modifiedByName = modifiedByID[:7]
					}
					fmt.Printf("Modified by: %s\n", modifiedByName)
				}
			}
		}
		return nil
	})
}

// getDeviceAvailability returns a list of device IDs that have the file
func getDeviceAvailability(s *syncweb.Syncweb, folderID string, info protocol.FileInfo) []string {
	deviceSet := make(map[string]bool)
	for _, block := range info.Blocks {
		availables, err := s.Node.App.Internals.BlockAvailability(folderID, info, block)
		if err != nil {
			continue
		}
		for _, av := range availables {
			deviceSet[av.ID.String()] = true
		}
	}

	// Also include the local device
	deviceSet[s.Node.MyID().String()] = true

	devices := make([]string, 0, len(deviceSet))
	for devID := range deviceSet {
		devices = append(devices, devID)
	}
	slices.Sort(devices)
	return devices
}

// getFileType returns a human-readable file type string
func getFileType(info protocol.FileInfo) string {
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

// formatVersion formats the version vector for display
func formatVersion(version protocol.Vector) string {
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
