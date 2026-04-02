package syncweb_test

import (
	"testing"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
)

func TestConcurrentNodesSameHome(t *testing.T) {
	home := t.TempDir()

	// Start the first node
	s1, err := syncweb.NewSyncweb(home, "node1", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node 1: %v", err)
	}
	if err := s1.Start(); err != nil {
		t.Fatalf("failed to start node 1: %v", err)
	}
	t.Logf("Node 1 ID: %v", s1.Node.MyID())
	defer syncweb.StopAndCleanup(s1, home)

	// Attempt to start a second node on the same home
	s2, err := syncweb.NewSyncweb(home, "node2", "tcp://127.0.0.1:0")
	if err != nil {
		t.Logf("NewSyncweb for node 2 failed as expected: %v", err)
		return
	}
	defer syncweb.StopAndCleanup(s2, home)
	t.Logf("Node 2 ID: %v", s2.Node.MyID())

	t.Error("expected error when creating second node on same home directory, but got nil")
}
