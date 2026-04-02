package utils

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"slices"
	"strings"

	"github.com/chapmanjacobd/syncweb/internal/models"
)

func GetMountpoints(ctx context.Context) ([]models.Mountpoint, error) {
	devices, err := GetBlockDevices(ctx)
	if err != nil {
		return nil, err
	}
	return FilterMountpoints(devices), nil
}

func FilterMountpoints(devices []models.BlockDevice) []models.Mountpoint {
	var mounts []models.Mountpoint
	var flatten func([]models.BlockDevice)
	flatten = func(devs []models.BlockDevice) {
		for _, d := range devs {
			// Skip devices that include the root filesystem
			isRootDevice := slices.Contains(d.Mountpoints, "/")
			if isRootDevice {
				if len(d.Children) > 0 {
					flatten(d.Children)
				}
				continue
			}

			if len(d.Mountpoints) > 0 {
				hasRealMount := false
				for _, mp := range d.Mountpoints {
					if mp != "" && !strings.HasPrefix(mp, "[") {
						hasRealMount = true
						break
					}
				}
				if hasRealMount {
					mounts = append(mounts, models.Mountpoint{
						Name:        d.Name,
						Mountpoints: d.Mountpoints,
						Size:        d.Size,
						Type:        d.Type,
						Label:       d.Label,
						FSType:      d.FSType,
					})
				}
			}
			if len(d.Children) > 0 {
				flatten(d.Children)
			}
		}
	}

	flatten(devices)
	return mounts
}

func GetBlockDevices(ctx context.Context) ([]models.BlockDevice, error) {
	out, err := exec.CommandContext(ctx, "lsblk", "--json", "-o", "NAME,MOUNTPOINTS,SIZE,TYPE,LABEL,FSTYPE").
		Output()
	if err != nil {
		return nil, fmt.Errorf("lsblk failed: %w", err)
	}
	return ParseLsblkOutput(out)
}

func ParseLsblkOutput(data []byte) ([]models.BlockDevice, error) {
	var res struct {
		Blockdevices []models.BlockDevice `json:"blockdevices"`
	}
	if err := json.Unmarshal(data, &res); err != nil {
		return nil, fmt.Errorf("failed to unmarshal lsblk output: %w", err)
	}
	return res.Blockdevices, nil
}

func Mount(ctx context.Context, device, mountpoint string) error {
	out, err := exec.CommandContext(ctx, "mount", device, mountpoint).CombinedOutput()
	if err != nil {
		return fmt.Errorf("mount failed: %s: %w", string(out), err)
	}
	return nil
}

func Unmount(ctx context.Context, mountpoint string) error {
	// Find the device for this mountpoint
	devices, err := GetBlockDevices(ctx)
	if err != nil {
		return err
	}

	var targetDevice *models.BlockDevice
	var findDevice func([]models.BlockDevice)
	findDevice = func(devs []models.BlockDevice) {
		for _, d := range devs {
			if slices.Contains(d.Mountpoints, mountpoint) {
				targetDevice = &d
				return
			}
			if len(d.Children) > 0 {
				findDevice(d.Children)
			}
		}
	}
	findDevice(devices)

	if targetDevice == nil {
		// Fallback to simple unmount if device not found in lsblk
		out, umountErr := exec.CommandContext(ctx, "sudo", "umount", mountpoint).CombinedOutput()
		if umountErr != nil {
			return fmt.Errorf("unmount failed: %s: %w", string(out), umountErr)
		}
		return nil
	}

	if slices.Contains(targetDevice.Mountpoints, "/") {
		return errors.New("cannot unmount root filesystem")
	}

	// Unmount ALL mountpoints for this device
	for _, mp := range targetDevice.Mountpoints {
		if mp == "" || strings.HasPrefix(mp, "[") {
			continue
		}
		out, umountErr := exec.CommandContext(ctx, "sudo", "umount", mp).CombinedOutput()
		if umountErr != nil {
			return fmt.Errorf("failed to unmount %s: %s: %w", mp, string(out), umountErr)
		}
	}

	return nil
}

