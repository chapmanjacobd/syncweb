//go:build windows

package syncweb

import (
	"fmt"
	"os"
	"path/filepath"

	"golang.org/x/sys/windows"
)

func (n *Node) nodeLock(homeDir string) error {
	lockFilePath := filepath.Join(homeDir, "syncweb.lock")
	lockFile, err := os.OpenFile(lockFilePath, os.O_CREATE|os.O_RDWR, 0o600)
	if err != nil {
		return fmt.Errorf("failed to open lock file: %w", err)
	}

	handle := windows.Handle(lockFile.Fd())
	var overlapped windows.Overlapped
	// LOCKFILE_EXCLUSIVE_LOCK = 2
	// LOCKFILE_FAIL_IMMEDIATELY = 1
	flags := uint32(windows.LOCKFILE_EXCLUSIVE_LOCK | windows.LOCKFILE_FAIL_IMMEDIATELY)

	// Lock the first byte of the file
	if err := windows.LockFileEx(handle, flags, 0, 1, 0, &overlapped); err != nil {
		_ = lockFile.Close()
		return fmt.Errorf("failed to lock home directory: %w (another instance might be running)", err)
	}

	n.lockFile = lockFile
	return nil
}

func (n *Node) nodeUnlock() {
	if n.lockFile != nil {
		handle := windows.Handle(n.lockFile.Fd())
		var overlapped windows.Overlapped
		_ = windows.UnlockFileEx(handle, 0, 1, 0, &overlapped)
		_ = n.lockFile.Close()
		n.lockFile = nil
	}
}
