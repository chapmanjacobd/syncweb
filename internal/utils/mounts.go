package utils

import (
	"encoding/json"
	"fmt"
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
	out, err := exec.Command("umount", mountpoint).CombinedOutput()
	if err != nil {
		return fmt.Errorf("unmount failed: %s: %w", string(out), err)
	}
	return nil
}
