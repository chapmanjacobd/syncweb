package syncweb

import (
	"context"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"
)

type TestCluster struct {
	Nodes   []*Syncweb
	TempDir string
}

func NewTestCluster(t *testing.T, count int) *TestCluster {
	tempDir, err := os.MkdirTemp("", "syncweb-cluster-")
	if err != nil {
		t.Fatal(err)
	}

	cluster := &TestCluster{
		TempDir: tempDir,
	}

	for i := range count {
		home := filepath.Join(tempDir, fmt.Sprintf("node-%d", i))
		listenAddr := fmt.Sprintf("tcp://127.0.0.1:%d", 22000+i)
		sw, err := NewSyncweb(home, fmt.Sprintf("node-%d", i), listenAddr)
		if err != nil {
			t.Fatal(err)
		}
		cluster.Nodes = append(cluster.Nodes, sw)
	}

	return cluster
}

func (c *TestCluster) Close() {
	for _, node := range c.Nodes {
		node.Stop()
	}
	os.RemoveAll(c.TempDir)
}

func (c *TestCluster) ConnectAll(t *testing.T) {
	for i, nodeA := range c.Nodes {
		for j, nodeB := range c.Nodes {
			if i == j {
				continue
			}
			if err := nodeA.AddDevice(nodeB.Node.MyID().String(), fmt.Sprintf("node-%d", j), false); err != nil {
				t.Fatal(err)
			}
			addr := fmt.Sprintf("tcp://127.0.0.1:%d", 22000+j)
			if err := nodeA.SetDeviceAddresses(nodeB.Node.MyID().String(), []string{addr}); err != nil {
				t.Fatal(err)
			}
		}
	}
}

func (c *TestCluster) ShareFolder(t *testing.T, folderID string) {
	for i, node := range c.Nodes {
		path := filepath.Join(filepath.Dir(node.Node.Cfg.ConfigPath()), "data", folderID)
		if err := node.AddFolder(folderID, folderID, path, config.FolderTypeSendReceive); err != nil {
			t.Fatal(err)
		}
		for j, other := range c.Nodes {
			if i == j {
				continue
			}
			if err := node.AddFolderDevice(folderID, other.Node.MyID().String()); err != nil {
				t.Fatal(err)
			}
		}
	}
}

func (c *TestCluster) StartAll(t *testing.T) {
	for _, node := range c.Nodes {
		if err := node.Start(); err != nil {
			t.Fatal(err)
		}
	}
}

func (c *TestCluster) WaitConnected(t *testing.T) {
	timeout := time.After(60 * time.Second)
	tick := time.Tick(1 * time.Second)

	for {
		select {
		case <-timeout:
			t.Fatal("timed out waiting for all connections")
		case <-tick:
			allConnected := true
			for i, node := range c.Nodes {
				connectedCount := 0
				for j, other := range c.Nodes {
					if i == j {
						continue
					}
					if node.Node.App.Internals.IsConnectedTo(other.Node.MyID()) {
						connectedCount++
					}
				}
				if connectedCount < len(c.Nodes)-1 {
					allConnected = false
					break
				}
			}
			if allConnected {
				return
			}
		}
	}
}

