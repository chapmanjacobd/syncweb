package version

import (
	"strings"
	"testing"
)

func TestInfo(t *testing.T) {
	// Save original values
	origVersion := Version
	origBuildTime := BuildTime
	origGitHash := GitHash
	origGitDirty := GitDirty
	defer func() {
		Version = origVersion
		BuildTime = origBuildTime
		GitHash = origGitHash
		GitDirty = origGitDirty
	}()

	t.Run("default dev version", func(t *testing.T) {
		Version = "dev"
		BuildTime = "unknown"
		GitHash = "unknown"
		GitDirty = ""

		info := Info()
		if !strings.Contains(info, "syncweb dev") {
			t.Errorf("Expected 'syncweb dev', got %q", info)
		}
	})

	t.Run("version with git hash", func(t *testing.T) {
		Version = "v1.0.0"
		BuildTime = "2024-01-01 00:00:00 UTC"
		GitHash = "abc123"
		GitDirty = ""

		info := Info()
		if !strings.Contains(info, "syncweb v1.0.0") {
			t.Errorf("Expected 'syncweb v1.0.0', got %q", info)
		}
		if !strings.Contains(info, "2024-01-01") {
			t.Errorf("Expected build time in info, got %q", info)
		}
	})

	t.Run("dirty version", func(t *testing.T) {
		Version = "v1.0.0"
		BuildTime = "unknown"
		GitHash = "abc123"
		GitDirty = "-dirty"

		info := Info()
		if !strings.Contains(info, "-dirty") {
			t.Errorf("Expected '-dirty' in info, got %q", info)
		}
	})
}

func TestFullInfo(t *testing.T) {
	// Save original values
	origVersion := Version
	origBuildTime := BuildTime
	origGitHash := GitHash
	origGitDirty := GitDirty
	defer func() {
		Version = origVersion
		BuildTime = origBuildTime
		GitHash = origGitHash
		GitDirty = origGitDirty
	}()

	t.Run("full info with all fields", func(t *testing.T) {
		Version = "v1.0.0"
		BuildTime = "2024-01-01 00:00:00 UTC"
		GitHash = "abc123"
		GitDirty = "-dirty"

		info := FullInfo()
		if !strings.Contains(info, "syncweb v1.0.0-dirty") {
			t.Errorf("Expected version with dirty flag, got %q", info)
		}
		if !strings.Contains(info, "commit:   abc123") {
			t.Errorf("Expected commit hash, got %q", info)
		}
		if !strings.Contains(info, "built:    2024-01-01") {
			t.Errorf("Expected build time, got %q", info)
		}
	})

	t.Run("full info without hash", func(t *testing.T) {
		Version = "dev"
		BuildTime = "unknown"
		GitHash = "unknown"
		GitDirty = ""

		info := FullInfo()
		if !strings.Contains(info, "syncweb dev") {
			t.Errorf("Expected 'syncweb dev', got %q", info)
		}
		// Should not contain "commit:" or "built:" for unknown values
		if strings.Contains(info, "commit:   unknown") {
			t.Errorf("Should not contain unknown commit, got %q", info)
		}
	})
}
