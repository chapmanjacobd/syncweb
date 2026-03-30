package utils

import (
	"testing"
	"time"
)

func TestParseSyncwebPath(t *testing.T) {
	tests := []struct {
		name     string
		url      string
		decode   bool
		wantID   string
		wantPath string
		wantDev  string
		wantErr  bool
	}{
		{
			name:    "basic URL",
			url:     "syncweb://folder-id#device-id",
			wantID:  "folder-id",
			wantDev: "device-id",
		},
		{
			name:     "URL with subpath",
			url:      "syncweb://folder-id/sub/path#device-id",
			wantID:   "folder-id",
			wantPath: "sub/path",
			wantDev:  "device-id",
		},
		{
			name:    "URL without device",
			url:     "syncweb://folder-id",
			wantID:  "folder-id",
			wantDev: "",
		},
		{
			name:    "invalid URL scheme",
			url:     "http://example.com",
			wantErr: true,
		},
		{
			name:    "empty URL",
			url:     "",
			wantErr: true,
		},
		{
			name:    "missing scheme",
			url:     "folder-id",
			wantErr: true,
		},
		{
			name:    "only scheme",
			url:     "syncweb://",
			wantID:  "",
			wantDev: "",
		},
		{
			name:     "URL with encoded characters",
			url:      "syncweb://folder%20name/path%2Fencoded#device-id",
			decode:   true,
			wantID:   "folder name",
			wantPath: "path/encoded",
			wantDev:  "device-id",
		},
		{
			name:     "URL with deep subpath",
			url:      "syncweb://folder/a/b/c/d/e/file.txt#device",
			wantID:   "folder",
			wantPath: "a/b/c/d/e/file.txt",
			wantDev:  "device",
		},
		{
			name:     "URL with empty folder ID",
			url:      "syncweb:///path#device",
			wantID:   "",
			wantPath: "path",
			wantDev:  "device",
		},
		{
			name:    "URL with special characters in folder ID",
			url:     "syncweb://my-folder_123#device",
			wantID:  "my-folder_123",
			wantDev: "device",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			decode := tt.decode
			if !decode && tt.decode {
				decode = true
			}
			ref, err := ParseSyncwebPath(tt.url, decode)
			if (err != nil) != tt.wantErr {
				t.Errorf("ParseSyncwebPath() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if err != nil {
				return
			}

			if ref.FolderID != tt.wantID {
				t.Errorf("FolderID = %q, want %q", ref.FolderID, tt.wantID)
			}
			if ref.Subpath != tt.wantPath {
				t.Errorf("Subpath = %q, want %q", ref.Subpath, tt.wantPath)
			}
			if ref.DeviceID != tt.wantDev {
				t.Errorf("DeviceID = %q, want %q", ref.DeviceID, tt.wantDev)
			}
		})
	}
}

