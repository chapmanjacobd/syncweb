package commands

import (
	"bufio"
	"errors"
	"fmt"
	"os"
	"strings"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
)

// Repl command examples
const replExamples = `
Examples:
  # Start interactive REPL
  syncweb repl

  # Available REPL commands:
  #   folders, lsf       - List folders
  #   devices, lsd       - List devices
  #   pending            - List pending devices
  #   events             - Show recent events
  #   stats              - Show folder statistics
  #   ignores <folder>   - Show ignore patterns
  #   add-device <id>    - Add a device
  #   pause-folder <id>  - Pause a folder
  #   whoami             - Show node info
  #   exit, quit, q      - Exit REPL
`

// SyncwebReplCmd provides an interactive REPL for debugging
type SyncwebReplCmd struct{}

// Help displays examples for the repl command
func (c *SyncwebReplCmd) Help() string {
	return replExamples
}

func (c *SyncwebReplCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		fmt.Println("Syncweb REPL - Interactive Debug Mode")
		fmt.Println("Type 'help' for available commands, 'exit' to quit")
		fmt.Println()

		reader := bufio.NewReader(os.Stdin)

		for {
			fmt.Print("syncweb> ")
			input, err := reader.ReadString('\n')
			if err != nil {
				return err
			}

			input = strings.TrimSpace(input)
			if input == "" {
				continue
			}

			if input == "exit" || input == "quit" || input == "q" {
				fmt.Println("Exiting REPL")
				break
			}

			if err := c.executeCommand(input, s); err != nil {
				fmt.Printf("Error: %v\n", err)
			}
		}

		return nil
	})
}

