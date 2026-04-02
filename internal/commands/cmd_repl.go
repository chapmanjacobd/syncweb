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
		c.printFolders(s)
	case "devices", "lsd":
		c.printDevices(s)
	case "pending", "pending-devices":
		c.printPendingDevices(s)
	case "events":
		c.printEvents(s)
	case "stats", "folder-stats":
		c.printFolderStats(s)
	case "device-stats":
		c.printDeviceStats(s)
	case "ignores":
		return c.handleIgnores(s, args)
	case "set-ignores":
		return c.handleSetIgnores(s, args)
	case "add-device":
		return c.handleAddDevice(s, args)
	case "add-folder":
		return c.handleAddFolder(s, args)
	case "pause-folder":
		return c.handlePauseFolder(s, args)
	case "resume-folder":
		return c.handleResumeFolder(s, args)
	case "pause-device":
		return c.handlePauseDevice(s, args)
	case "resume-device":
		return c.handleResumeDevice(s, args)
	case "delete-folder":
		return c.handleDeleteFolder(s, args)
	case "delete-device":
		return c.handleDeleteDevice(s, args)
	case "ls", "list":
		c.handleList(args)
	case "whoami":
		c.printWhoami(s)
	default:
		fmt.Printf("Unknown command: %s\n", cmd)
		c.printHelp()
	}

	return nil
}

// printFolders prints the list of folders
func (c *SyncwebReplCmd) printFolders(s *syncweb.Syncweb) {
	folders := s.GetFolders()
	for _, f := range folders {
		fmt.Printf("%s (%s): %s [type=%s, paused=%v]\n", f.ID, f.Label, f.Path, f.Type, f.Paused)
		for _, dev := range f.Devices {
			fmt.Printf("  - device: %s\n", dev)
		}
	}
}

// printDevices prints the list of devices
func (c *SyncwebReplCmd) printDevices(s *syncweb.Syncweb) {
	devices := s.GetDevices()
	for _, d := range devices {
		fmt.Printf("%s (%s): %v [paused=%v]\n", d.ID, d.Name, d.Addresses, d.Paused)
	}
}

// printPendingDevices prints pending devices
func (c *SyncwebReplCmd) printPendingDevices(s *syncweb.Syncweb) {
	pending := s.GetPendingDevices()
	if len(pending) == 0 {
		fmt.Println("No pending devices")
	} else {
		for id, t := range pending {
			fmt.Printf("%s (since %v)\n", id, t)
		}
	}
}

// printEvents prints recent events
func (c *SyncwebReplCmd) printEvents(s *syncweb.Syncweb) {
	events := s.GetEvents()
	for _, ev := range events {
		fmt.Printf("%s: %s\n", ev.Type, ev.Message)
	}
}

// printFolderStats prints folder statistics
func (c *SyncwebReplCmd) printFolderStats(s *syncweb.Syncweb) {
	stats := s.GetFolderStats()
	for id, stat := range stats {
		fmt.Printf("%s: %v\n", id, stat)
	}
}

// printDeviceStats prints device statistics
func (c *SyncwebReplCmd) printDeviceStats(s *syncweb.Syncweb) {
	stats := s.GetDeviceStats()
	for id, stat := range stats {
		fmt.Printf("%s: %v\n", id, stat)
	}
}

// handleIgnores handles the ignores command
func (c *SyncwebReplCmd) handleIgnores(s *syncweb.Syncweb, args []string) error {
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
	return nil
}

// handleSetIgnores handles the set-ignores command
func (c *SyncwebReplCmd) handleSetIgnores(s *syncweb.Syncweb, args []string) error {
	if len(args) < 2 {
		return errors.New("usage: set-ignores <folder-id> <pattern1> [pattern2]")
	}
	folderID := args[0]
	patterns := args[1:]
	if err := s.SetIgnores(folderID, patterns); err != nil {
		return err
	}
	fmt.Printf("Set %d ignore patterns for folder %s\n", len(patterns), folderID)
	return nil
}

// handleAddDevice handles the add-device command
func (c *SyncwebReplCmd) handleAddDevice(s *syncweb.Syncweb, args []string) error {
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
	return nil
}

// handleAddFolder handles the add-folder command
func (c *SyncwebReplCmd) handleAddFolder(s *syncweb.Syncweb, args []string) error {
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
	return nil
}

// handlePauseFolder handles the pause-folder command
func (c *SyncwebReplCmd) handlePauseFolder(s *syncweb.Syncweb, args []string) error {
	if len(args) < 1 {
		return errors.New("usage: pause-folder <folder-id>")
	}
	if err := s.PauseFolder(args[0]); err != nil {
		return err
	}
	fmt.Printf("Paused folder %s\n", args[0])
	return nil
}

// handleResumeFolder handles the resume-folder command
func (c *SyncwebReplCmd) handleResumeFolder(s *syncweb.Syncweb, args []string) error {
	if len(args) < 1 {
		return errors.New("usage: resume-folder <folder-id>")
	}
	if err := s.ResumeFolder(args[0]); err != nil {
		return err
	}
	fmt.Printf("Resumed folder %s\n", args[0])
	return nil
}

// handlePauseDevice handles the pause-device command
func (c *SyncwebReplCmd) handlePauseDevice(s *syncweb.Syncweb, args []string) error {
	if len(args) < 1 {
		return errors.New("usage: pause-device <device-id>")
	}
	if err := s.PauseDevice(args[0]); err != nil {
		return err
	}
	fmt.Printf("Paused device %s\n", args[0])
	return nil
}

// handleResumeDevice handles the resume-device command
func (c *SyncwebReplCmd) handleResumeDevice(s *syncweb.Syncweb, args []string) error {
	if len(args) < 1 {
		return errors.New("usage: resume-device <device-id>")
	}
	if err := s.ResumeDevice(args[0]); err != nil {
		return err
	}
	fmt.Printf("Resumed device %s\n", args[0])
	return nil
}

// handleDeleteFolder handles the delete-folder command
func (c *SyncwebReplCmd) handleDeleteFolder(s *syncweb.Syncweb, args []string) error {
	if len(args) < 1 {
		return errors.New("usage: delete-folder <folder-id>")
	}
	if err := s.DeleteFolder(args[0]); err != nil {
		return err
	}
	fmt.Printf("Deleted folder %s\n", args[0])
	return nil
}

// handleDeleteDevice handles the delete-device command
func (c *SyncwebReplCmd) handleDeleteDevice(s *syncweb.Syncweb, args []string) error {
	if len(args) < 1 {
		return errors.New("usage: delete-device <device-id>")
	}
	if err := s.DeleteDevice(args[0]); err != nil {
		return err
	}
	fmt.Printf("Deleted device %s\n", args[0])
	return nil
}

// handleList handles the ls command
func (c *SyncwebReplCmd) handleList(args []string) {
	path := "."
	if len(args) > 0 {
		path = args[0]
	}
	fmt.Printf("Listing: %s (use 'syncweb ls %s' for full output)\n", path, path)
}

// printWhoami prints current node information
func (c *SyncwebReplCmd) printWhoami(s *syncweb.Syncweb) {
	fmt.Printf("Node ID: %s\n", s.Node.MyID())
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
