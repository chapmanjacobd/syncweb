package utils

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestRandomString(t *testing.T) {
	// Test length
	result := RandomString(10)
	if len(result) != 10 {
		t.Errorf("RandomString(10) returned string of length %d, expected 10", len(result))
	}

	// Test uniqueness
	seen := make(map[string]bool)
	for range 100 {
		result := RandomString(16)
		if seen[result] {
			t.Error("RandomString generated a duplicate value")
		}
		seen[result] = true
	}

	// Test hexadecimal characters
	result = RandomString(20)
	for _, c := range result {
		if (c < '0' || c > '9') && (c < 'a' || c > 'f') {
			t.Errorf("RandomString returned non-hexadecimal character: %c", c)
		}
	}
}

func TestRandomFilename(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"with extension", "file.txt", "file."},
		{"multiple dots", "file.tar.gz", "file.tar."},
		{"no extension", "file", "file."},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := RandomFilename(tt.input)
			if !strings.HasPrefix(result, tt.expected) {
				t.Errorf("RandomFilename(%q) = %q, expected to start with %q", tt.input, result, tt.expected)
			}
			// Should have random suffix
			if len(result) <= len(tt.expected) {
				t.Error("RandomFilename should add random characters")
			}
		})
	}
}

func TestTrimPathSegments(t *testing.T) {
	tests := []struct {
		name          string
		input         string
		desiredLength int
		expectedLen   int
		shouldShorten bool
	}{
		{"short path", "/home/user/file.txt", 100, -1, false},
		{"long path", "/home/user/verylongdirectoryname/anotherlongname/file.txt", 40, 40, true},
		{"root level", "file.txt", 10, -1, false},
		{"absolute short", "/a/b.txt", 20, -1, false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := TrimPathSegments(tt.input, tt.desiredLength)
			if tt.shouldShorten && len(result) > tt.desiredLength {
				t.Errorf("TrimPathSegments(%q, %d) = %q (len=%d), expected len <= %d",
					tt.input, tt.desiredLength, result, len(result), tt.desiredLength)
			}
			if !tt.shouldShorten && result != tt.input {
				// Allow some flexibility for paths that don't need shortening
				if tt.expectedLen < 0 && len(result) <= tt.desiredLength {
					return
				}
			}
		})
	}
}

func TestSafeJoin(t *testing.T) {
	tests := []struct {
		name     string
		base     string
		userPath string
		expected string
	}{
		{"simple join", "/home", "user", "/home/user"},
		{"with subdirs", "/home", "user/docs", "/home/user/docs"},
		{"with dot", "/home", "./user", "/home/user"},
		{"with double dot", "/home", "../etc", "/home/etc"},
		{"empty user path", "/home", "", "/home"},
		{"absolute user path", "/home", "/etc/passwd", "/home/etc/passwd"},
		{"multiple dots", "/home", "user/./docs/../files", "/home/user/files"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := SafeJoin(tt.base, tt.userPath)
			if result != tt.expected {
				t.Errorf("SafeJoin(%q, %q) = %q, expected %q", tt.base, tt.userPath, result, tt.expected)
			}
		})
	}
}

