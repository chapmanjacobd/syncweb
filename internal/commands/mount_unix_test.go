//go:build !windows

package commands

import (
	"path/filepath"
	"testing"
)

func TestGetMountpoint(t *testing.T) {
	// Since we can't easily mock syscall.Stat, we'll test with the current directory
	// which is guaranteed to exist.

	dir := "."
	absDir, _ := filepath.Abs(dir)
	got := getMountpoint(dir)

	if got == "" {
		t.Error("getMountpoint returned empty string")
	}

	if !filepath.IsAbs(got) {
		t.Errorf("getMountpoint returned relative path: %s", got)
	}

	if !filepath.HasPrefix(absDir, got) {
		t.Errorf("getMountpoint result %s is not a prefix of %s", got, absDir)
	}
}
