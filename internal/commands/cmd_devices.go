package commands

import (
	"encoding/json"
	"fmt"
	"sort"
	"strconv"
	"strings"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

// SyncwebDevicesCmd lists Syncthing devices.
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

//nolint:maintidx // CLI command with many flags is inherently complex
func (c *SyncwebDevicesCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		// If no filter specified, show all
		if !c.Accepted && !c.Pending && !c.Discovered {
			c.Accepted = true
			c.Pending = true
			c.Discovered = true
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

		var devices []deviceEntry
		seenIDs := make(map[string]bool)

		cfg := s.Node.Cfg.RawCopy()
		localDeviceID := s.Node.MyID().String()

		// Get accepted devices
		if c.Accepted {
			for _, d := range cfg.Devices {
				if seenIDs[d.DeviceID.String()] {
					continue
				}
				seenIDs[d.DeviceID.String()] = true

				name := d.Name
				if name == "" || strings.ToLower(name) == "syncweb" || strings.ToLower(name) == "syncthing" {
					name = d.DeviceID.String()[:7]
				}

				// Use IsConnectedTo() for accurate online status
				status := "😴"
				if d.DeviceID.String() == localDeviceID {
					status = "🏠"
				} else if d.Paused {
					status = "⏸️"
				} else if s.Node.App.Internals.IsConnectedTo(d.DeviceID) {
					status = "🌐"
				}

				entry := deviceEntry{
					ID:        d.DeviceID.String(),
					Name:      name,
					Status:    status,
					Paused:    d.Paused,
					Connected: s.Node.App.Internals.IsConnectedTo(d.DeviceID),
					Bandwidth: formatBandwidth(d.MaxSendKbps, d.MaxRecvKbps),
				}

				devices = append(devices, entry)
			}
		}

		// Get pending devices
		if c.Pending {
			pending := s.GetPendingDevices()
			for id := range pending {
				if seenIDs[id] {
					continue
				}
				seenIDs[id] = true

				name := id[:7]
				devices = append(devices, deviceEntry{
					ID:      id,
					Name:    name,
					Status:  "💬",
					Pending: true,
				})
			}
		}

		// Get discovered devices
		if c.Discovered {
			discovered := s.GetDiscoveredDevices()
			for id, info := range discovered {
				if seenIDs[id] {
					continue
				}
				// Only show if not already in accepted list
				alreadyAccepted := false
				for _, d := range cfg.Devices {
					if d.DeviceID.String() == id {
						alreadyAccepted = true
						break
					}
				}
				if alreadyAccepted {
					continue
				}

				name, _ := info["name"].(string)
				if name == "" {
					name = id[:7]
				}

				devices = append(devices, deviceEntry{
					ID:         id,
					Name:       name,
					Status:     "🗨️",
					Discovered: true,
				})
			}
		}

		// Apply filters
		filtered := []deviceEntry{}
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

		if g.JSON {
			data, err := json.MarshalIndent(filtered, "", "  ")
			if err != nil {
				return err
			}
			fmt.Println(string(data))
			return nil
		}

		if c.Print {
			for _, d := range filtered {
				fmt.Println(d.ID)
			}
			return nil
		}

		// Print table
		if c.Xfer {
			fmt.Printf("%-63s  %-8s  %-22s  %-10s  %-25s  %s\n",
				"Device ID", "Name", "Last Seen", "Duration", "Bandwidth Limit", "Transfer Rate")
		} else {
			fmt.Printf("%-63s  %-8s  %-22s  %-10s  %s\n",
				"Device ID", "Name", "Last Seen", "Duration", "Bandwidth Limit")
		}
		fmt.Println(strings.Repeat("-", 150))

		for _, d := range filtered {
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

		// Actions
		if c.Accept {
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

		if c.Pause {
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
		if c.Resume {
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

		return nil
	})
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
