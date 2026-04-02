package utils_test

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/chapmanjacobd/syncweb/internal/utils"
)

func TestSampleHashFile(t *testing.T) {
	// Create a temporary file for testing
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "test_hash.txt")
	content := "Hello, World! This is a test file for hashing."
	err := os.WriteFile(testFile, []byte(content), 0o644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	// Test with default parameters
	hash, err := utils.SampleHashFile(testFile, 1, 0.5, 0)
	if err != nil {
		t.Fatalf("SampleHashFile failed: %v", err)
	}
	if hash == "" {
		t.Error("Expected non-empty hash")
	}

	// Test with custom chunk size
	hash2, err := utils.SampleHashFile(testFile, 2, 0.3, 64)
	if err != nil {
		t.Fatalf("SampleHashFile with custom chunk size failed: %v", err)
	}
	if hash2 == "" {
		t.Error("Expected non-empty hash with custom chunk size")
	}

	// Hash should be deterministic
	hash3, err := utils.SampleHashFile(testFile, 1, 0.5, 0)
	if err != nil {
		t.Fatalf("SampleHashFile for determinism test failed: %v", err)
	}
	if hash != hash3 {
		t.Error("Hash should be deterministic")
	}
}

func TestSampleHashFile_EmptyFile(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "empty.txt")
	err := os.WriteFile(testFile, []byte(""), 0o644)
	if err != nil {
		t.Fatalf("Failed to create empty test file: %v", err)
	}

	hash, err := utils.SampleHashFile(testFile, 1, 0.5, 0)
	if err != nil {
		t.Fatalf("SampleHashFile on empty file failed: %v", err)
	}
	if hash != "" {
		t.Errorf("Expected empty hash for empty file, got: %s", hash)
	}
}

func TestSampleHashFile_NonExistent(t *testing.T) {
	_, err := utils.SampleHashFile("/nonexistent/path/file.txt", 1, 0.5, 0)
	if err == nil {
		t.Error("Expected error for non-existent file")
	}
}

func TestFullHashFile(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "test_full_hash.txt")
	content := "Test content for full hashing"
	err := os.WriteFile(testFile, []byte(content), 0o644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	hash, err := utils.FullHashFile(testFile)
	if err != nil {
		t.Fatalf("FullHashFile failed: %v", err)
	}
	if hash == "" {
		t.Error("Expected non-empty hash")
	}

	// Verify hash is deterministic
	hash2, err := utils.FullHashFile(testFile)
	if err != nil {
		t.Fatalf("FullHashFile second call failed: %v", err)
	}
	if hash != hash2 {
		t.Error("Hash should be deterministic")
	}
}

func TestFullHashFile_NonExistent(t *testing.T) {
	_, err := utils.FullHashFile("/nonexistent/path/file.txt")
	if err == nil {
		t.Error("Expected error for non-existent file")
	}
}

