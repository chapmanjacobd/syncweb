package syncweb

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/syncthing/syncthing/lib/config"
)

func TestSecurity_SyncthingConfig(t *testing.T) {
	homeDir := t.TempDir()

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	cfg := sw.Node.Cfg.RawCopy()

	// 1. Ensure usage reporting is disabled
	if cfg.Options.URAccepted != -1 {
		t.Errorf("Security: Usage reporting should be disabled (URAccepted = -1), got %d", cfg.Options.URAccepted)
	}

	// 2. Ensure browser is not started automatically
	if cfg.Options.StartBrowser {
		t.Error("Security: StartBrowser should be false")
	}

	// 3. Ensure GUI is disabled by default
	if cfg.GUI.Enabled {
		t.Error("Security: GUI should be disabled")
	}

	// 4. Check folder defaults
	if cfg.Defaults.Folder.Path != "" {
		// Syncthing lib sometimes sets default path to ~, which we want to avoid or at least be aware of.
		// In our NewNode, we don't explicitly change Defaults.Folder.Path, so it might be whatever Syncthing uses.
		t.Logf("Default folder path: %s", cfg.Defaults.Folder.Path)
	}
}

func TestSecurity_PathValidation(t *testing.T) {
	homeDir := t.TempDir()

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	syncDir := filepath.Join(homeDir, "sync")
	os.MkdirAll(syncDir, 0o700)
	folderID := "test"
	sw.AddFolder(folderID, "test", syncDir, config.FolderTypeSendReceive)

	tests := []struct {
		name    string
		path    string
		wantErr bool
	}{
		{"Normal path", "sync://test/file.txt", false},
		{"Traversal ..", "sync://test/../secret.txt", true},
		{"Traversal middle", "sync://test/dir/../../secret.txt", true},
		{"Absolute path", "sync://test//etc/passwd", true},
		{"Invalid folder", "sync://invalid/file.txt", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, _, err := sw.ResolveLocalPath(tt.path)
			if (err != nil) != tt.wantErr {
				t.Errorf("ResolveLocalPath(%q) error = %v, wantErr %v", tt.path, err, tt.wantErr)
			}
		})
	}
}
