package syncweb

import (
	"os"
	"testing"
	"time"
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
	home, err := os.MkdirTemp("", "node-test-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(home)

	node, err := NewNode(home, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}

	if node.IsRunning() {
		t.Error("node should not be running yet")
	}

	if err := node.Start(); err != nil {
		t.Fatalf("failed to start node: %v", err)
	}

	if !node.IsRunning() {
		t.Error("node should be running")
	}

	time.Sleep(100 * time.Millisecond)

	node.Stop()
	if node.IsRunning() {
		t.Error("node should not be running after stop")
	}
}

func testNodeDoubleStart(t *testing.T) {
	home, err := os.MkdirTemp("", "node-test-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(home)

	node, err := NewNode(home, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}

	if err := node.Start(); err != nil {
		t.Fatalf("failed to start node: %v", err)
	}
	defer node.Stop()

	// Double start should be a no-op
	if err := node.Start(); err != nil {
		t.Errorf("double start failed: %v", err)
	}
}

func testNodeDoubleStop(t *testing.T) {
	home, err := os.MkdirTemp("", "node-test-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(home)

	node, err := NewNode(home, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}

	if err := node.Start(); err != nil {
		t.Fatalf("failed to start node: %v", err)
	}

	node.Stop()
	if node.IsRunning() {
		t.Error("node should not be running after stop")
	}

	// Double stop should be a no-op
	node.Stop()
}

func testNodeRestart(t *testing.T) {
	home, err := os.MkdirTemp("", "node-restart-test-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(home)

	node, err := NewNode(home, "restart-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}

	if err := node.Start(); err != nil {
		t.Fatalf("failed to start node: %v", err)
	}
	node.Stop()

	// Re-create node from same home
	node2, err := NewNode(home, "restart-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to re-create node: %v", err)
	}

	if err := node2.Start(); err != nil {
		t.Fatalf("failed to restart node: %v", err)
	}
	node2.Stop()
}

func testNodeInvalidHome(t *testing.T) {
	// Try a read-only or non-existent path where we can't write
	_, err := NewNode("/root/should-not-be-writable", "test", "")
	if err == nil {
		t.Error("expected error with invalid home dir")
	}
}

func testNodeEmptyHome(t *testing.T) {
	// Empty home should create a temp directory
	node, err := NewNode("", "test", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node with empty home: %v", err)
	}
	defer node.Stop()

	if err := node.Start(); err != nil {
		t.Fatalf("failed to start node: %v", err)
	}

	if !node.IsRunning() {
		t.Error("node should be running")
	}
}

func TestNodeInvalidListenAddr(t *testing.T) {
	home, err := os.MkdirTemp("", "node-test-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(home)

	// Invalid listen address should still work (Syncthing will use default)
	node, err := NewNode(home, "test-node", "invalid-addr")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}
	defer node.Stop()

	if err := node.Start(); err != nil {
		t.Fatalf("failed to start node: %v", err)
	}
}

func TestNodeMyID(t *testing.T) {
	home, err := os.MkdirTemp("", "node-test-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(home)

	node, err := NewNode(home, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}
	defer node.Stop()

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
	home, err := os.MkdirTemp("", "node-test-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(home)

	node, err := NewNode(home, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create node: %v", err)
	}
	defer node.Stop()

	sub := node.Subscribe(0) // Subscribe to all events
	if sub == nil {
		t.Error("Subscribe() returned nil")
	}
	defer sub.Unsubscribe()
}
