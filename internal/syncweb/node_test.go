package syncweb_test

import (
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
)

func TestNodeLifecycle(t *testing.T) {
	tests := []struct {
		name string
		test func(*testing.T)
	}{
		{"basic lifecycle", testNodeBasicLifecycle},
		{"double start", testNodeDoubleStart},
		{"double stop", testNodeDoubleStop},
		{"restart after stop", testNodeRestart},
		{"invalid home directory", testNodeInvalidHome},
		{"empty home directory", testNodeEmptyHome},
	}

	for _, tt := range tests {
		t.Run(tt.name, tt.test)
	}
}

func setupNode(t *testing.T, home, name, addr string) *syncweb.Node {
	node, err := syncweb.NewNode(home, name, addr)
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}
	t.Cleanup(func() {
		node.Stop()
		_ = syncweb.CleanupTestHomeDir(home)
		// Give the OS a moment to fully release all file handles
		time.Sleep(250 * time.Millisecond)
	})
	return node
}

func testNodeBasicLifecycle(t *testing.T) {
	home := t.TempDir()
	node := setupNode(t, home, "test-node", "tcp://127.0.0.1:0")

	if node.IsRunning() {
		t.Error("node should not be running yet")
	}

	if startErr := node.Start(); startErr != nil {
		t.Fatalf("failed to start node: %v", startErr)
	}

	if !node.IsRunning() {
		t.Error("node should be running")
	}

	time.Sleep(100 * time.Millisecond)
}

func testNodeDoubleStart(t *testing.T) {
	home := t.TempDir()
	node := setupNode(t, home, "test-node", "tcp://127.0.0.1:0")

	if startErr := node.Start(); startErr != nil {
		t.Fatalf("failed to start node: %v", startErr)
	}

	// Double start should be a no-op
	if startErr := node.Start(); startErr != nil {
		t.Errorf("double start failed: %v", startErr)
	}
}

func testNodeDoubleStop(t *testing.T) {
	home := t.TempDir()
	node := setupNode(t, home, "test-node", "tcp://127.0.0.1:0")

	if startErr := node.Start(); startErr != nil {
		t.Fatalf("failed to start node: %v", startErr)
	}

	node.Stop()
	if node.IsRunning() {
		t.Error("node should not be running after stop")
	}

	// Double stop should be a no-op
	node.Stop()
}

func testNodeRestart(t *testing.T) {
	home := t.TempDir()

	{
		node := setupNode(t, home, "restart-node", "tcp://127.0.0.1:0")
		if startErr := node.Start(); startErr != nil {
			t.Fatalf("failed to start node: %v", startErr)
		}
		// setupNode will clean it up via Cleanup when test finishes,
		// but we need to stop it manually to restart it on the same home.
		node.Stop()
		_ = syncweb.CleanupTestHomeDir(home)
		time.Sleep(250 * time.Millisecond)
	}

	// Re-create node from same home
	node2 := setupNode(t, home, "restart-node", "tcp://127.0.0.1:0")
	if startErr2 := node2.Start(); startErr2 != nil {
		t.Fatalf("failed to restart node: %v", startErr2)
	}
}

func testNodeInvalidHome(t *testing.T) {
	// Try a read-only or non-existent path where we can't write
	_, err := syncweb.NewNode("/root/should-not-be-writable", "test", "")
	if err == nil {
		t.Error("expected error with invalid home dir")
	}
}

func testNodeEmptyHome(t *testing.T) {
	// Empty home should create a temp directory
	node := setupNode(t, "", "test", "tcp://127.0.0.1:0")
	home := filepath.Dir(node.Cfg.ConfigPath())
	// In this special case, the temp dir is NOT homeDir passed to setupNode (which was ""),
	// so we add an extra cleanup for the auto-generated home.
	t.Cleanup(func() {
		os.RemoveAll(home)
	})

	if startErr := node.Start(); startErr != nil {
		t.Fatalf("failed to start node: %v", startErr)
	}

	if !node.IsRunning() {
		t.Error("node should be running")
	}
}

func TestNodeInvalidListenAddr(t *testing.T) {
	home := t.TempDir()
	node := setupNode(t, home, "test-node", "invalid-addr")

	if startErr := node.Start(); startErr != nil {
		t.Fatalf("failed to start node: %v", startErr)
	}
}

func TestNodeMyID(t *testing.T) {
	home := t.TempDir()
	node := setupNode(t, home, "test-node", "tcp://127.0.0.1:0")

	id := node.MyID()
	if id.String() == "" {
		t.Error("MyID() returned empty device ID")
	}

	// Verify ID is consistent
	id2 := node.MyID()
	if id != id2 {
		t.Errorf("MyID() returned different IDs: %v vs %v", id, id2)
	}
}

func TestNodeSubscribe(t *testing.T) {
	home := t.TempDir()
	node := setupNode(t, home, "test-node", "tcp://127.0.0.1:0")

	sub := node.Subscribe(0) // Subscribe to all events
	if sub == nil {
		t.Error("Subscribe() returned nil")
	}
	defer sub.Unsubscribe()
}
