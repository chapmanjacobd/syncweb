package commands

import (
	"encoding/json"
	"fmt"
	"sort"
	"strconv"
	"strings"

	"github.com/syncthing/syncthing/lib/config"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// Devices command examples
const devicesExamples = `
Examples:
  # Show all devices
  syncweb devices

  # Show only pending devices and accept them
  syncweb devices --pending --accept

  # Search for devices by name or ID
  syncweb devices -s server,backup

  # Exclude devices by pattern
  syncweb devices -E test,temp

  # Show transfer statistics (wait 5 seconds)
  syncweb devices --xfer

  # Configure device as introducer
  syncweb devices -s main-server --introducer

  # Pause matching devices
  syncweb devices -s test --pause
`

// SyncwebDevicesCmd lists Syncthing devices
type SyncwebDevicesCmd struct {
	Accepted   bool     `help:"Only show accepted devices"`
	Pending    bool     `help:"Only show pending devices"`
	Discovered bool     `help:"Only show discovered devices"`
	Accept     bool     `help:"Accept pending devices"`
	LocalOnly  bool     `help:"Only include local devices"`
	Include    []string `help:"Search for devices which match by name or ID" short:"s"`
	Exclude    []string `help:"Exclude devices which match by name or ID"    short:"E"`
	Introducer bool     `help:"Configure devices as introducers"`
	Pause      bool     `help:"Pause matching devices"`
	Resume     bool     `help:"Resume matching devices"`
	Print      bool     `help:"Print only device IDs"`
	Xfer       bool     `help:"Show transfer statistics"`
}

// Help displays examples for the devices command
func (c *SyncwebDevicesCmd) Help() string {
	return devicesExamples
}

func (c *SyncwebDevicesCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s syncweb.Engine) error {
		// If no filter specified, show all
		if !c.Accepted && !c.Pending && !c.Discovered {
			c.Accepted = true
			c.Pending = true
			c.Discovered = true
		}

		cfg := s.RawConfig()
		localDeviceID := s.MyID().String()

		// Collect all devices
		devices := c.collectDevices(s, &cfg, localDeviceID)

		// Apply filters
		filtered := c.filterDevices(devices)

		// Sort: localhost first, then connected, then by ID
		sort.Slice(filtered, func(i, j int) bool {
			if filtered[i].ID == localDeviceID {
				return true
			}
			if filtered[j].ID == localDeviceID {
				return false
			}
			if filtered[i].Connected != filtered[j].Connected {
				return filtered[i].Connected
			}
			return filtered[i].ID < filtered[j].ID
		})

		// Output
		if g.JSON {
			return c.outputJSON(filtered)
		}

		if c.Print {
			return c.outputPrint(filtered)
		}

		return c.outputTable(s, filtered, localDeviceID)
	})
}

type deviceEntry struct {
	ID         string  `json:"id"`
	Name       string  `json:"name"`
	Status     string  `json:"status"`
	LastSeen   int64   `json:"last_seen"`
	Duration   int     `json:"duration"`
	Bandwidth  string  `json:"bandwidth"`
	UL         float64 `json:"ul"`
	DL         float64 `json:"dl"`
	Connected  bool    `json:"connected"`
	Pending    bool    `json:"pending"`
	Discovered bool    `json:"discovered"`
	Paused     bool    `json:"paused"`
}

func (c *SyncwebDevicesCmd) collectDevices(
	s syncweb.Engine,
	cfg *config.Configuration,
	localDeviceID string,
) []deviceEntry {
	var devices []deviceEntry
	seenIDs := make(map[string]bool)

	// Get accepted devices
	if c.Accepted {
		devices = c.collectAcceptedDevices(devices, seenIDs, cfg, s, localDeviceID)
	}

	// Get pending devices
	if c.Pending {
		devices = c.collectPendingDevices(devices, seenIDs, s)
	}

	// Get discovered devices
	if c.Discovered {
		devices = c.collectDiscoveredDevices(devices, seenIDs, cfg, s)
	}

	return devices
}

