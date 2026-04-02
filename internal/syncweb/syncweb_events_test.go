package syncweb

import (
	"testing"
	"github.com/chapmanjacobd/syncweb/internal/models"
)

func TestGetEvents(t *testing.T) {
	s := &Syncweb{}
	
	// Test before initialization
	events := s.GetEvents()
	if events != nil {
		t.Errorf("expected nil events before initialization, got %v", events)
	}

	// Test after initialization
	s.events = make([]models.SyncEvent, 0)
	s.eventsCache.Store(s.events)
	events = s.GetEvents()
	if events == nil || len(events) != 0 {
		t.Errorf("expected empty slice after initialization, got %v", events)
	}

	// Test after adding events
	s.addEvent("TestType", "TestMessage", nil)
	events = s.GetEvents()
	if len(events) != 1 {
		t.Fatalf("expected 1 event, got %d", len(events))
	}
	if events[0].Type != "TestType" || events[0].Message != "TestMessage" {
		t.Errorf("unexpected event data: %v", events[0])
	}

	// Verify it's a copy
	events[0].Type = "Modified"
	events2 := s.GetEvents()
	if events2[0].Type != "TestType" {
		t.Errorf("expected original type, got %s (copy modification affected cache)", events2[0].Type)
	}
}