//nolint:maintidx // REPL command interpreter with many subcommands
func (c *SyncwebReplCmd) executeCommand(input string, s *syncweb.Syncweb) error {
	parts := strings.Fields(input)
	if len(parts) == 0 {
		return nil
	}

	cmd := parts[0]
	args := parts[1:]

	switch cmd {
	case "help", "h", "?":
		c.printHelp()

	case "folders", "lsf":
		folders := s.GetFolders()
		for _, f := range folders {
			fmt.Printf("%s (%s): %s [type=%s, paused=%v]\n", f.ID, f.Label, f.Path, f.Type, f.Paused)
			for _, dev := range f.Devices {
				fmt.Printf("  - device: %s\n", dev)
			}
		}

	case "devices", "lsd":
		devices := s.GetDevices()
		for _, d := range devices {
			fmt.Printf("%s (%s): %v [paused=%v]\n", d.ID, d.Name, d.Addresses, d.Paused)
		}

	case "pending", "pending-devices":
		pending := s.GetPendingDevices()
		if len(pending) == 0 {
			fmt.Println("No pending devices")
		} else {
			for id, t := range pending {
				fmt.Printf("%s (since %v)\n", id, t)
			}
		}

	case "events":
		events := s.GetEvents()
		for _, ev := range events {
			fmt.Printf("%s: %s\n", ev.Type, ev.Message)
		}

	case "stats", "folder-stats":
		stats := s.GetFolderStats()
		for id, stat := range stats {
			fmt.Printf("%s: %v\n", id, stat)
		}

	case "device-stats":
		stats := s.GetDeviceStats()
		for id, stat := range stats {
			fmt.Printf("%s: %v\n", id, stat)
		}

	case "ignores":
		if len(args) < 1 {
			return errors.New("usage: ignores <folder-id>")
		}
		folderID := args[0]
		lines, err := s.GetIgnores(folderID)
		if err != nil {
			return err
		}
		for _, line := range lines {
			fmt.Println(line)
		}

	case "set-ignores":
		if len(args) < 2 {
			return errors.New("usage: set-ignores <folder-id> <pattern1> [pattern2]")
		}
		folderID := args[0]
		patterns := args[1:]
		if err := s.SetIgnores(folderID, patterns); err != nil {
			return err
		}
		fmt.Printf("Set %d ignore patterns for folder %s\n", len(patterns), folderID)

	case "add-device":
		if len(args) < 1 {
			return errors.New("usage: add-device <device-id> [name]")
		}
		deviceID := args[0]
		name := ""
		if len(args) > 1 {
			name = args[1]
		}
		if err := s.AddDevice(deviceID, name, false); err != nil {
			return err
		}
		fmt.Printf("Added device %s\n", deviceID)

	case "add-folder":
		if len(args) < 3 {
			return errors.New("usage: add-folder <id> <label> <path>")
		}
		id := args[0]
		label := args[1]
		path := args[2]
		if err := s.AddFolder(id, label, path, 0); err != nil {
			return err
		}
		fmt.Printf("Added folder %s (%s) at %s\n", id, label, path)

	case "pause-folder":
		if len(args) < 1 {
			return errors.New("usage: pause-folder <folder-id>")
		}
		if err := s.PauseFolder(args[0]); err != nil {
			return err
		}
		fmt.Printf("Paused folder %s\n", args[0])

	case "resume-folder":
		if len(args) < 1 {
			return errors.New("usage: resume-folder <folder-id>")
		}
		if err := s.ResumeFolder(args[0]); err != nil {
			return err
		}
		fmt.Printf("Resumed folder %s\n", args[0])

	case "pause-device":
		if len(args) < 1 {
			return errors.New("usage: pause-device <device-id>")
		}
		if err := s.PauseDevice(args[0]); err != nil {
			return err
		}
		fmt.Printf("Paused device %s\n", args[0])

	case "resume-device":
		if len(args) < 1 {
			return errors.New("usage: resume-device <device-id>")
		}
		if err := s.ResumeDevice(args[0]); err != nil {
			return err
		}
		fmt.Printf("Resumed device %s\n", args[0])

	case "delete-folder":
		if len(args) < 1 {
			return errors.New("usage: delete-folder <folder-id>")
		}
		if err := s.DeleteFolder(args[0]); err != nil {
			return err
		}
		fmt.Printf("Deleted folder %s\n", args[0])

	case "delete-device":
		if len(args) < 1 {
			return errors.New("usage: delete-device <device-id>")
		}
		if err := s.DeleteDevice(args[0]); err != nil {
			return err
		}
		fmt.Printf("Deleted device %s\n", args[0])

	case "ls", "list":
		// Delegate to ls command logic
		path := "."
		if len(args) > 0 {
			path = args[0]
		}
		fmt.Printf("Listing: %s (use 'syncweb ls %s' for full output)\n", path, path)

	case "whoami":
		fmt.Printf("Node ID: %s\n", s.Node.MyID())

	default:
		fmt.Printf("Unknown command: %s\n", cmd)
		c.printHelp()
	}

	return nil
}

func (c *SyncwebReplCmd) printHelp() {
	fmt.Println("Available commands:")
	fmt.Println("  folders, lsf          - List folders")
	fmt.Println("  devices, lsd          - List devices")
	fmt.Println("  pending               - List pending devices")
	fmt.Println("  events                - Show recent events")
	fmt.Println("  stats                 - Show folder statistics")
	fmt.Println("  device-stats          - Show device statistics")
	fmt.Println("  ignores <folder>      - Show ignore patterns")
	fmt.Println("  set-ignores <folder> <patterns> - Set ignore patterns")
	fmt.Println("  add-device <id> [name] - Add a device")
	fmt.Println("  add-folder <id> <label> <path> - Add a folder")
	fmt.Println("  pause-folder <id>     - Pause a folder")
	fmt.Println("  resume-folder <id>    - Resume a folder")
	fmt.Println("  pause-device <id>     - Pause a device")
	fmt.Println("  resume-device <id>    - Resume a device")
	fmt.Println("  delete-folder <id>    - Delete a folder")
	fmt.Println("  delete-device <id>    - Delete a device")
	fmt.Println("  whoami                - Show current node info")
	fmt.Println("  exit, quit, q         - Exit REPL")
}
