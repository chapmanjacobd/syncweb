package syncweb_test

import (
	"os"
	"testing"
	"time"

	"github.com/syncthing/syncthing/lib/config"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
)

func TestNodeIDPersistence(t *testing.T) {
	home := t.TempDir()

	var firstID string

	// First run
	{
		sw, err := syncweb.NewSyncweb(home, "node1", "tcp://127.0.0.1:0")
		if err != nil {
			t.Fatalf("Failed to create syncweb: %v", err)
		}
		if err := sw.Start(); err != nil {
			t.Fatalf("Failed to start syncweb: %v", err)
		}
		firstID = sw.MyID().String()

		// Add a folder to test config persistence
		err = sw.AddFolder("test-folder", "Test Folder", os.TempDir(), config.FolderTypeSendReceive)
		if err != nil {
			t.Fatalf("Failed to add folder: %v", err)
		}

		sw.Stop()
		// Give some time for file locks to release
		time.Sleep(500 * time.Millisecond)
	}

	// Second run
	{
		sw, err := syncweb.NewSyncweb(home, "node1", "tcp://127.0.0.1:0")
		if err != nil {
			t.Fatalf("Failed to recreate syncweb: %v", err)
		}
		if err := sw.Start(); err != nil {
			t.Fatalf("Failed to restart syncweb: %v", err)
		}
		defer sw.Stop()

		secondID := sw.MyID().String()
		if firstID != secondID {
			t.Errorf("ID changed across restarts: %s != %s", firstID, secondID)
		}

		// Check if folder still exists in config
		folders := sw.GetFolders()
		found := false
		for _, f := range folders {
			if f.ID == "test-folder" {
				found = true
				break
			}
		}
		if !found {
			t.Error("Folder 'test-folder' not found after restart")
		}
	}
}
