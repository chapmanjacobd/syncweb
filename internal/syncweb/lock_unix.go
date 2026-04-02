//go:build !windows

package syncweb

import (
	"fmt"
	"os"
	"path/filepath"

	"golang.org/x/sys/unix"
)

func (n *Node) nodeLock(homeDir string) error {
	lockFilePath := filepath.Join(homeDir, "syncweb.lock")
	lockFile, err := os.OpenFile(lockFilePath, os.O_CREATE|os.O_RDWR, 0o600)
	if err != nil {
		return fmt.Errorf("failed to open lock file: %w", err)
	}

	if err := unix.Flock(int(lockFile.Fd()), unix.LOCK_EX|unix.LOCK_NB); err != nil {
		_ = lockFile.Close()
		return fmt.Errorf("failed to lock home directory: %w (another instance might be running)", err)
	}

	n.lockFile = lockFile
	return nil
}

func (n *Node) nodeUnlock() {
	if n.lockFile != nil {
		_ = unix.Flock(int(n.lockFile.Fd()), unix.LOCK_UN)
		_ = n.lockFile.Close()
		n.lockFile = nil
	}
}
