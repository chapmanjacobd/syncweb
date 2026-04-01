package utils

import (
	"fmt"
	"net/url"
	"path/filepath"
	"regexp"
	"strings"
	"time"
)

// SyncwebRef represents a parsed sync:// URL
type SyncwebRef struct {
	FolderID string
	Subpath  string
	DeviceID string
}

// ParseSyncwebPath parses a sync:// or syncweb:// URL into its components
// Format: sync://folder-id/subpath#device-id
func ParseSyncwebPath(rawURL string, decode bool) (*SyncwebRef, error) {
	var trimmed string
	if after, ok := strings.CutPrefix(rawURL, "sync://"); ok {
		trimmed = after
	} else if after, ok := strings.CutPrefix(rawURL, "syncweb://"); ok {
		trimmed = after
	} else {
		return nil, fmt.Errorf("invalid sync URL: %s", rawURL)
	}

	// Split by # to separate device ID
	var deviceID string
	mainPart := trimmed
	if before, after, ok := strings.Cut(trimmed, "#"); ok {
		mainPart = before
		deviceID = after
		if decode && deviceID != "" {
			var err error
			deviceID, err = url.PathUnescape(deviceID)
			if err != nil {
				return nil, fmt.Errorf("failed to decode device ID: %w", err)
			}
		}
	}

	// Split by / to separate folder ID and subpath
	var folderID, subpath string
	if before, after, ok := strings.Cut(mainPart, "/"); ok {
		folderID = before
		subpath = after
	} else {
		folderID = mainPart
	}

	if decode && folderID != "" {
		var err error
		folderID, err = url.PathUnescape(folderID)
		if err != nil {
			return nil, fmt.Errorf("failed to decode folder ID: %w", err)
		}
	}
	if decode && subpath != "" {
		var err error
		subpath, err = url.PathUnescape(subpath)
		if err != nil {
			return nil, fmt.Errorf("failed to decode subpath: %w", err)
		}
	}

	return &SyncwebRef{
		FolderID: folderID,
		Subpath:  subpath,
		DeviceID: deviceID,
	}, nil
}

// ExtractDeviceID extracts a device ID from a string
// Handles both full and short device IDs
func ExtractDeviceID(s string) (string, error) {
	s = strings.TrimSpace(s)

	// Remove any sync:// or syncweb:// prefix
	if strings.HasPrefix(s, "sync://") || strings.HasPrefix(s, "syncweb://") {
		ref, err := ParseSyncwebPath(s, true)
		if err != nil {
			return "", err
		}
		if ref.DeviceID != "" {
			return ref.DeviceID, nil
		}
	}

	// Check if it's already a valid device ID format (XXVXXXXX-...)
	deviceIDPattern := regexp.MustCompile(`^[A-Z0-9]{7}(-[A-Z0-9]{7}){7}$`)
	if deviceIDPattern.MatchString(s) {
		return s, nil
	}

	// Try to find a device ID pattern in the string
	matches := deviceIDPattern.FindString(s)
	if matches != "" {
		return matches, nil
	}

	return "", fmt.Errorf("invalid device ID: %s", s)
}

// DeviceIDShort2Long expands a short device ID to a full one by matching against known devices
func DeviceIDShort2Long(short string, knownDevices []string) string {
	short = strings.ToUpper(strings.TrimSpace(short))

	// If already full length, return as-is
	if len(short) == 63 { // 8*7 + 7 dashes
		return short
	}

	var matches []string
	for _, dev := range knownDevices {
		if strings.HasPrefix(strings.ToUpper(dev), short) {
			matches = append(matches, dev)
		}
	}

	if len(matches) == 1 {
		return matches[0]
	}
	return ""
}

// DeviceIDLong2Name returns a short device ID or name for display
func DeviceIDLong2Name(long string, devicesMap map[string]map[string]any) string {
	short := long
	if len(long) >= 7 {
		short = long[:7]
	}

	if devicesMap != nil {
		if dev, ok := devicesMap[long]; ok {
			if name, ok := dev["name"].(string); ok && name != "" {
				nameLower := strings.ToLower(name)
				if nameLower != "syncweb" && nameLower != "syncthing" {
					return fmt.Sprintf("%s (%s)", name, short)
				}
			}
		}
	}

	// Check if it's the local device
	if len(long) >= 7 {
		return short
	}
	return short + "-???????"
}

// CreateFolderID generates a folder ID from a path
func CreateFolderID(path string, existingFolders map[string]bool) string {
	name := filepath.Base(path)

	if !existingFolders[name] {
		return name
	}

	// If basename exists, use full path with separators replaced
	return SepReplace(path)
}

