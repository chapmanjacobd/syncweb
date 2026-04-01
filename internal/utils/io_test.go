package utils_test

import (
	"bytes"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"testing"

	"github.com/chapmanjacobd/syncweb/internal/utils"
)

func TestFileExists(t *testing.T) {
	tmpDir := t.TempDir()
	existingFile := filepath.Join(tmpDir, "exists.txt")
	nonExistingFile := filepath.Join(tmpDir, "not_exists.txt")

	err := os.WriteFile(existingFile, []byte("test"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	if !utils.FileExists(existingFile) {
		t.Error("FileExists should return true for existing file")
	}

	if utils.FileExists(nonExistingFile) {
		t.Error("FileExists should return false for non-existing file")
	}
}

func TestDirExists(t *testing.T) {
	tmpDir := t.TempDir()
	existingDir := filepath.Join(tmpDir, "exists")
	nonExistingDir := filepath.Join(tmpDir, "not_exists")

	err := os.MkdirAll(existingDir, 0o755)
	if err != nil {
		t.Fatalf("Failed to create test directory: %v", err)
	}

	// Create a file (not a directory)
	filePath := filepath.Join(tmpDir, "file.txt")
	err = os.WriteFile(filePath, []byte("test"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	if !utils.DirExists(existingDir) {
		t.Error("DirExists should return true for existing directory")
	}

	if utils.DirExists(nonExistingDir) {
		t.Error("DirExists should return false for non-existing directory")
	}

	if utils.DirExists(filePath) {
		t.Error("DirExists should return false for a file")
	}
}

func TestGetDefaultBrowser(t *testing.T) {
	result := utils.GetDefaultBrowser()
	if result == "" {
		t.Error("GetDefaultBrowser should return a non-empty string")
	}

	// Check it returns a known command
	validCommands := []string{"xdg-open", "open", "start"}
	found := slices.Contains(validCommands, result)
	if !found {
		t.Errorf("GetDefaultBrowser returned unknown command: %s", result)
	}
}

func TestIsSQLite(t *testing.T) {
	tmpDir := t.TempDir()

	// Create a valid SQLite file (with proper header)
	sqliteFile := filepath.Join(tmpDir, "test.db")
	header := []byte("SQLite format 3\x00")
	content := append(header, []byte("rest of file")...)
	err := os.WriteFile(sqliteFile, content, 0o644)
	if err != nil {
		t.Fatalf("Failed to create SQLite file: %v", err)
	}

	if !utils.IsSQLite(sqliteFile) {
		t.Error("IsSQLite should return true for valid SQLite file")
	}

	// Create a non-SQLite file
	nonSqliteFile := filepath.Join(tmpDir, "not.db")
	err = os.WriteFile(nonSqliteFile, []byte("not a sqlite file"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create non-SQLite file: %v", err)
	}

	if utils.IsSQLite(nonSqliteFile) {
		t.Error("IsSQLite should return false for non-SQLite file")
	}

	// Test non-existent file
	if utils.IsSQLite("/nonexistent/file.db") {
		t.Error("IsSQLite should return false for non-existent file")
	}

	// Test empty file
	emptyFile := filepath.Join(tmpDir, "empty.db")
	err = os.WriteFile(emptyFile, []byte(""), 0o644)
	if err != nil {
		t.Fatalf("Failed to create empty file: %v", err)
	}

	if utils.IsSQLite(emptyFile) {
		t.Error("IsSQLite should return false for empty file")
	}
}

func TestReadLines(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected []string
	}{
		{
			name:     "simple lines",
			input:    "line1\nline2\nline3",
			expected: []string{"line1", "line2", "line3"},
		},
		{
			name:     "with empty lines",
			input:    "line1\n\nline2\n\nline3",
			expected: []string{"line1", "line2", "line3"},
		},
		{
			name:     "with whitespace",
			input:    "  line1  \n  line2  \n",
			expected: []string{"line1", "line2"},
		},
		{
			name:     "empty input",
			input:    "",
			expected: []string{},
		},
		{
			name:     "only whitespace",
			input:    "   \n\n   ",
			expected: []string{},
		},
		{
			name:     "windows line endings",
			input:    "line1\r\nline2\r\nline3",
			expected: []string{"line1", "line2", "line3"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			reader := strings.NewReader(tt.input)
			result := utils.ReadLines(reader)
			if len(result) != len(tt.expected) {
				t.Errorf("ReadLines returned %d lines, expected %d", len(result), len(tt.expected))
				return
			}
			for i, line := range result {
				if line != tt.expected[i] {
					t.Errorf("Line %d: got %q, expected %q", i, line, tt.expected[i])
				}
			}
		})
	}
}

func TestExpandStdin(t *testing.T) {
	// Save original stdin
	originalStdin := utils.Stdin

	tests := []struct {
		name     string
		input    []string
		stdin    string
		expected []string
	}{
		{
			name:     "no stdin expansion",
			input:    []string{"file1", "file2"},
			stdin:    "",
			expected: []string{"file1", "file2"},
		},
		{
			name:     "with stdin expansion",
			input:    []string{"file1", "-", "file2"},
			stdin:    "stdin1\nstdin2\nstdin3",
			expected: []string{"file1", "stdin1", "stdin2", "stdin3", "file2"},
		},
		{
			name:     "only stdin",
			input:    []string{"-"},
			stdin:    "line1\nline2",
			expected: []string{"line1", "line2"},
		},
		{
			name:     "empty stdin",
			input:    []string{"-"},
			stdin:    "",
			expected: []string{},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			utils.Stdin = strings.NewReader(tt.stdin)
			result := utils.ExpandStdin(tt.input)
			if len(result) != len(tt.expected) {
				t.Errorf("ExpandStdin returned %d elements, expected %d", len(result), len(tt.expected))
				return
			}
			for i, v := range result {
				if v != tt.expected[i] {
					t.Errorf("Element %d: got %q, expected %q", i, v, tt.expected[i])
				}
			}
		})
	}

	// Restore original stdin
	utils.Stdin = originalStdin
}

func TestConfirm(t *testing.T) {
	originalStdin := utils.Stdin
	originalStdout := utils.Stdout

	tests := []struct {
		name     string
		input    string
		expected bool
	}{
		{"yes lowercase", "y\n", true},
		{"yes uppercase", "Y\n", true},
		{"yes full", "yes\n", true},
		{"yes full uppercase", "YES\n", true},
		{"no", "n\n", false},
		{"no full", "no\n", false},
		{"empty", "\n", false},
		{"random", "random\n", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			utils.Stdin = strings.NewReader(tt.input)
			var output bytes.Buffer
			utils.Stdout = &output

			result := utils.Confirm("Test message")
			if result != tt.expected {
				t.Errorf("Confirm(%q) = %v, expected %v", tt.input, result, tt.expected)
			}

			// Check prompt was written
			if !strings.Contains(output.String(), "Test message") {
				t.Error("Confirm should write the message to stdout")
			}
		})
	}

	utils.Stdin = originalStdin
	utils.Stdout = originalStdout
}

func TestPrompt(t *testing.T) {
	originalStdin := utils.Stdin
	originalStdout := utils.Stdout

	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"simple input", "hello\n", "hello"},
		{"with whitespace", "  hello world  \n", "hello world"},
		{"empty", "\n", ""},
		{"no newline", "hello", "hello"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			utils.Stdin = strings.NewReader(tt.input)
			var output bytes.Buffer
			utils.Stdout = &output

			result := utils.Prompt("Test message")
			if result != tt.expected {
				t.Errorf("Prompt(%q) = %q, expected %q", tt.input, result, tt.expected)
			}

			// Check prompt was written
			if !strings.Contains(output.String(), "Test message") {
				t.Error("Prompt should write the message to stdout")
			}
		})
	}

	utils.Stdin = originalStdin
	utils.Stdout = originalStdout
}
