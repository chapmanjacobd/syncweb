package utils

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func TestSafeUnmountRemovable(t *testing.T) {
	if os.Getenv("CI") != "" || os.Getuid() != 0 {
		// This test requires root and loop device support
		// If sudo -n true worked before, we might be able to run it with sudo
		// but for now let's try to run it and see.
	}

	tmpDir, err := os.MkdirTemp("", "syncweb-mount-test")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmpDir)

	imagePath := filepath.Join(tmpDir, "test.img")
	// Create a 64MB FAT32 image
	if err := exec.Command("truncate", "-s", "64M", imagePath).Run(); err != nil {
		t.Fatal(err)
	}
	if err := exec.Command("mkfs.vfat", imagePath).Run(); err != nil {
		t.Fatal(err)
	}

	// Setup loop device
	out, err := exec.Command("sudo", "losetup", "-f", "--show", imagePath).Output()
	if err != nil {
		t.Fatalf("losetup failed: %v", err)
	}
	loopDev := strings.TrimSpace(string(out))
	defer exec.Command("sudo", "losetup", "-d", loopDev).Run()

	// Mount it twice
	mp1 := filepath.Join(tmpDir, "mnt1")
	mp2 := filepath.Join(tmpDir, "mnt2")
	os.MkdirAll(mp1, 0755)
	os.MkdirAll(mp2, 0755)

	if err := exec.Command("sudo", "mount", loopDev, mp1).Run(); err != nil {
		t.Fatal(err)
	}
	defer exec.Command("sudo", "umount", mp1).Run()

	if err := exec.Command("sudo", "mount", loopDev, mp2).Run(); err != nil {
		t.Fatal(err)
	}
	defer exec.Command("sudo", "umount", mp2).Run()

	t.Logf("Mounted %s to %s and %s", loopDev, mp1, mp2)

	// Call our Unmount function on mp1
	if err := Unmount(mp1); err != nil {
		t.Fatalf("Unmount failed: %v", err)
	}

	// Verify both mp1 and mp2 are unmounted
	if isMounted(mp1) {
		t.Errorf("%s is still mounted", mp1)
	}
	if isMounted(mp2) {
		t.Errorf("%s is still mounted", mp2)
	}
}

func TestSafePrepareForRead(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "syncweb-prepare-test")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmpDir)

	imagePath := filepath.Join(tmpDir, "test.img")
	exec.Command("truncate", "-s", "64M", imagePath).Run()
	exec.Command("mkfs.vfat", imagePath).Run()

	out, _ := exec.Command("sudo", "losetup", "-f", "--show", imagePath).Output()
	loopDev := strings.TrimSpace(string(out))
	defer exec.Command("sudo", "losetup", "-d", loopDev).Run()

	mp1 := filepath.Join(tmpDir, "mnt1")
	mp2 := filepath.Join(tmpDir, "mnt2")
	os.MkdirAll(mp1, 0755)
	os.MkdirAll(mp2, 0755)

	exec.Command("sudo", "mount", loopDev, mp1).Run()
	exec.Command("sudo", "mount", loopDev, mp2).Run()
	defer exec.Command("sudo", "umount", "-l", loopDev).Run()

	if err := SafePrepareForRead(loopDev); err != nil {
		t.Fatalf("SafePrepareForRead failed: %v", err)
	}

	// Verify that exactly one of them is still mounted
	m1 := isMounted(mp1)
	m2 := isMounted(mp2)

	if m1 && m2 {
		t.Errorf("Both %s and %s are still mounted, should have unmounted one", mp1, mp2)
	}
	if !m1 && !m2 {
		t.Errorf("Neither %s nor %s is mounted, should have kept one", mp1, mp2)
	}
}

func isMounted(path string) bool {
	out, _ := exec.Command("mount").Output()
	return strings.Contains(string(out), path)
}
