package syncweb_test

import (
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

func testNodeBasicLifecycle(t *testing.T) {
	home := t.TempDir()

	node, err := syncweb.NewNode(home, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}

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

	node.Stop()
	_ = syncweb.CleanupTestHomeDir(home)
	if node.IsRunning() {
		t.Error("node should not be running after stop")
	}
}

func testNodeDoubleStart(t *testing.T) {
	home := t.TempDir()

	node, err := syncweb.NewNode(home, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}

	if startErr := node.Start(); startErr != nil {
		t.Fatalf("failed to start node: %v", startErr)
	}
	defer func() {
		node.Stop()
		_ = syncweb.CleanupTestHomeDir(home)
	}()

	// Double start should be a no-op
	if startErr := node.Start(); startErr != nil {
		t.Errorf("double start failed: %v", startErr)
	}
}

func testNodeDoubleStop(t *testing.T) {
	home := t.TempDir()

	node, err := syncweb.NewNode(home, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}

	if startErr := node.Start(); startErr != nil {
		t.Fatalf("failed to start node: %v", startErr)
	}

	node.Stop()
	_ = syncweb.CleanupTestHomeDir(home)
	if node.IsRunning() {
		t.Error("node should not be running after stop")
	}

	// Double stop should be a no-op
	node.Stop()
}

func testNodeRestart(t *testing.T) {
	home := t.TempDir()

	node, err := syncweb.NewNode(home, "restart-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}

	if startErr := node.Start(); startErr != nil {
		t.Fatalf("failed to start node: %v", startErr)
	}
	node.Stop()
	_ = syncweb.CleanupTestHomeDir(home)

	// Re-create node from same home
	node2, err := syncweb.NewNode(home, "restart-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to re-create node: %v", err)
	}

	if err := node2.Start(); err != nil {
		t.Fatalf("failed to restart node: %v", err)
	}
	node2.Stop()
	_ = syncweb.CleanupTestHomeDir(home)
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
	// Note: We don't clean up the temp dir here since we don't track it
	node, err := syncweb.NewNode("", "test", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node with empty home: %v", err)
	}
	defer node.Stop()

	if startErr := node.Start(); startErr != nil {
		t.Fatalf("failed to start node: %v", startErr)
	}

	if !node.IsRunning() {
		t.Error("node should be running")
	}
}

func TestNodeInvalidListenAddr(t *testing.T) {
	home := t.TempDir()

	// Invalid listen address should still work (Syncthing will use default)
	node, err := syncweb.NewNode(home, "test-node", "invalid-addr")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}
	defer func() {
		node.Stop()
		_ = syncweb.CleanupTestHomeDir(home)
	}()

	if startErr := node.Start(); startErr != nil {
		t.Fatalf("failed to start node: %v", startErr)
	}
}

func TestNodeMyID(t *testing.T) {
	home := t.TempDir()

	node, err := syncweb.NewNode(home, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}
	defer func() {
		node.Stop()
		_ = syncweb.CleanupTestHomeDir(home)
	}()

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

	node, err := syncweb.NewNode(home, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}
	defer func() {
		node.Stop()
		_ = syncweb.CleanupTestHomeDir(home)
	}()

	sub := node.Subscribe(0) // Subscribe to all events
	if sub == nil {
		t.Error("Subscribe() returned nil")
	}
	defer sub.Unsubscribe()
}