func TestFilterDeleted(t *testing.T) {
	tmpDir := t.TempDir()

	// Create some test files
	existingFile := filepath.Join(tmpDir, "existing.txt")
	deletedFile := filepath.Join(tmpDir, "deleted.txt")

	err := os.WriteFile(existingFile, []byte("exists"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create existing file: %v", err)
	}

	// Test with mix of existing and non-existing files
	paths := []string{existingFile, deletedFile}
	result := utils.FilterDeleted(paths)

	if len(result) != 1 {
		t.Errorf("Expected 1 existing file, got %d", len(result))
	}
	if result[0] != existingFile {
		t.Errorf("Expected existing file in result, got %s", result[0])
	}
}

func TestFilterDeleted_WithDeletedDir(t *testing.T) {
	tmpDir := t.TempDir()

	// Create a subdirectory and file
	subDir := filepath.Join(tmpDir, "subdir")
	os.MkdirAll(subDir, 0o755)
	fileInSubdir := filepath.Join(subDir, "file.txt")
	err := os.WriteFile(fileInSubdir, []byte("test"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create file in subdir: %v", err)
	}

	// Remove the directory
	os.RemoveAll(subDir)

	// Test filtering - file in deleted dir should be filtered out
	paths := []string{fileInSubdir}
	result := utils.FilterDeleted(paths)

	if len(result) != 0 {
		t.Errorf("Expected 0 files after deleting parent dir, got %d", len(result))
	}
}

func TestGetFileStats(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "test_stats.txt")
	content := "Test content"
	err := os.WriteFile(testFile, []byte(content), 0o644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	stats, err := utils.GetFileStats(testFile)
	if err != nil {
		t.Fatalf("GetFileStats failed: %v", err)
	}

	if stats.Size != int64(len(content)) {
		t.Errorf("Expected size %d, got %d", len(content), stats.Size)
	}
	if stats.TimeCreated == 0 {
		t.Error("Expected non-zero creation time")
	}
	if stats.TimeModified == 0 {
		t.Error("Expected non-zero modification time")
	}
}

func TestGetFileStats_NonExistent(t *testing.T) {
	_, err := utils.GetFileStats("/nonexistent/path/file.txt")
	if err == nil {
		t.Error("Expected error for non-existent file")
	}
}

func TestIsFileOpen(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "test_open.txt")
	content := "Test content"
	err := os.WriteFile(testFile, []byte(content), 0o644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	// File should not be open
	if utils.IsFileOpen(testFile) {
		t.Error("File should not be reported as open when it's not")
	}

	// Open the file and test
	f, err := os.Open(testFile)
	if err != nil {
		t.Fatalf("Failed to open test file: %v", err)
	}
	defer f.Close()

	// On Linux, the file might be detected as open
	// On other platforms, this might not work
	// We just verify the function doesn't crash
	_ = utils.IsFileOpen(testFile)
}

func TestDetectMimeType(t *testing.T) {
	tmpDir := t.TempDir()

	// Test with a text file
	txtFile := filepath.Join(tmpDir, "test.txt")
	err := os.WriteFile(txtFile, []byte("Hello, World!"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	mime := utils.DetectMimeType(txtFile)
	if mime == "" {
		t.Error("Expected non-empty MIME type for text file")
	}

	// Test with .apk extension (special case)
	apkFile := filepath.Join(tmpDir, "test.apk")
	err = os.WriteFile(apkFile, []byte("fake apk"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create apk file: %v", err)
	}

	mime = utils.DetectMimeType(apkFile)
	if mime != "application/vnd.android.package-archive" {
		t.Errorf("Expected APK MIME type, got: %s", mime)
	}

	// Test with .zim extension (special case)
	zimFile := filepath.Join(tmpDir, "test.zim")
	err = os.WriteFile(zimFile, []byte("fake zim"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create zim file: %v", err)
	}

	mime = utils.DetectMimeType(zimFile)
	if mime != "application/x-zim" {
		t.Errorf("Expected ZIM MIME type, got: %s", mime)
	}
}

func TestDetectMimeType_NonExistent(t *testing.T) {
	mime := utils.DetectMimeType("/nonexistent/path/file.txt")
	if mime != "" {
		t.Errorf("Expected empty MIME type for non-existent file, got: %s", mime)
	}
}

func TestRename(t *testing.T) {
	tmpDir := t.TempDir()
	src := filepath.Join(tmpDir, "src.txt")
	dst := filepath.Join(tmpDir, "dst.txt")

	err := os.WriteFile(src, []byte("test"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create source file: %v", err)
	}

	err = utils.Rename(false, src, dst)
	if err != nil {
		t.Fatalf("Rename failed: %v", err)
	}

	// Verify file was renamed
	if _, statErr := os.Stat(src); !os.IsNotExist(statErr) {
		t.Error("Source file should not exist after rename")
	}
	if _, statErr := os.Stat(dst); os.IsNotExist(statErr) {
		t.Error("Destination file should exist after rename")
	}
}

func TestRename_Simulate(t *testing.T) {
	tmpDir := t.TempDir()
	src := filepath.Join(tmpDir, "src.txt")
	dst := filepath.Join(tmpDir, "dst.txt")

	err := os.WriteFile(src, []byte("test"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create source file: %v", err)
	}

	err = utils.Rename(true, src, dst)
	if err != nil {
		t.Fatalf("Rename in simulate mode failed: %v", err)
	}

	// Verify file was NOT renamed in simulate mode
	if _, statErr := os.Stat(src); os.IsNotExist(statErr) {
		t.Error("Source file should still exist in simulate mode")
	}
	if _, statErr := os.Stat(dst); !os.IsNotExist(statErr) {
		t.Error("Destination file should not exist in simulate mode")
	}
}

func TestUnlink(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "test.txt")

	err := os.WriteFile(testFile, []byte("test"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	err = utils.Unlink(false, testFile)
	if err != nil {
		t.Fatalf("Unlink failed: %v", err)
	}

	// Verify file was deleted
	if _, statErr := os.Stat(testFile); !os.IsNotExist(statErr) {
		t.Error("File should not exist after unlink")
	}
}

func TestUnlink_Simulate(t *testing.T) {
	tmpDir := t.TempDir()
	testFile := filepath.Join(tmpDir, "test.txt")

	err := os.WriteFile(testFile, []byte("test"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	err = utils.Unlink(true, testFile)
	if err != nil {
		t.Fatalf("Unlink in simulate mode failed: %v", err)
	}

	// Verify file was NOT deleted in simulate mode
	if _, statErr := os.Stat(testFile); os.IsNotExist(statErr) {
		t.Error("File should still exist in simulate mode")
	}
}

func TestRmtree(t *testing.T) {
	tmpDir := t.TempDir()
	testDir := filepath.Join(tmpDir, "testdir")

	err := os.MkdirAll(testDir, 0o755)
	if err != nil {
		t.Fatalf("Failed to create test directory: %v", err)
	}

	// Create a file inside
	testFile := filepath.Join(testDir, "file.txt")
	err = os.WriteFile(testFile, []byte("test"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	err = utils.Rmtree(false, testDir)
	if err != nil {
		t.Fatalf("Rmtree failed: %v", err)
	}

	// Verify directory was deleted
	if _, statErr := os.Stat(testDir); !os.IsNotExist(statErr) {
		t.Error("Directory should not exist after rmtree")
	}
}

func TestRmtree_Simulate(t *testing.T) {
	tmpDir := t.TempDir()
	testDir := filepath.Join(tmpDir, "testdir")

	err := os.MkdirAll(testDir, 0o755)
	if err != nil {
		t.Fatalf("Failed to create test directory: %v", err)
	}

	err = utils.Rmtree(true, testDir)
	if err != nil {
		t.Fatalf("Rmtree in simulate mode failed: %v", err)
	}

	// Verify directory was NOT deleted in simulate mode
	if _, statErr := os.Stat(testDir); os.IsNotExist(statErr) {
		t.Error("Directory should still exist in simulate mode")
	}
}

func TestAltName(t *testing.T) {
	tmpDir := t.TempDir()

	// Test with non-existent file - should return same path
	testFile := filepath.Join(tmpDir, "test.txt")
	result := utils.AltName(testFile)
	if result != testFile {
		t.Errorf("Expected same path for non-existent file, got %s", result)
	}

	// Create the file
	err := os.WriteFile(testFile, []byte("test"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create test file: %v", err)
	}

	// Test with existing file - should return alternative
	result = utils.AltName(testFile)
	expected := filepath.Join(tmpDir, "test_1.txt")
	if result != expected {
		t.Errorf("Expected %s, got %s", expected, result)
	}

	// Create the alternative file too
	err = os.WriteFile(result, []byte("test2"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create alternative file: %v", err)
	}

	// Test again - should increment counter
	result2 := utils.AltName(testFile)
	expected2 := filepath.Join(tmpDir, "test_2.txt")
	if result2 != expected2 {
		t.Errorf("Expected %s, got %s", expected2, result2)
	}
}

func TestCommonPath(t *testing.T) {
	tests := []struct {
		name     string
		paths    []string
		expected string
	}{
		{
			name:     "empty paths",
			paths:    []string{},
			expected: "",
		},
		{
			name:     "single path",
			paths:    []string{"/home/user/file.txt"},
			expected: "/home/user",
		},
		{
			name:     "common prefix",
			paths:    []string{"/home/user/docs/a.txt", "/home/user/docs/b.txt"},
			expected: "/home/user/docs",
		},
		{
			name:     "no common prefix",
			paths:    []string{"/home/user/a.txt", "/tmp/b.txt"},
			expected: "",
		},
		{
			name:     "different depths",
			paths:    []string{"/home/user/a.txt", "/home/user/docs/b.txt"},
			expected: "/home/user",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := utils.CommonPath(tt.paths)
			if result != tt.expected {
				t.Errorf("Expected %s, got %s", tt.expected, result)
			}
		})
	}
}

func TestCommonPathFull(t *testing.T) {
	// CommonPathFull currently just calls CommonPath
	paths := []string{"/home/user/docs/a.txt", "/home/user/docs/b.txt"}
	result := utils.CommonPathFull(paths)
	expected := "/home/user/docs"
	if result != expected {
		t.Errorf("Expected %s, got %s", expected, result)
	}
}

func TestCopyFile(t *testing.T) {
	tmpDir := t.TempDir()
	src := filepath.Join(tmpDir, "src.txt")
	dst := filepath.Join(tmpDir, "dst.txt")

	content := "Test content for copying"
	err := os.WriteFile(src, []byte(content), 0o644)
	if err != nil {
		t.Fatalf("Failed to create source file: %v", err)
	}

	err = utils.CopyFile(src, dst)
	if err != nil {
		t.Fatalf("CopyFile failed: %v", err)
	}

	// Verify content was copied
	copied, err := os.ReadFile(dst)
	if err != nil {
		t.Fatalf("Failed to read copied file: %v", err)
	}
	if string(copied) != content {
		t.Errorf("Content mismatch. Expected %s, got %s", content, string(copied))
	}
}

func TestCopyFile_NonExistent(t *testing.T) {
	tmpDir := t.TempDir()
	dst := filepath.Join(tmpDir, "dst.txt")

	err := utils.CopyFile("/nonexistent/src.txt", dst)
	if err == nil {
		t.Error("Expected error for non-existent source file")
	}
}

func TestCopyDir(t *testing.T) {
	tmpDir := t.TempDir()
	src := filepath.Join(tmpDir, "src")
	dst := filepath.Join(tmpDir, "dst")

	// Create source directory structure
	err := os.MkdirAll(filepath.Join(src, "subdir"), 0o755)
	if err != nil {
		t.Fatalf("Failed to create source directory: %v", err)
	}

	// Create test files
	err = os.WriteFile(filepath.Join(src, "file1.txt"), []byte("content1"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create file1: %v", err)
	}
	err = os.WriteFile(filepath.Join(src, "subdir", "file2.txt"), []byte("content2"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create file2: %v", err)
	}

	err = utils.CopyDir(src, dst)
	if err != nil {
		t.Fatalf("CopyDir failed: %v", err)
	}

	// Verify directory structure was copied
	if _, statErr := os.Stat(filepath.Join(dst, "subdir")); os.IsNotExist(statErr) {
		t.Error("Subdirectory should exist in destination")
	}

	// Verify files were copied
	content1, err := os.ReadFile(filepath.Join(dst, "file1.txt"))
	if err != nil {
		t.Fatalf("Failed to read copied file1: %v", err)
	}
	if string(content1) != "content1" {
		t.Errorf("File1 content mismatch")
	}

	content2, err := os.ReadFile(filepath.Join(dst, "subdir", "file2.txt"))
	if err != nil {
		t.Fatalf("Failed to read copied file2: %v", err)
	}
	if string(content2) != "content2" {
		t.Errorf("File2 content mismatch")
	}
}

func TestCopyDir_NonExistent(t *testing.T) {
	tmpDir := t.TempDir()
	dst := filepath.Join(tmpDir, "dst")

	err := utils.CopyDir("/nonexistent/src", dst)
	if err == nil {
		t.Error("Expected error for non-existent source directory")
	}
}

func TestGetExternalSubtitles(t *testing.T) {
	tmpDir := t.TempDir()

	// Create a media file
	mediaFile := filepath.Join(tmpDir, "movie.mp4")
	err := os.WriteFile(mediaFile, []byte("fake video"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create media file: %v", err)
	}

	// Create some subtitle files
	subFiles := []string{
		filepath.Join(tmpDir, "movie.srt"),
		filepath.Join(tmpDir, "movie.en.srt"),
		filepath.Join(tmpDir, "movie.vtt"),
		filepath.Join(tmpDir, "movie.ass"),
	}

	for _, subFile := range subFiles {
		err = os.WriteFile(subFile, []byte("subtitle content"), 0o644)
		if err != nil {
			t.Fatalf("Failed to create subtitle file: %v", err)
		}
	}

	// Create a non-matching file
	err = os.WriteFile(filepath.Join(tmpDir, "other.txt"), []byte("other"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create other file: %v", err)
	}

	subs := utils.GetExternalSubtitles(mediaFile)
	if len(subs) != 4 {
		t.Errorf("Expected 4 subtitle files, got %d", len(subs))
	}
}

func TestGetExternalSubtitles_NoSubtitles(t *testing.T) {
	tmpDir := t.TempDir()

	mediaFile := filepath.Join(tmpDir, "movie.mp4")
	err := os.WriteFile(mediaFile, []byte("fake video"), 0o644)
	if err != nil {
		t.Fatalf("Failed to create media file: %v", err)
	}

	subs := utils.GetExternalSubtitles(mediaFile)
	if len(subs) != 0 {
		t.Errorf("Expected 0 subtitle files, got %d", len(subs))
	}
}

func TestEnsureDir(t *testing.T) {
	tmpDir := t.TempDir()
	testDir := filepath.Join(tmpDir, "new", "nested", "directory")

	err := utils.EnsureDir(testDir)
	if err != nil {
		t.Fatalf("EnsureDir failed: %v", err)
	}

	// Verify directory was created
	info, err := os.Stat(testDir)
	if err != nil {
		t.Fatalf("Failed to stat created directory: %v", err)
	}
	if !info.IsDir() {
		t.Error("Expected directory")
	}
}

func TestEnsureDir_Existing(t *testing.T) {
	tmpDir := t.TempDir()

	err := utils.EnsureDir(tmpDir)
	if err != nil {
		t.Fatalf("EnsureDir failed for existing directory: %v", err)
	}
}
