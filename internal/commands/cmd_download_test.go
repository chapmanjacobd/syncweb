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

func TestGroupFoldersByMountpoint(t *testing.T) {
	folderSpaceInfos := map[string]*folderSpaceInfo{
		"f1": {Mountpoint: "/mnt/data/a"},
		"f2": {Mountpoint: "/mnt/data/a"},
		"f3": {Mountpoint: "/mnt/data/b"},
	}

	groups := groupFoldersByMountpoint(folderSpaceInfos)

	if len(groups["/mnt/data/a"]) != 2 {
		t.Errorf("expected 2 folders in /mnt/data/a, got %d", len(groups["/mnt/data/a"]))
	}
	if len(groups["/mnt/data/b"]) != 1 {
		t.Errorf("expected 1 folder in /mnt/data/b, got %d", len(groups["/mnt/data/b"]))
	}
}
