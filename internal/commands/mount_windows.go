//go:build windows

package commands

import "path/filepath"

// getMountpoint returns the mountpoint for a path (simplified for Windows)
func getMountpoint(path string) string {
	absPath, _ := filepath.Abs(path)
	return filepath.VolumeName(absPath) + "\\"
}
