//go:build !windows

package commands

import (
	"path/filepath"
	"syscall"
)

// getMountpoint returns the mountpoint for a path
func getMountpoint(path string) string {
	absPath, err := filepath.Abs(path)
	if err != nil {
		return filepath.Dir(path)
	}

	var stat syscall.Stat_t
	if err := syscall.Stat(absPath, &stat); err != nil {
		return filepath.Dir(absPath)
	}

	dev := stat.Dev
	parent := absPath
	for {
		nextParent := filepath.Dir(parent)
		if nextParent == parent {
			return parent
		}

		var nextStat syscall.Stat_t
		if err := syscall.Stat(nextParent, &nextStat); err != nil {
			return parent
		}

		if nextStat.Dev != dev {
			return parent
		}
		parent = nextParent
	}
}
