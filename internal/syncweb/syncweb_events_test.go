package syncweb_test

import (
	"testing"

	"github.com/chapmanjacobd/syncweb/internal/models"
	"github.com/chapmanjacobd/syncweb/internal/syncweb"
)

func TestGetEvents(t *testing.T) {
	s := &syncweb.Syncweb{}

	// Test before initialization
	events := s.GetEvents()
	if events != nil {
		t.Errorf("expected nil events before initialization, got %v", events)
	}

	homeDir := t.TempDir()
	initialized, err := syncweb.NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	defer syncweb.StopAndCleanup(initialized, homeDir)

	events = initialized.GetEvents()
	if events == nil {
		t.Errorf("expected non-nil events after initialization, got nil")
	}

	events = append(events, models.SyncEvent{Type: "mutated"})
	events2 := initialized.GetEvents()
	if len(events2) != 0 {
		t.Errorf("expected GetEvents to return an independent copy, got %d events", len(events2))
	}
}