func TestSyncwebIntegration(t *testing.T) {
	cluster := NewTestCluster(t, 2)
	defer cluster.Close()

	cluster.ConnectAll(t)
	folderID := "test-folder"
	cluster.ShareFolder(t, folderID)

	// Start all nodes after they are configured
	cluster.StartAll(t)
	cluster.WaitConnected(t)

	node0 := cluster.Nodes[0]
	node1 := cluster.Nodes[1]

	// Write file to node 0
	folder0Path, _ := node0.GetFolderPath(folderID)
	testFile := "hello.txt"
	testContent := "hello from node 0"
	if err := os.WriteFile(filepath.Join(folder0Path, testFile), []byte(testContent), 0o644); err != nil {
		t.Fatal(err)
	}

	// Wait for node 1 to see the file globally
	timeout := time.After(15 * time.Second)
	var info protocol.FileInfo
	var ok bool
	for {
		select {
		case <-timeout:
			t.Fatal("timed out waiting for file info on node 1")
		default:
			var err error
			info, ok, err = node1.GetGlobalFileInfo(folderID, testFile)
			if err == nil && ok {
				goto Found
			}
			time.Sleep(500 * time.Millisecond)
		}
	}

Found:
	if info.Size != int64(len(testContent)) {
		t.Errorf("expected size %d, got %d", len(testContent), info.Size)
	}

	// Test block pulling on node 1 (even if file is not local)
	// We ensure it's not local by ignoring it on node 1 (selective sync)
	if err := node1.Node.App.Internals.SetIgnores(folderID, []string{"*"}); err != nil {
		t.Fatal(err)
	}

	rs, err := node1.NewReadSeeker(context.Background(), folderID, testFile)
	if err != nil {
		t.Fatal(err)
	}

	buf, err := io.ReadAll(rs)
	if err != nil {
		t.Fatal(err)
	}

	if string(buf) != testContent {
		t.Errorf("expected content %q, got %q", testContent, string(buf))
	}
	t.Log("Successfully pulled blocks from peer")
}

func TestSyncwebChain(t *testing.T) {
	// Test chain: node 0 <-> node 1 <-> node 2
	// Node 0 has file, node 2 should be able to pull it via node 1
	cluster := NewTestCluster(t, 3)
	defer cluster.Close()

	n0 := cluster.Nodes[0]
	n1 := cluster.Nodes[1]
	n2 := cluster.Nodes[2]

	// Connect 0-1 and 1-2
	connect := func(a, b *Syncweb, portB int) {
		if err := a.AddDevice(b.Node.MyID().String(), "peer", false); err != nil {
			t.Fatal(err)
		}
		addr := fmt.Sprintf("tcp://127.0.0.1:%d", portB)
		if err := a.SetDeviceAddresses(b.Node.MyID().String(), []string{addr}); err != nil {
			t.Fatal(err)
		}
	}

	connect(n0, n1, 22001)
	connect(n1, n0, 22000)
	connect(n1, n2, 22002)
	connect(n2, n1, 22001)

	folderID := "chain-folder"
	cluster.ShareFolder(t, folderID)
	cluster.StartAll(t)

	// Wait for 0-1 and 1-2 connections
	waitConn := func(node *Syncweb, peer protocol.DeviceID) {
		timeout := time.After(30 * time.Second)
		for {
			select {
			case <-timeout:
				t.Fatal("timed out waiting for connection")
			default:
				if node.Node.App.Internals.IsConnectedTo(peer) {
					return
				}
				time.Sleep(500 * time.Millisecond)
			}
		}
	}

	waitConn(n0, n1.Node.MyID())
	waitConn(n1, n2.Node.MyID())
	t.Log("Chain connected")

	// Write file to node 0
	folder0Path, _ := n0.GetFolderPath(folderID)
	testFile := "chain.txt"
	testContent := "chain content"
	os.WriteFile(filepath.Join(folder0Path, testFile), []byte(testContent), 0o644)

	// Wait for node 2 to see the file via node 1
	timeout := time.After(30 * time.Second)
	var ok bool
	for {
		select {
		case <-timeout:
			t.Fatal("timed out waiting for file info on node 2")
		default:
			var err error
			_, ok, err = n2.GetGlobalFileInfo(folderID, testFile)
			if err == nil && ok {
				goto Found
			}
			time.Sleep(500 * time.Millisecond)
		}
	}

Found:
	// Pull from node 2
	rs, err := n2.NewReadSeeker(context.Background(), folderID, testFile)
	if err != nil {
		t.Fatal(err)
	}

	buf, err := io.ReadAll(rs)
	if err != nil {
		t.Fatal(err)
	}

	if string(buf) != testContent {
		t.Errorf("expected content %q, got %q", testContent, string(buf))
	}
	t.Log("Successfully pulled blocks across chain")
}