func (c *SyncwebDevicesCmd) collectAcceptedDevices(
	devices []deviceEntry,
	seenIDs map[string]bool,
	cfg *config.Configuration,
	s syncweb.Engine,
	localDeviceID string,
) []deviceEntry {
	for _, d := range cfg.Devices {
		if seenIDs[d.DeviceID.String()] {
			continue
		}
		seenIDs[d.DeviceID.String()] = true
		devices = append(devices, c.buildAcceptedDeviceEntry(&d, s, localDeviceID))
	}
	return devices
}

func (c *SyncwebDevicesCmd) collectPendingDevices(
	devices []deviceEntry,
	seenIDs map[string]bool,
	s syncweb.Engine,
) []deviceEntry {
	pending := s.GetPendingDevices()
	for id := range pending {
		if seenIDs[id] {
			continue
		}
		seenIDs[id] = true
		devices = append(devices, deviceEntry{
			ID:      id,
			Name:    id[:7],
			Status:  "💬",
			Pending: true,
		})
	}
	return devices
}

func (c *SyncwebDevicesCmd) collectDiscoveredDevices(
	devices []deviceEntry,
	seenIDs map[string]bool,
	cfg *config.Configuration,
	s syncweb.Engine,
) []deviceEntry {
	discovered := s.GetDiscoveredDevices()
	for id := range discovered {
		if seenIDs[id] {
			continue
		}
		// Only show if not already in accepted list
		if c.isDeviceAccepted(id, cfg) {
			continue
		}

		name := id[:7]

		devices = append(devices, deviceEntry{
			ID:         id,
			Name:       name,
			Status:     "🗨️",
			Discovered: true,
		})
	}
	return devices
}

func (c *SyncwebDevicesCmd) isDeviceAccepted(id string, cfg *config.Configuration) bool {
	for _, d := range cfg.Devices {
		if d.DeviceID.String() == id {
			return true
		}
	}
	return false
}

func (c *SyncwebDevicesCmd) buildAcceptedDeviceEntry(
	d *config.DeviceConfiguration,
	s syncweb.Engine,
	localDeviceID string,
) deviceEntry {
	name := d.Name
	if name == "" || strings.EqualFold(name, "syncweb") || strings.EqualFold(name, "syncthing") {
		name = d.DeviceID.String()[:7]
	}

	// Use IsConnectedTo() for accurate online status
	status := "😴"
	if d.DeviceID.String() == localDeviceID {
		status = "🏠"
	} else if d.Paused {
		status = "⏸️"
	} else if s.IsConnectedTo(d.DeviceID) {
		status = "🌐"
	}

	return deviceEntry{
		ID:        d.DeviceID.String(),
		Name:      name,
		Status:    status,
		Paused:    d.Paused,
		Connected: s.IsConnectedTo(d.DeviceID),
		Bandwidth: formatBandwidth(d.MaxSendKbps, d.MaxRecvKbps),
	}
}

func (c *SyncwebDevicesCmd) filterDevices(devices []deviceEntry) []deviceEntry {
	if len(c.Include) == 0 && len(c.Exclude) == 0 {
		return devices
	}

	filtered := make([]deviceEntry, 0, len(devices))
	for _, d := range devices {
		// Include filter
		if len(c.Include) > 0 {
			matched := false
			for _, s := range c.Include {
				if strings.Contains(d.Name, s) || strings.Contains(d.ID, s) {
					matched = true
					break
				}
			}
			if !matched {
				continue
			}
		}

		// Exclude filter
		if len(c.Exclude) > 0 {
			excluded := false
			for _, s := range c.Exclude {
				if strings.Contains(d.Name, s) || strings.Contains(d.ID, s) {
					excluded = true
					break
				}
			}
			if excluded {
				continue
			}
		}

		filtered = append(filtered, d)
	}

	return filtered
}

func (c *SyncwebDevicesCmd) outputJSON(filtered []deviceEntry) error {
	data, err := json.MarshalIndent(filtered, "", "  ")
	if err != nil {
		return err
	}
	fmt.Println(string(data))
	return nil
}

func (c *SyncwebDevicesCmd) outputPrint(filtered []deviceEntry) error {
	for _, d := range filtered {
		fmt.Println(d.ID)
	}
	return nil
}