// SepReplace replaces path separators with dashes for use as folder ID
func SepReplace(path string) string {
	// Replace both forward and back slashes with dashes
	result := strings.ReplaceAll(path, "/", "-")
	result = strings.ReplaceAll(result, "\\", "-")
	// Clean up multiple dashes
	for strings.Contains(result, "--") {
		result = strings.ReplaceAll(result, "--", "-")
	}
	// Remove leading/trailing dashes
	result = strings.Trim(result, "-")
	return result
}

// FormatSizeHuman formats bytes to human-readable string (alias for FormatSize)
func FormatSizeHuman(bytes int64) string {
	return FormatSize(bytes)
}

// RelativeTime formats a timestamp to relative time string
func RelativeTime(timestamp int64) string {
	if timestamp == 0 {
		return "-"
	}

	t := time.Unix(timestamp, 0)
	now := time.Now()
	diff := now.Sub(t)

	if diff < 0 {
		// Future
		absDiff := -diff
		if absDiff < time.Minute {
			return "in a few seconds"
		}
		if absDiff < time.Hour {
			mins := int(absDiff.Minutes())
			return fmt.Sprintf("in %d minutes", mins)
		}
		if absDiff < 24*time.Hour {
			hours := int(absDiff.Hours())
			return fmt.Sprintf("in %d hours", hours)
		}
		if absDiff < 48*time.Hour {
			return "tomorrow, " + t.Format("15:04")
		}
		if absDiff < 30*24*time.Hour {
			days := int(absDiff.Hours() / 24)
			return fmt.Sprintf("in %d days, %s", days, t.Format("15:04"))
		}
		return t.Format("2006-01-02")
	}

	// Past
	if diff < time.Minute {
		return "just now"
	}
	if diff < time.Hour {
		mins := int(diff.Minutes())
		return fmt.Sprintf("%d minutes ago", mins)
	}
	if diff < 24*time.Hour {
		hours := int(diff.Hours())
		return fmt.Sprintf("%d hours ago", hours)
	}
	if diff < 48*time.Hour {
		return "yesterday, " + t.Format("15:04")
	}
	if diff < 30*24*time.Hour {
		days := int(diff.Hours() / 24)
		return fmt.Sprintf("%d days ago, %s", days, t.Format("15:04"))
	}
	return t.Format("2006-01-02")
}

// FormatTimeLong formats a timestamp for long listing
func FormatTimeLong(timestamp int64) string {
	if timestamp == 0 {
		return "-"
	}
	t := time.Unix(timestamp, 0)
	// Format like: "06 Jan 15:04" or "02 Jan  2022"
	now := time.Now()
	if t.Year() == now.Year() && t.YearDay() == now.YearDay() {
		return t.Format("15:04")
	}
	if t.Year() == now.Year() {
		return t.Format("02 Jan 15:04")
	}
	return t.Format("02 Jan 2006")
}

// IsoDateToSeconds converts ISO 8601 datetime to Unix timestamp
func IsoDateToSeconds(isoDate string) int64 {
	if isoDate == "" {
		return 0
	}

	// Replace Z with +00:00 for Go parsing
	isoDate = strings.Replace(isoDate, "Z", "+00:00", 1)

	// Try RFC3339 first
	if t, err := time.Parse(time.RFC3339, isoDate); err == nil {
		return t.Unix()
	}

	// Try RFC3339Nano
	if t, err := time.Parse(time.RFC3339Nano, isoDate); err == nil {
		return t.Unix()
	}

	// Try basic ISO format
	if t, err := time.Parse("2006-01-02T15:04:05", isoDate); err == nil {
		return t.Unix()
	}

	// Try date only
	if t, err := time.Parse("2006-01-02", isoDate); err == nil {
		return t.Unix()
	}

	return 0
}

// ParseHumanToRange parses human-readable constraints into a Range
// Supports formats like: "3 days", "+6", "-10", "5%10", ">100", "<50"
func ParseHumanToRange(s string, converter func(string) (int64, error)) (Range, error) {
	s = strings.TrimSpace(s)
	if s == "" {
		return Range{}, nil
	}

	// Handle prefix notation
	if strings.HasPrefix(s, "+") {
		val, err := converter(s[1:])
		if err != nil {
			return Range{}, err
		}
		return Range{Min: &val}, nil
	}
	if strings.HasPrefix(s, "-") {
		val, err := converter(s[1:])
		if err != nil {
			return Range{}, err
		}
		return Range{Max: &val}, nil
	}

	// Use existing ParseRange
	return ParseRange(s, converter)
}