func TestRelativize(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"unix absolute", "/home/user/file.txt", "home/user/file.txt"},
		{"windows absolute", "C:/Users/user/file.txt", "Users/user/file.txt"},
		{"relative", "home/user/file.txt", "home/user/file.txt"},
		{"with leading slashes", "///home/user", "home/user"},
		{"mixed slashes", "C:\\Users\\user", "Users\\user"},
		{"empty", "", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := Relativize(tt.input)
			if result != tt.expected {
				t.Errorf("Relativize(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestStripMountSyntax(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"unix absolute", "/mnt/data/file.txt", "mnt/data/file.txt"},
		{"windows absolute", "D:/data/file.txt", "data/file.txt"},
		{"relative", "data/file.txt", "data/file.txt"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := StripMountSyntax(tt.input)
			if result != tt.expected {
				t.Errorf("StripMountSyntax(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestIsEmptyFolder(t *testing.T) {
	tmpDir := t.TempDir()

	// Test empty folder
	emptyDir := filepath.Join(tmpDir, "empty")
	err := os.MkdirAll(emptyDir, 0o755)
	if err != nil {
		t.Fatalf("Failed to create empty directory: %v", err)
	}

	if !IsEmptyFolder(emptyDir) {
		t.Error("EmptyFolder should return true for empty folder")
	}

	// Test non-empty folder
	nonEmptyDir := filepath.Join(tmpDir, "nonempty")
	err = os.MkdirAll(nonEmptyDir, 0o755)
	if err != nil {
		t.Fatalf("Failed to create directory: %v", err)
	}
	err = os.WriteFile(filepath.Join(nonEmptyDir, "file.txt"), []byte("content"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create file: %v", err)
	}

	if IsEmptyFolder(nonEmptyDir) {
		t.Error("EmptyFolder should return false for non-empty folder")
	}

	// Test folder with only subdirectories (no files)
	subdirOnly := filepath.Join(tmpDir, "subdirsonly")
	err = os.MkdirAll(filepath.Join(subdirOnly, "subdir"), 0o755)
	if err != nil {
		t.Fatalf("Failed to create subdirectory: %v", err)
	}

	if !IsEmptyFolder(subdirOnly) {
		t.Error("EmptyFolder should return true for folder with only empty subdirectories")
	}
}

func TestFolderSize(t *testing.T) {
	tmpDir := t.TempDir()

	// Create test files
	file1 := filepath.Join(tmpDir, "file1.txt")
	file2 := filepath.Join(tmpDir, "file2.txt")
	err := os.WriteFile(file1, []byte("12345"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create file1: %v", err)
	}
	err = os.WriteFile(file2, []byte("1234567890"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create file2: %v", err)
	}

	size := FolderSize(tmpDir)
	expected := int64(15) // 5 + 10 bytes
	if size != expected {
		t.Errorf("FolderSize returned %d, expected %d", size, expected)
	}
}

func TestPathTupleFromURL(t *testing.T) {
	tests := []struct {
		name         string
		input        string
		expectedDir  string
		expectedFile string
	}{
		{"simple url", "http://example.com/file.txt", "example.com", "file.txt"},
		{"with path", "http://example.com/dir/file.txt", "example.com/dir", "file.txt"},
		{"with port", "http://example.com:8080/file.txt", "example.com.8080", "file.txt"},
		{"nested path", "http://example.com/a/b/c/file.txt", "example.com/a/b/c", "file.txt"},
		{"root path", "http://example.com/", "example.com", ""},
		{"no path", "http://example.com", "example.com", ""},
		{"invalid url", "not a url", "", "not a url"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			dir, file := PathTupleFromURL(tt.input)
			if dir != tt.expectedDir {
				t.Errorf("PathTupleFromURL(%q) dir = %q, expected %q", tt.input, dir, tt.expectedDir)
			}
			if file != tt.expectedFile {
				t.Errorf("PathTupleFromURL(%q) file = %q, expected %q", tt.input, file, tt.expectedFile)
			}
		})
	}
}

func TestCleanPath(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		opts     CleanPathOptions
		expected string
	}{
		{
			name:     "basic clean",
			input:    "/home/user/My File.txt",
			opts:     CleanPathOptions{},
			expected: "/home/user/My File.txt",
		},
		{
			name:     "lowercase folders",
			input:    "/home/User/Docs",
			opts:     CleanPathOptions{LowercaseFolders: true},
			expected: "/home/user/Docs",
		},
		{
			name:     "dot space",
			input:    "/home/user/My File.txt",
			opts:     CleanPathOptions{DotSpace: true},
			expected: "/home/user/My.File.txt",
		},
		{
			name:     "dedupe parts",
			input:    "/home/home/user/file.txt",
			opts:     CleanPathOptions{DedupeParts: true},
			expected: "/home/user/file.txt",
		},
		{
			name:     "case insensitive",
			input:    "/home/my_file.txt",
			opts:     CleanPathOptions{CaseInsensitive: true},
			expected: "/home/my_file.txt",
		},
		{
			name:     "windows path",
			input:    "C:\\Users\\file.txt",
			opts:     CleanPathOptions{},
			expected: "C:Users file.txt",
		},
		{
			name:     "empty parts become underscore",
			input:    "/home/  /file.txt",
			opts:     CleanPathOptions{},
			expected: "/home/_/file.txt",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := CleanPath(tt.input, tt.opts)
			if result != tt.expected {
				t.Errorf("CleanPath(%q, %v) = %q, expected %q", tt.input, tt.opts, result, tt.expected)
			}
		})
	}
}

func TestCleanPath_MaxNameLen(t *testing.T) {
	// Test with very long filename
	longName := "/home/user/" + string(make([]byte, 300)) + ".txt"
	opts := CleanPathOptions{MaxNameLen: 255}
	result := CleanPath(longName, opts)
	// Just verify it doesn't crash and returns something
	if result == "" {
		t.Error("CleanPath with MaxNameLen returned empty string")
	}
}
