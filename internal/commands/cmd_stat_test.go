package commands_test

import (
	"testing"

	"github.com/syncthing/syncthing/lib/protocol"

	"github.com/chapmanjacobd/syncweb/internal/commands"
)

func TestGetFileType(t *testing.T) {
	tests := []struct {
		name     string
		fileType protocol.FileInfoType
		expected string
	}{
		{"file", protocol.FileInfoTypeFile, "regular file"},
		{"directory", protocol.FileInfoTypeDirectory, "directory"},
		{"symlink", protocol.FileInfoTypeSymlink, "symbolic link"},
		{"unknown", protocol.FileInfoTypeFile, "unknown"}, // Using File as default for unknown
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			info := protocol.FileInfo{Type: tt.fileType}
			if tt.name == "unknown" {
				info.Type = 99 // Invalid type
			}
			result := commands.GetFileType(&info)
			if result != tt.expected {
				t.Errorf("GetFileType(%v) = %q, expected %q", tt.fileType, result, tt.expected)
			}
		})
	}
}

func TestFormatVersion(t *testing.T) {
	tests := []struct {
		name     string
		version  protocol.Vector
		expected string
	}{
		{
			name:     "empty",
			version:  protocol.Vector{},
			expected: "none",
		},
		{
			name: "single counter",
			version: protocol.Vector{
				Counters: []protocol.Counter{{ID: 1, Value: 5}},
			},
			expected: "1:5",
		},
		{
			name: "multiple counters",
			version: protocol.Vector{
				Counters: []protocol.Counter{
					{ID: 1, Value: 5},
					{ID: 2, Value: 3},
				},
			},
			expected: "1:5, 2:3",
		},
		{
			name: "many counters",
			version: protocol.Vector{
				Counters: []protocol.Counter{
					{ID: 1, Value: 5},
					{ID: 2, Value: 3},
					{ID: 3, Value: 7},
					{ID: 4, Value: 2},
				},
			},
			expected: "1:5, 2:3, 3:7, ... (4 total)",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := commands.FormatVersion(tt.version)
			if result != tt.expected {
				t.Errorf("FormatVersion(%v) = %q, expected %q", tt.version, result, tt.expected)
			}
		})
	}
}
