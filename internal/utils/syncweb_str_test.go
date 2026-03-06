package utils

import (
	"testing"
	"time"
)

func TestParseSyncwebPath(t *testing.T) {
	tests := []struct {
		name     string
		url      string
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
			name:    "invalid URL",
			url:     "http://example.com",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ref, err := ParseSyncwebPath(tt.url, true)
			if (err != nil) != tt.wantErr {
				t.Fatalf("ParseSyncwebPath() error = %v, wantErr %v", err, tt.wantErr)
			}
			if err != nil {
				return
			}

			if ref.FolderID != tt.wantID {
				t.Errorf("FolderID = %v, want %v", ref.FolderID, tt.wantID)
			}
			if ref.Subpath != tt.wantPath {
				t.Errorf("Subpath = %v, want %v", ref.Subpath, tt.wantPath)
			}
			if ref.DeviceID != tt.wantDev {
				t.Errorf("DeviceID = %v, want %v", ref.DeviceID, tt.wantDev)
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
			name:    "invalid device ID",
			input:   "invalid-id",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := ExtractDeviceID(tt.input)
			if (err != nil) != tt.wantErr {
				t.Fatalf("ExtractDeviceID() error = %v, wantErr %v", err, tt.wantErr)
			}
			if err != nil {
				return
			}
			if got != tt.want {
				t.Errorf("ExtractDeviceID() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestDeviceIDShort2Long(t *testing.T) {
	knownDevices := []string{
		"ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		"ABD1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
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
			want:  "", // Both start with AB, so ambiguous
		},
		{
			name:  "full ID",
			short: "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
			want:  "ABC1234-DEF5678-GHI9012-JKL3456-MNO7890-PQR1234-STU5678-VWX9012",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := DeviceIDShort2Long(tt.short, knownDevices)
			if got != tt.want {
				t.Errorf("DeviceIDShort2Long() = %v, want %v", got, tt.want)
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
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := CreateFolderID(tt.path, tt.existing)
			if got != tt.want {
				t.Errorf("CreateFolderID() = %v, want %v", got, tt.want)
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
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := SepReplace(tt.path)
			if got != tt.want {
				t.Errorf("SepReplace() = %v, want %v", got, tt.want)
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
			name:  "KB",
			bytes: 1024,
			want:  "1.0 KB",
		},
		{
			name:  "MB",
			bytes: 1024 * 1024,
			want:  "1.0 MB",
		},
		{
			name:  "GB",
			bytes: 1024 * 1024 * 1024,
			want:  "1.0 GB",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := FormatSize(tt.bytes)
			if got != tt.want {
				t.Errorf("FormatSize() = %v, want %v", got, tt.want)
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
		name  string
		input string
		want  int64
	}{
		{
			name:  "RFC3339",
			input: "2024-01-15T10:30:00Z",
			want:  1705314600,
		},
		{
			name:  "date only",
			input: "2024-01-15",
			want:  1705276800,
		},
		{
			name:  "empty",
			input: "",
			want:  0,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := IsoDateToSeconds(tt.input)
			// Allow some tolerance for timezone differences
			diff := got - tt.want
			if diff < -86400 || diff > 86400 {
				t.Errorf("IsoDateToSeconds() = %v, want %v (diff: %d)", got, tt.want, diff)
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
			name:  "KB",
			input: "10KB",
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
			name:  "bytes",
			input: "1024",
			want:  1024,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := HumanToBytes(tt.input)
			if (err != nil) != tt.wantErr {
				t.Fatalf("HumanToBytes() error = %v, wantErr %v", err, tt.wantErr)
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
			name:  "hours",
			input: "2 hours",
			want:  7200,
		},
		{
			name:  "days",
			input: "3 days",
			want:  259200,
		},
		{
			name:  "seconds",
			input: "30 seconds",
			want:  30,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := HumanToSeconds(tt.input)
			if (err != nil) != tt.wantErr {
				t.Fatalf("HumanToSeconds() error = %v, wantErr %v", err, tt.wantErr)
			}
			if got != tt.want {
				t.Errorf("HumanToSeconds() = %v, want %v", got, tt.want)
			}
		})
	}
}
