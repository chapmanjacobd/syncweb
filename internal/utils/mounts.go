package utils

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"strings"

	"github.com/chapmanjacobd/syncweb/internal/models"
)

func GetMountpoints() ([]models.Mountpoint, error) {
	out, err := exec.Command("lsblk", "--json", "-o", "NAME,MOUNTPOINTS,SIZE,TYPE,LABEL,FSTYPE").Output()
	if err != nil {
		return nil, fmt.Errorf("lsblk failed: %w", err)
	}

	var res struct {
		Blockdevices []models.BlockDevice `json:"blockdevices"`
	}
	if err := json.Unmarshal(out, &res); err != nil {
		return nil, fmt.Errorf("failed to unmarshal lsblk output: %w", err)
	}

	var mounts []models.Mountpoint
	var flatten func([]models.BlockDevice)
	flatten = func(devices []models.BlockDevice) {
		for _, d := range devices {
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

	flatten(res.Blockdevices)
	return mounts, nil
}

func GetBlockDevices() ([]models.BlockDevice, error) {
	out, err := exec.Command("lsblk", "--json", "-o", "NAME,MOUNTPOINTS,SIZE,TYPE,LABEL,FSTYPE").Output()
	if err != nil {
		return nil, fmt.Errorf("lsblk failed: %w", err)
	}

	var res struct {
		Blockdevices []models.BlockDevice `json:"blockdevices"`
	}
	if err := json.Unmarshal(out, &res); err != nil {
		return nil, fmt.Errorf("failed to unmarshal lsblk output: %w", err)
	}

	return res.Blockdevices, nil
}

func Mount(device string, mountpoint string) error {
	out, err := exec.Command("mount", device, mountpoint).CombinedOutput()
	if err != nil {
		return fmt.Errorf("mount failed: %s: %w", string(out), err)
	}
	return nil
}

func Unmount(mountpoint string) error {
	// Find the device for this mountpoint
	devices, err := GetBlockDevices()
	if err != nil {
		return err
	}

	var targetDevice *models.BlockDevice
	var findDevice func([]models.BlockDevice)
	findDevice = func(devs []models.BlockDevice) {
		for _, d := range devs {
			for _, mp := range d.Mountpoints {
				if mp == mountpoint {
					targetDevice = &d
					return
				}
			}
			if len(d.Children) > 0 {
				findDevice(d.Children)
			}
		}
	}
	findDevice(devices)

	if targetDevice == nil {
		// Fallback to simple unmount if device not found in lsblk
		out, err := exec.Command("sudo", "umount", mountpoint).CombinedOutput()
		if err != nil {
			return fmt.Errorf("unmount failed: %s: %w", string(out), err)
		}
		return nil
	}

	// If it's a removable device or has multiple mountpoints, we might want to unmount all
	// But according to requirements: unmount all points when unmounting a removable device.
	// For now let's identify all mountpoints for this device.
	
	for _, mp := range targetDevice.Mountpoints {
		if mp == "/" {
			return fmt.Errorf("cannot unmount root filesystem")
		}
	}

	// Unmount ALL mountpoints for this device
	for _, mp := range targetDevice.Mountpoints {
		if mp == "" || strings.HasPrefix(mp, "[") {
			continue
		}
		out, err := exec.Command("sudo", "umount", mp).CombinedOutput()
		if err != nil {
			return fmt.Errorf("failed to unmount %s: %s: %w", mp, string(out), err)
		}
	}

	return nil
}

func GetFstabMounts() (map[string]bool, error) {
	data, err := os.ReadFile("/etc/fstab")
	if err != nil {
		return nil, err
	}
	
	res := make(map[string]bool)
	lines := strings.Split(string(data), "\n")
	for _, line := range lines {
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

func SafePrepareForRead(deviceName string) error {
	// 1. Get all block devices
	devices, err := GetBlockDevices()
	if err != nil {
		return err
	}

	// 2. Find our target device
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
	find(devices)

	if target == nil {
		return fmt.Errorf("device %s not found", deviceName)
	}

	// 3. Skip if thread-safe (Btrfs)
	if target.FSType == "btrfs" {
		return nil
	}

	// 4. Identify preferred mountpoint (fstab > udisks2 > others)
	if len(target.Mountpoints) <= 1 {
		return nil
	}

	fstab, _ := GetFstabMounts()
	
	var preferred string
	for _, mp := range target.Mountpoints {
		if fstab[mp] {
			preferred = mp
			break
		}
	}
	if preferred == "" {
		for _, mp := range target.Mountpoints {
			if IsUdisks2Mount(mp) {
				preferred = mp
				break
			}
		}
	}
	if preferred == "" {
		preferred = target.Mountpoints[0]
	}

	// 5. Unmount others
	for _, mp := range target.Mountpoints {
		if mp == preferred || mp == "" || strings.HasPrefix(mp, "[") {
			continue
		}
		if mp == "/" {
			continue // Safety
		}
		out, err := exec.Command("sudo", "umount", mp).CombinedOutput()
		if err != nil {
			return fmt.Errorf("failed to unmount extra mountpoint %s: %s: %w", mp, string(out), err)
		}
	}

	return nil
}