func AutoCleanupMounts(ctx context.Context) error {
	devices, err := GetBlockDevices(ctx)
	if err != nil {
		return err
	}

	var walk func([]models.BlockDevice)
	//nolint:contextcheck // walk is a closure that doesn't directly use context
	walk = func(devs []models.BlockDevice) {
		for _, d := range devs {
			if len(d.Mountpoints) > 1 {
				// Potential duplicates found, unmount them safely
				if cleanupErr := SafePrepareForRead(context.Background(), d.Name, devices); cleanupErr != nil {
					fmt.Printf("Warning: failed to cleanup mounts for %s: %v\n", d.Name, cleanupErr)
				}
			}
			if len(d.Children) > 0 {
				walk(d.Children)
			}
		}
	}

	walk(devices)
	return nil
}

func GetFstabMounts() (map[string]bool, error) {
	data, err := os.ReadFile("/etc/fstab")
	if err != nil {
		return nil, err
	}

	res := make(map[string]bool)
	lines := strings.SplitSeq(string(data), "\n")
	for line := range lines {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		fields := strings.Fields(line)
		if len(fields) >= 2 {
			res[fields[1]] = true
		}
	}
	return res, nil
}

func IsUdisks2Mount(path string) bool {
	return strings.HasPrefix(path, "/run/media/") || strings.HasPrefix(path, "/media/")
}

func SafePrepareForRead(ctx context.Context, deviceName string, optionalDevices []models.BlockDevice) error {
	devices, err := getDevicesForSearch(ctx, optionalDevices)
	if err != nil {
		return err
	}

	// Find our target device
	target := findDevice(deviceName, devices)
	if target == nil {
		return fmt.Errorf("device %s not found", deviceName)
	}

	// Skip if root device (safety)
	if slices.Contains(target.Mountpoints, "/") {
		return nil
	}

	// Skip if thread-safe (Btrfs)
	if target.FSType == "btrfs" {
		return nil
	}

	// Identify preferred mountpoint
	preferred := findPreferredMountpoint(target.Mountpoints)
	if preferred == "" {
		return nil
	}

	// Unmount others
	return unmountExtraMountpoints(ctx, target.Mountpoints, preferred)
}

func getDevicesForSearch(ctx context.Context, optionalDevices []models.BlockDevice) ([]models.BlockDevice, error) {
	if len(optionalDevices) > 0 {
		return optionalDevices, nil
	}
	return GetBlockDevices(ctx)
}

func findDevice(deviceName string, devs []models.BlockDevice) *models.BlockDevice {
	var target *models.BlockDevice
	var find func([]models.BlockDevice)
	find = func(devs []models.BlockDevice) {
		for _, d := range devs {
			if d.Name == deviceName || "/dev/"+d.Name == deviceName {
				target = &d
				return
			}
			if len(d.Children) > 0 {
				find(d.Children)
			}
		}
	}
	find(devs)
	return target
}

func findPreferredMountpoint(mountpoints []string) string {
	if len(mountpoints) <= 1 {
		return ""
	}

	fstab, _ := GetFstabMounts()

	// Priority 1: fstab
	for _, mp := range mountpoints {
		if fstab[mp] {
			return mp
		}
	}

	// Priority 2: udisks2
	for _, mp := range mountpoints {
		if IsUdisks2Mount(mp) {
			return mp
		}
	}

	// Priority 3: first available
	return mountpoints[0]
}

func unmountExtraMountpoints(ctx context.Context, mountpoints []string, preferred string) error {
	for _, mp := range mountpoints {
		if mp == preferred || mp == "" || strings.HasPrefix(mp, "[") {
			continue
		}
		if mp == "/" {
			continue // Safety
		}
		out, umountErr := exec.CommandContext(ctx, "sudo", "umount", mp).CombinedOutput()
		if umountErr != nil {
			return fmt.Errorf("failed to unmount extra mountpoint %s: %s: %w", mp, string(out), umountErr)
		}
	}
	return nil
}