func TestExtractDeviceID(t *testing.T) {
	tests := []struct {
		name    string
		input   string
		want    string
		wantErr bool
	}{
		{
			name:  "valid full device ID",
			input: "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
			want:  "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		},
		{
			name:    "invalid device ID - too short",
			input:   "invalid-id",
			wantErr: true,
		},
		{
			name:    "invalid device ID - wrong format",
			input:   "ABC-DEF-GHI",
			wantErr: true,
		},
		{
			name:    "empty input",
			input:   "",
			wantErr: true,
		},
		{
			name:  "device ID with whitespace",
			input: "  ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012  ",
			want:  "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		},
		{
			name:  "device ID from syncweb URL",
			input: "syncweb://folder#ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
			want:  "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		},
		{
			name:    "syncweb URL without device ID",
			input:   "syncweb://folder",
			wantErr: true,
		},
		{
			name:  "device ID embedded in text",
			input: "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
			want:  "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		},
		{
			name:    "lowercase device ID",
			input:   "abc1234-def5678-ghi9012-jkl3456-mno7890-pqr1234-stu5678-vwx9012",
			wantErr: true, // Device IDs should be uppercase
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := ExtractDeviceID(tt.input)
			if (err != nil) != tt.wantErr {
				t.Errorf("ExtractDeviceID() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if err != nil {
				return
			}
			if got != tt.want {
				t.Errorf("ExtractDeviceID() = %q, want %q", got, tt.want)
			}
		})
	}
}

func TestDeviceIDShort2Long(t *testing.T) {
	knownDevices := []string{
		"ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		"ABD1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		"XYZ9999-AAA1111-BBB2222-CCC3333-DDD4444-EEE5555-FFF6666-GGG7777",
	}

	tests := []struct {
		name  string
		short string
		want  string
	}{
		{
			name:  "unique match",
			short: "ABC1234",
			want:  "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		},
		{
			name:  "ambiguous prefix",
			short: "AB",
			want:  "", // Both ABC and ABD start with AB
		},
		{
			name:  "full ID",
			short: "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
			want:  "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		},
		{
			name:  "empty short ID",
			short: "",
			want:  "",
		},
		{
			name:  "whitespace in short ID",
			short: "  ABC1234  ",
			want:  "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		},
		{
			name:  "lowercase short ID",
			short: "abc1234",
			want:  "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		},
		{
			name:  "no match",
			short: "ZZZ9999",
			want:  "",
		},
		{
			name:  "partial match multiple segments",
			short: "ABC1234-DEF5678",
			want:  "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		},
		{
			name:  "single character",
			short: "A",
			want:  "", // Ambiguous - matches multiple
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := DeviceIDShort2Long(tt.short, knownDevices)
			if got != tt.want {
				t.Errorf("DeviceIDShort2Long() = %q, want %q", got, tt.want)
			}
		})
	}
}

func TestCreateFolderID(t *testing.T) {
	tests := []struct {
		name     string
		path     string
		existing map[string]bool
		want     string
	}{
		{
			name:     "new folder",
			path:     "/home/user/music",
			existing: map[string]bool{},
			want:     "music",
		},
		{
			name:     "existing folder",
			path:     "/home/user/music",
			existing: map[string]bool{"music": true},
			want:     "home-user-music",
		},
		{
			name:     "root path",
			path:     "/",
			existing: map[string]bool{},
			want:     "/", // filepath.Base("/") returns "/"
		},
		{
			name:     "windows path",
			path:     "C:\\Users\\Documents",
			existing: map[string]bool{},
			want:     "C:\\Users\\Documents", // filepath.Base on Linux doesn't handle Windows paths
		},
		{
			name:     "windows path with conflict",
			path:     "C:\\Users\\Documents",
			existing: map[string]bool{"Documents": true, "C:\\Users\\Documents": true},
			want:     "C:-Users-Documents",
		},
		{
			name:     "path with trailing slash",
			path:     "/home/user/photos/",
			existing: map[string]bool{},
			want:     "photos", // filepath.Base strips trailing slash
		},
		{
			name:     "empty path",
			path:     "",
			existing: map[string]bool{},
			want:     ".", // filepath.Base("") returns "."
		},
		{
			name:     "single component",
			path:     "music",
			existing: map[string]bool{},
			want:     "music",
		},
		{
			name:     "multiple conflicts",
			path:     "/data/backup/files",
			existing: map[string]bool{"files": true, "data-backup-files": true},
			want:     "data-backup-files",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := CreateFolderID(tt.path, tt.existing)
			if got != tt.want {
				t.Errorf("CreateFolderID() = %q, want %q", got, tt.want)
			}
		})
	}
}

func TestSepReplace(t *testing.T) {
	tests := []struct {
		name string
		path string
		want string
	}{
		{
			name: "unix path",
			path: "/home/user/music",
			want: "home-user-music",
		},
		{
			name: "windows path",
			path: "C:\\Users\\Music",
			want: "C:-Users-Music", // Colon is preserved, backslash becomes dash
		},
		{
			name: "mixed separators",
			path: "/home\\user/music",
			want: "home-user-music",
		},
		{
			name: "multiple consecutive separators",
			path: "/home//user///music",
			want: "home-user-music",
		},
		{
			name: "leading separator",
			path: "/home",
			want: "home",
		},
		{
			name: "trailing separator",
			path: "home/",
			want: "home",
		},
		{
			name: "no separators",
			path: "music",
			want: "music",
		},
		{
			name: "empty path",
			path: "",
			want: "",
		},
		{
			name: "path with spaces",
			path: "/home/user/My Music",
			want: "home-user-My Music",
		},
		{
			name: "path with special characters",
			path: "/home/user/music_2024",
			want: "home-user-music_2024",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := SepReplace(tt.path)
			if got != tt.want {
				t.Errorf("SepReplace() = %q, want %q", got, tt.want)
			}
		})
	}
}

func TestFormatSize(t *testing.T) {
	tests := []struct {
		name  string
		bytes int64
		want  string
	}{
		{
			name:  "zero",
			bytes: 0,
			want:  "-",
		},
		{
			name:  "bytes",
			bytes: 512,
			want:  "512 B",
		},
		{
			name:  "one byte",
			bytes: 1,
			want:  "1 B",
		},
		{
			name:  "KB",
			bytes: 1024,
			want:  "1.0 KB",
		},
		{
			name:  "KB fraction",
			bytes: 1536,
			want:  "1.5 KB",
		},
		{
			name:  "MB",
			bytes: 1024 * 1024,
			want:  "1.0 MB",
		},
		{
			name:  "MB fraction",
			bytes: 1572864,
			want:  "1.5 MB",
		},
		{
			name:  "GB",
			bytes: 1024 * 1024 * 1024,
			want:  "1.0 GB",
		},
		{
			name:  "GB fraction",
			bytes: 1610612736,
			want:  "1.5 GB",
		},
		{
			name:  "TB",
			bytes: 1024 * 1024 * 1024 * 1024,
			want:  "1.0 TB",
		},
		{
			name:  "large TB",
			bytes: 1024 * 1024 * 1024 * 1024 * 5,
			want:  "5.0 TB",
		},
		{
			name:  "negative bytes",
			bytes: -1024,
			want:  "-1024 B", // Negative bytes stay as bytes
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := FormatSize(tt.bytes)
			if got != tt.want {
				t.Errorf("FormatSize() = %q, want %q", got, tt.want)
			}
		})
	}
}

func TestRelativeTime(t *testing.T) {
	now := time.Now()
	past := now.Add(-24 * time.Hour).Unix()

	result := RelativeTime(past)
	if result == "" {
		t.Error("RelativeTime() returned empty string")
	}
}

func TestIsoDateToSeconds(t *testing.T) {
	tests := []struct {
		name    string
		input   string
		wantMin int64 // minimum expected (for timezone tolerance)
		wantMax int64 // maximum expected (for timezone tolerance)
	}{
		{
			name:    "RFC3339 UTC",
			input:   "2024-01-15T10:30:00Z",
			wantMin: 1705314600,
			wantMax: 1705314600,
		},
		{
			name:    "date only",
			input:   "2024-01-15",
			wantMin: 1705276800,
			wantMax: 1705276800,
		},
		{
			name:    "empty",
			input:   "",
			wantMin: 0,
			wantMax: 0,
		},
		{
			name:    "RFC3339 with timezone offset",
			input:   "2024-01-15T10:30:00+00:00",
			wantMin: 1705314600,
			wantMax: 1705314600,
		},
		{
			name:    "ISO format without seconds",
			input:   "2024-01-15T10:30",
			wantMin: 0, // This format is not supported by the current implementation
			wantMax: 0,
		},
		{
			name:    "invalid format",
			input:   "not-a-date",
			wantMin: 0,
			wantMax: 0,
		},
		{
			name:    "RFC3339Nano",
			input:   "2024-01-15T10:30:00.123456789Z",
			wantMin: 1705314600,
			wantMax: 1705314600,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := IsoDateToSeconds(tt.input)
			if got < tt.wantMin || got > tt.wantMax {
				t.Errorf("IsoDateToSeconds() = %v, want [%v, %v]", got, tt.wantMin, tt.wantMax)
			}
		})
	}
}

func TestHumanToBytes(t *testing.T) {
	tests := []struct {
		name    string
		input   string
		want    int64
		wantErr bool
	}{
		{
			name:  "KB uppercase",
			input: "10KB",
			want:  10 * 1024,
		},
		{
			name:  "KB lowercase",
			input: "10kb",
			want:  10 * 1024,
		},
		{
			name:  "MB",
			input: "5MB",
			want:  5 * 1024 * 1024,
		},
		{
			name:  "GB",
			input: "2GB",
			want:  2 * 1024 * 1024 * 1024,
		},
		{
			name:  "TB",
			input: "1TB",
			want:  1024 * 1024 * 1024 * 1024,
		},
		{
			name:  "bytes only",
			input: "1024",
			want:  1024,
		},
		{
			name:  "zero bytes",
			input: "0",
			want:  0,
		},
		{
			name:  "with space",
			input: "10 KB",
			want:  10 * 1024,
		},
		{
			name:    "invalid unit",
			input:   "10XB",
			wantErr: true,
		},
		{
			name:    "empty input",
			input:   "",
			wantErr: true,
		},
		{
			name:    "non-numeric",
			input:   "abc",
			wantErr: true,
		},
		{
			name:  "fractional KB",
			input: "1.5KB",
			want:  1536,
		},
		{
			name:  "fractional MB",
			input: "2.5MB",
			want:  2621440,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := HumanToBytes(tt.input)
			if (err != nil) != tt.wantErr {
				t.Errorf("HumanToBytes() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if err != nil {
				return
			}
			if got != tt.want {
				t.Errorf("HumanToBytes() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestHumanToSeconds(t *testing.T) {
	tests := []struct {
		name    string
		input   string
		want    int64
		wantErr bool
	}{
		{
			name:  "minutes",
			input: "5 minutes",
			want:  300,
		},
		{
			name:  "minute singular",
			input: "1 minute",
			want:  60,
		},
		{
			name:  "hours",
			input: "2 hours",
			want:  7200,
		},
		{
			name:  "hour singular",
			input: "1 hour",
			want:  3600,
		},
		{
			name:  "days",
			input: "3 days",
			want:  259200,
		},
		{
			name:  "day singular",
			input: "1 day",
			want:  86400,
		},
		{
			name:  "seconds",
			input: "30 seconds",
			want:  30,
		},
		{
			name:  "second singular",
			input: "1 second",
			want:  1,
		},
		{
			name:  "abbreviated minutes",
			input: "5m",
			want:  300,
		},
		{
			name:  "abbreviated hours",
			input: "2h",
			want:  7200,
		},
		{
			name:  "abbreviated days",
			input: "3d",
			want:  259200,
		},
		{
			name:  "abbreviated seconds",
			input: "30s",
			want:  30,
		},
		{
			name:    "invalid unit",
			input:   "5xyz",
			wantErr: true,
		},
		{
			name:    "empty input",
			input:   "",
			want:    0, // Empty input returns 0
			wantErr: false,
		},
		{
			name:    "non-numeric",
			input:   "abc minutes",
			wantErr: true,
		},
		{
			name:  "fractional minutes",
			input: "1.5 minutes",
			want:  90,
		},
		{
			name:  "fractional hours",
			input: "0.5 hours",
			want:  1800,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := HumanToSeconds(tt.input)
			if (err != nil) != tt.wantErr {
				t.Errorf("HumanToSeconds() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if err != nil {
				return
			}
			if got != tt.want {
				t.Errorf("HumanToSeconds() = %v, want %v", got, tt.want)
			}
		})
	}
}
