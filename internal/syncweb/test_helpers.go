//go:build !e2e
// +build !e2e

package syncweb

import (
	"os"
	"path/filepath"
	"strings"
)

// cleanupTestHomeDir removes Syncthing temp, database, and config files from a test home directory.
// This should be called in tests after Stop() to avoid "directory not empty" errors
// during t.TempDir() cleanup.
func cleanupTestHomeDir(homeDir string) error {
	entries, err := os.ReadDir(homeDir)
	if err != nil {
		return err
	}
	for _, entry := range entries {
		name := entry.Name()
		// Remove Syncthing temp files, database files, and config files
		if strings.HasPrefix(name, ".syncthing.tmp.") ||
			strings.HasPrefix(name, "index-v") ||
			strings.HasPrefix(name, ".syncthing") ||
			strings.HasSuffix(name, ".xml") ||
			strings.HasSuffix(name, ".pem") {
			path := filepath.Join(homeDir, name)
			_ = os.RemoveAll(path) // Best-effort cleanup
		}
	}
	return nil
}

// stopAndCleanup stops a Syncweb instance and cleans up its home directory.
// Use this with defer: defer stopAndCleanup(sw, homeDir)
func stopAndCleanup(sw *Syncweb, homeDir string) {
	sw.Stop()
	_ = cleanupTestHomeDir(homeDir)
}

// stopNodeAndCleanup stops a Node and cleans up its home directory.
// Use this with defer: defer stopNodeAndCleanup(node, homeDir)
func stopNodeAndCleanup(node *Node, homeDir string) {
	node.Stop()
	_ = cleanupTestHomeDir(homeDir)
}
