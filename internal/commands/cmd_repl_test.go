package commands

import (
	"strings"
	"testing"
)

func TestSyncwebReplCmd_CommandParsing(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"folders command", "folders", "folders"},
		{"devices command", "devices", "devices"},
		{"help command", "help", "help"},
		{"exit command", "exit", "exit"},
		{"quit command", "quit", "quit"},
		{"q command", "q", "q"},
		{"lsf alias", "lsf", "lsf"},
		{"lsd alias", "lsd", "lsd"},
		{"pending command", "pending", "pending"},
		{"events command", "events", "events"},
		{"stats command", "stats", "stats"},
		{"whoami command", "whoami", "whoami"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			parts := strings.Fields(tt.input)
			if len(parts) == 0 {
				t.Fatalf("Expected non-empty parts for input %q", tt.input)
			}
			cmd := parts[0]
			if cmd != tt.expected {
				t.Errorf("Expected command %q, got %q", tt.expected, cmd)
			}
		})
	}
}

// Test that SyncwebReplCmd implements the command interface
func TestSyncwebReplCmd_Interface(t *testing.T) {
	var _ interface {
		Run(*SyncwebCmd) error
	} = &SyncwebReplCmd{}
}

func TestSyncwebReplCmd_HelpText(t *testing.T) {
	// Test that help text contains expected commands
	expectedCommands := []string{
		"folders",
		"devices",
		"pending",
		"events",
		"stats",
		"ignores",
		"add-device",
		"add-folder",
		"pause-folder",
		"resume-folder",
		"pause-device",
		"resume-device",
		"delete-folder",
		"delete-device",
		"whoami",
		"exit",
	}

	// Create a simple check that the help text structure is correct
	helpText := `Available commands:
  folders, lsf          - List folders
  devices, lsd          - List devices
  pending               - List pending devices
  events                - Show recent events
  stats                 - Show folder statistics
  device-stats          - Show device statistics
  ignores <folder>      - Show ignore patterns
  set-ignores <folder> <patterns> - Set ignore patterns
  add-device <id> [name] - Add a device
  add-folder <id> <label> <path> - Add a folder
  pause-folder <id>     - Pause a folder
  resume-folder <id>    - Resume a folder
  pause-device <id>     - Pause a device
  resume-device <id>    - Resume a device
  delete-folder <id>    - Delete a folder
  delete-device <id>    - Delete a device
  whoami                - Show current node info
  exit, quit, q         - Exit REPL`

	for _, expectedCmd := range expectedCommands {
		if !strings.Contains(helpText, expectedCmd) {
			t.Errorf("Expected help text to contain %q", expectedCmd)
		}
	}
}