func (c *SyncwebDevicesCmd) outputTable(s syncweb.Engine, filtered []deviceEntry, localDeviceID string) error {
	// Print header
	if c.Xfer {
		fmt.Printf("%-63s  %-8s  %-22s  %-10s  %-25s  %s\n",
			"Device ID", "Name", "Last Seen", "Duration", "Bandwidth Limit", "Transfer Rate")
	} else {
		fmt.Printf("%-63s  %-8s  %-22s  %-10s  %s\n",
			"Device ID", "Name", "Last Seen", "Duration", "Bandwidth Limit")
	}
	fmt.Println(strings.Repeat("-", 150))

	// Print rows
	for i := range filtered {
		c.printDeviceRow(&filtered[i])
	}

	// Actions
	c.executeActions(s, filtered, localDeviceID)

	return nil
}

func (c *SyncwebDevicesCmd) printDeviceRow(d *deviceEntry) {
	lastSeen := formatLastSeen(d.LastSeen, d.Status)
	duration := "-"
	if d.Duration > 0 {
		duration = utils.FormatDurationShort(d.Duration)
	}

	bw := d.Bandwidth
	xfer := "-"
	if c.Xfer && d.Connected {
		xfer = fmt.Sprintf("↑%.1f KB/s  ↓%.1f KB/s", d.UL, d.DL)
	}

	if c.Xfer {
		fmt.Printf("%-63s  %-8s  %-22s  %-10s  %-25s  %s\n",
			d.ID, d.Name, lastSeen, duration, bw, xfer)
	} else {
		fmt.Printf("%-63s  %-8s  %-22s  %-10s  %s\n",
			d.ID, d.Name, lastSeen, duration, bw)
	}
}

func (c *SyncwebDevicesCmd) executeActions(s syncweb.Engine, filtered []deviceEntry, localDeviceID string) {
	if c.Accept {
		c.actionAccept(s, filtered)
	}

	if c.Pause {
		c.actionPause(s, filtered, localDeviceID)
	}

	if c.Resume {
		c.actionResume(s, filtered)
	}
}

func (c *SyncwebDevicesCmd) actionAccept(s syncweb.Engine, filtered []deviceEntry) {
	var toAccept []string
	for _, d := range filtered {
		if d.Pending {
			toAccept = append(toAccept, d.ID)
		}
	}
	if len(toAccept) > 0 {
		for _, id := range toAccept {
			if err := s.AddDevice(id, "", c.Introducer); err != nil {
				fmt.Printf("Error accepting %s: %v\n", id, err)
			}
		}
		fmt.Printf("Accepted %d %s\n", len(toAccept), utils.Pluralize(len(toAccept), "device", "devices"))
	}
}

func (c *SyncwebDevicesCmd) actionPause(s syncweb.Engine, filtered []deviceEntry, localDeviceID string) {
	count := 0
	for _, d := range filtered {
		if !d.Paused && d.ID != localDeviceID {
			if err := s.PauseDevice(d.ID); err == nil {
				count++
			}
		}
	}
	fmt.Printf("Paused %d %s\n", count, utils.Pluralize(count, "device", "devices"))
}

func (c *SyncwebDevicesCmd) actionResume(s syncweb.Engine, filtered []deviceEntry) {
	count := 0
	for _, d := range filtered {
		if d.Paused {
			if err := s.ResumeDevice(d.ID); err == nil {
				count++
			}
		}
	}
	fmt.Printf("Resumed %d %s\n", count, utils.Pluralize(count, "device", "devices"))
}

func formatBandwidth(sendKbps, recvKbps int) string {
	if sendKbps == 0 && recvKbps == 0 {
		return "Unlimited"
	}
	sendStr := "∞"
	recvStr := "∞"
	if sendKbps > 0 {
		sendStr = strconv.Itoa(sendKbps)
	}
	if recvKbps > 0 {
		recvStr = strconv.Itoa(recvKbps)
	}
	return fmt.Sprintf("↑%s/↓%s Kbps", sendStr, recvStr)
}

func formatLastSeen(timestamp int64, status string) string {
	if status == "🏠" {
		return "🏠"
	}
	if timestamp == 0 {
		if status == "💬" {
			return "pending"
		}
		return "never"
	}
	return utils.RelativeTime(timestamp)
}
