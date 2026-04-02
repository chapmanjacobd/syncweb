package syncweb_test

import (
	"reflect"
	"testing"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
)

func TestAddIgnores(t *testing.T) {
	// Note: We need a real node or a mock of Node.App.Internals for this.
	// Since Syncweb uses Node.App.Internals, and that's hard to mock without
	// interfaces, we'll try to use a temporary node.
	
	home := t.TempDir()
	s, err := syncweb.NewSyncweb(home, "test", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}
	if err := s.Start(); err != nil {
		t.Fatalf("failed to start node: %v", err)
	}
	defer s.Stop()

	folderID := "testfolder"
	if err := s.AddFolder(folderID, folderID, home, 0); err != nil {
		t.Fatalf("failed to add folder: %v", err)
	}
	if err := s.ResumeFolder(folderID); err != nil {
		t.Fatalf("failed to resume folder: %v", err)
	}
	if err := s.WaitUntilIdle(folderID, 5*time.Second); err != nil {
		t.Logf("Warning: WaitUntilIdle timed out (expected if folder is new): %v", err)
	}

	// 1. Set some initial user ignores
	initialIgnores := []string{"pattern1", "pattern2"}
	if err := s.SetIgnores(folderID, initialIgnores); err != nil {
		t.Fatalf("failed to set ignores: %v", err)
	}
	_ = s.ScanFolders()
	time.Sleep(100 * time.Millisecond)

	// 2. Add managed ignores
	managed := []string{"file.txt"}
	if err := s.AddIgnores(folderID, managed); err != nil {
		t.Fatalf("failed to add managed ignores: %v", err)
	}
	_ = s.ScanFolders()
	time.Sleep(100 * time.Millisecond)

	// 3. Check final ignores
	final, err := s.GetIgnores(folderID)
	if err != nil {
		t.Fatalf("failed to get final ignores: %v", err)
	}

	expected := []string{
		"pattern1",
		"pattern2",
		"// BEGIN Syncweb-managed",
		"!/file.txt",
		"*",
		"// END Syncweb-managed",
	}

	if !reflect.DeepEqual(final, expected) {
		t.Errorf("ignores mismatch.\nGot: %v\nWant: %v", final, expected)
	}

	// 4. Test adding more user ignores AFTER AddIgnores
	// If AddIgnores added a '*', anything after it in the file will be ignored by Syncthing.
	// But AddIgnores *reconstructs* the whole list.
	
	newUserIgnores := append(final, "pattern3")
	t.Logf("Setting newUserIgnores: %v", newUserIgnores)
	if err := s.SetIgnores(folderID, newUserIgnores); err != nil {
		t.Fatalf("failed to set ignores with pattern3: %v", err)
	}
	_ = s.ScanFolders()
	time.Sleep(100 * time.Millisecond)
	
	// If we call AddIgnores again, does it preserve pattern3?
	if err := s.AddIgnores(folderID, []string{"another.txt"}); err != nil {
		t.Fatalf("failed to add second managed ignore: %v", err)
	}
	
	final2, _ := s.GetIgnores(folderID)
	t.Logf("Final ignores after second AddIgnores: %v", final2)
	
	// Search for pattern3
	found := false
	for _, p := range final2 {
		if p == "pattern3" {
			found = true
			break
		}
	}
	
	if !found {
		t.Error("pattern3 was lost after second AddIgnores")
	}
	
	// The REAL problem is if the user adds patterns MANUALLY to the .stignore file
	// outside of Syncweb's API, and Syncweb puts its block at the end with a '*'.
}
