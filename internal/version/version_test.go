package version_test

import (
	"strings"
	"testing"

	"github.com/chapmanjacobd/syncweb/internal/version"
)

func TestInfo(t *testing.T) {
	t.Run("default dev version", func(t *testing.T) {
		info := version.Info()
		if !strings.Contains(info, "syncweb") {
			t.Errorf("Expected 'syncweb', got %q", info)
		}
	})
}

func TestFullInfo(t *testing.T) {
	t.Run("full info", func(t *testing.T) {
		info := version.FullInfo()
		if !strings.Contains(info, "syncweb") {
			t.Errorf("Expected 'syncweb', got %q", info)
		}
		// Should contain commit and built sections
		if !strings.Contains(info, "commit:") {
			t.Errorf("Expected commit section, got %q", info)
		}
		if !strings.Contains(info, "built:") {
			t.Errorf("Expected built section, got %q", info)
		}
	})
}
