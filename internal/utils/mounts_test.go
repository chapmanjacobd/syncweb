package utils

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"github.com/chapmanjacobd/syncweb/internal/models"
)

// TestSafeUnmountRemovable tests unmounting a device mounted at multiple mountpoints.
// This test requires root privileges and loop device support.
func TestSafeUnmountRemovable(t *testing.T) {
	if os.Getenv("CI") != "" || os.Getuid() != 0 {
		t.Skip("This test requires root and loop device support")
	}

	tmpDir := t.TempDir()
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
	os.MkdirAll(mp1, 0o755)
	os.MkdirAll(mp2, 0o755)

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

// TestSafePrepareForRead tests preparing a device for read by unmounting duplicate mountpoints.
// This test requires root privileges and loop device support.
func TestSafePrepareForRead(t *testing.T) {
	if os.Getenv("CI") != "" || os.Getuid() != 0 {
		t.Skip("This test requires root and loop device support")
	}

	tmpDir := t.TempDir()
	imagePath := filepath.Join(tmpDir, "test.img")
	exec.Command("truncate", "-s", "64M", imagePath).Run()
	exec.Command("mkfs.vfat", imagePath).Run()

	out, _ := exec.Command("sudo", "losetup", "-f", "--show", imagePath).Output()
	loopDev := strings.TrimSpace(string(out))
	defer exec.Command("sudo", "losetup", "-d", loopDev).Run()

	mp1 := filepath.Join(tmpDir, "mnt1")
	mp2 := filepath.Join(tmpDir, "mnt2")
	os.MkdirAll(mp1, 0o755)
	os.MkdirAll(mp2, 0o755)

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

// TestAutoCleanupMounts tests automatic cleanup of duplicate mountpoints.
// This test requires root privileges and loop device support.
func TestAutoCleanupMounts(t *testing.T) {
	if os.Getenv("CI") != "" || os.Getuid() != 0 {
		t.Skip("This test requires root and loop device support")
	}

	tmpDir := t.TempDir()
	imagePath := filepath.Join(tmpDir, "test.img")
	exec.Command("truncate", "-s", "64M", imagePath).Run()
	exec.Command("mkfs.vfat", imagePath).Run()

	out, _ := exec.Command("sudo", "losetup", "-f", "--show", imagePath).Output()
	loopDev := strings.TrimSpace(string(out))
	defer exec.Command("sudo", "losetup", "-d", loopDev).Run()

	mp1 := filepath.Join(tmpDir, "mnt1")
	mp2 := filepath.Join(tmpDir, "mnt2")
	os.MkdirAll(mp1, 0o755)
	os.MkdirAll(mp2, 0o755)

	exec.Command("sudo", "mount", loopDev, mp1).Run()
	exec.Command("sudo", "mount", loopDev, mp2).Run()
	defer exec.Command("sudo", "umount", "-l", loopDev).Run()

	if err := AutoCleanupMounts(); err != nil {
		t.Fatalf("AutoCleanupMounts failed: %v", err)
	}

	// Verify that exactly one of them is still mounted for our loop device
	m1 := isMounted(mp1)
	m2 := isMounted(mp2)

	if m1 && m2 {
		t.Errorf("Both %s and %s are still mounted, should have cleaned up one", mp1, mp2)
	}
	if !m1 && !m2 {
		t.Errorf("Neither %s nor %s is mounted, should have kept one", mp1, mp2)
	}
}

// TestSafePrepareForReadRoot tests that root devices are skipped for safety.
func TestSafePrepareForReadRoot(t *testing.T) {
	tests := []struct {
		name    string
		device  models.BlockDevice
		wantErr bool
	}{
		{
			name: "root device with multiple mountpoints",
			device: models.BlockDevice{
				Name:        "rootdev",
				Mountpoints: []string{"/home", "/"},
				FSType:      "ext4",
			},
			wantErr: false, // Should skip silently
		},
		{
			name: "root device single mountpoint",
			device: models.BlockDevice{
				Name:        "rootdev2",
				Mountpoints: []string{"/"},
				FSType:      "ext4",
			},
			wantErr: false, // Should skip silently
		},
		{
			name: "non-root device with multiple mountpoints",
			device: models.BlockDevice{
				Name:        "datadev",
				Mountpoints: []string{"/mnt/data1", "/mnt/data2"},
				FSType:      "ext4",
			},
			wantErr: true, // Will fail because mountpoints don't exist in test environment
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := SafePrepareForRead(tt.device.Name, []models.BlockDevice{tt.device})
			if (err != nil) != tt.wantErr {
				t.Errorf("SafePrepareForRead(%q) error = %v, wantErr %v", tt.device.Name, err, tt.wantErr)
			}
		})
	}
}

// TestFilterMountpointsExcludesRoot tests that root filesystem is excluded from mountpoints.
func TestFilterMountpointsExcludesRoot(t *testing.T) {
	tests := []struct {
		name         string
		devices      []models.BlockDevice
		wantCount    int
		wantExcluded []string // device names that should be excluded
		wantIncluded []string // device names that should be included
	}{
		{
			name: "root device excluded",
			devices: []models.BlockDevice{
				{
					Name:        "sda1",
					Mountpoints: []string{"/"},
					Size:        "500G",
				},
				{
					Name:        "sdb1",
					Mountpoints: []string{"/mnt/data"},
					Size:        "1T",
				},
			},
			wantCount:    1,
			wantExcluded: []string{"sda1"},
			wantIncluded: []string{"sdb1"},
		},
		{
			name: "root device with children - children included",
			devices: []models.BlockDevice{
				{
					Name:        "sda",
					Mountpoints: []string{"/"},
					Size:        "500G",
					Children: []models.BlockDevice{
						{
							Name:        "sda1",
							Mountpoints: []string{"/boot"},
							Size:        "1G",
						},
					},
				},
			},
			wantCount:    1,
			wantExcluded: []string{"sda"},
			wantIncluded: []string{"sda1"},
		},
		{
			name: "multiple non-root devices",
			devices: []models.BlockDevice{
				{
					Name:        "sdb1",
					Mountpoints: []string{"/mnt/data1"},
					Size:        "1T",
				},
				{
					Name:        "sdc1",
					Mountpoints: []string{"/mnt/data2"},
					Size:        "2T",
				},
			},
			wantCount:    2,
			wantExcluded: []string{},
			wantIncluded: []string{"sdb1", "sdc1"},
		},
		{
			name: "empty mountpoints excluded",
			devices: []models.BlockDevice{
				{
					Name:        "sda1",
					Mountpoints: []string{""},
					Size:        "500G",
				},
				{
					Name:        "sdb1",
					Mountpoints: []string{"/mnt/data"},
					Size:        "1T",
				},
			},
			wantCount:    1,
			wantExcluded: []string{"sda1"},
			wantIncluded: []string{"sdb1"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			mounts := FilterMountpoints(tt.devices)

			// Check count
			if len(mounts) != tt.wantCount {
				t.Errorf("FilterMountpoints() returned %d mounts, want %d", len(mounts), tt.wantCount)
			}

			// Check excluded devices
			for _, excluded := range tt.wantExcluded {
				for _, m := range mounts {
					if m.Name == excluded {
						t.Errorf("FilterMountpoints() should exclude %q but it was included", excluded)
					}
				}
			}

			// Check included devices
			for _, included := range tt.wantIncluded {
				found := false
				for _, m := range mounts {
					if m.Name == included {
						found = true
						break
					}
				}
				if !found {
					t.Errorf("FilterMountpoints() should include %q but it was not found", included)
				}
			}
		})
	}
}

// TestParseLsblkOutput tests parsing of lsblk JSON output.
func TestParseLsblkOutput(t *testing.T) {
	tests := []struct {
		name    string
		input   string
		wantErr bool
		wantLen int
	}{
		{
			name: "valid lsblk output",
			input: `{
				"blockdevices": [
					{"name": "sda", "mountpoints": ["/"], "size": "500G", "type": "disk", "label": "", "fstype": "ext4"},
					{"name": "sdb", "mountpoints": ["/mnt/data"], "size": "1T", "type": "disk", "label": "DATA", "fstype": "ext4"}
				]
			}`,
			wantErr: false,
			wantLen: 2,
		},
		{
			name:    "empty blockdevices",
			input:   `{"blockdevices": []}`,
			wantErr: false,
			wantLen: 0,
		},
		{
			name:    "invalid JSON",
			input:   `{"blockdevices": [`,
			wantErr: true,
			wantLen: 0,
		},
		{
			name:    "missing blockdevices key",
			input:   `{}`,
			wantErr: false,
			wantLen: 0,
		},
		{
			name: "nested children",
			input: `{
				"blockdevices": [
					{
						"name": "sda",
						"mountpoints": ["/"],
						"size": "500G",
						"type": "disk",
						"label": "",
						"fstype": "ext4",
						"children": [
							{"name": "sda1", "mountpoints": ["/boot"], "size": "1G", "type": "part", "label": "", "fstype": "ext4"}
						]
					}
				]
			}`,
			wantErr: false,
			wantLen: 1,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			devices, err := ParseLsblkOutput([]byte(tt.input))
			if (err != nil) != tt.wantErr {
				t.Errorf("ParseLsblkOutput() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if len(devices) != tt.wantLen {
				t.Errorf("ParseLsblkOutput() returned %d devices, want %d", len(devices), tt.wantLen)
			}
		})
	}
}

// TestSafePrepareForReadBtrfs tests that Btrfs filesystems are skipped (thread-safe).
func TestSafePrepareForReadBtrfs(t *testing.T) {
	mockDevices := []models.BlockDevice{
		{
			Name:        "btrfsdev",
			Mountpoints: []string{"/mnt/btrfs1", "/mnt/btrfs2"},
			FSType:      "btrfs",
		},
	}

	err := SafePrepareForRead("btrfsdev", mockDevices)
	if err != nil {
		t.Errorf("SafePrepareForRead() should skip Btrfs devices, got error: %v", err)
	}
}

// TestSafePrepareForReadNotFound tests error handling when device is not found.
func TestSafePrepareForReadNotFound(t *testing.T) {
	mockDevices := []models.BlockDevice{
		{
			Name:        "existingdev",
			Mountpoints: []string{"/mnt/data"},
			FSType:      "ext4",
		},
	}

	err := SafePrepareForRead("nonexistent", mockDevices)
	if err == nil {
		t.Error("SafePrepareForRead() should return error for non-existent device")
	}
}

func isMounted(path string) bool {
	out, _ := exec.Command("mount").Output()
	return strings.Contains(string(out), path)
}
