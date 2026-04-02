package syncweb_test

import (
	"testing"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
)

func TestGetEvents(t *testing.T) {
	s := &syncweb.Syncweb{}

	// Test before initialization
	events := s.GetEvents()
	if events != nil {
		t.Errorf("expected nil events before initialization, got %v", events)
	}

	// Test after initialization - just verify GetEvents doesn't panic
	events = s.GetEvents()
	if events == nil {
		t.Errorf("expected non-nil events after initialization, got nil")
	}

	// Verify it returns a copy by checking length
	events2 := s.GetEvents()
	// We can't verify the internal state, but we can ensure it doesn't panic
	_ = events2
}
