//go:build windows

package syncweb

func (n *Node) nodeLock(homeDir string) error {
	// Locking not implemented on Windows yet
	return nil
}

func (n *Node) nodeUnlock() {
}
