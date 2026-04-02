//nolint:testpackage // Need access to internal types for testing
package commands

import (
	"testing"
)

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
