package syncweb_test

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"net/http/httptest"
	"strconv"
	"testing"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
)

// TestRESTEngineBasic tests basic RESTEngine functionality
func TestRESTEngineBasic(t *testing.T) {
	// Create a test server that mimics Syncweb API
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/api/syncweb/status":
			w.Header().Set("Content-Type", "application/json")
			w.Write([]byte(`{"status": "Online", "offline": false}`))
		case "/api/syncweb/folders":
			w.Header().Set("Content-Type", "application/json")
			w.Write(
				[]byte(
					`[{"id": "test", "label": "Test", "path": "/tmp/test", "type": "sendreceive", "paused": false, "devices": [], "globalSize": {"Files": 0, "Bytes": 0}, "localSize": {"Files": 0, "Bytes": 0}, "needSize": {"Files": 0, "Bytes": 0}, "state": "idle", "completed": 0}]`,
				),
			)
		case "/api/syncweb/devices":
			w.Header().Set("Content-Type", "application/json")
			w.Write(
				[]byte(
					`[{"id": "TESTDEVICE", "name": "Test Device", "addresses": ["dynamic"], "introducer": false, "paused": false}]`,
				),
			)
		default:
			w.WriteHeader(http.StatusOK)
			w.Write([]byte(`{}`))
		}
	}))
	defer server.Close()

	engine := syncweb.NewRESTEngine("/tmp/test", server.URL, "test-token")

	// Test IsRunning
	if !engine.IsRunning() {
		t.Error("Expected IsRunning to return true")
	}

	// Test GetFolders
	folders := engine.GetFolders()
	if len(folders) != 1 {
		t.Errorf("Expected 1 folder, got %d", len(folders))
	}
	if folders[0].ID != "test" {
		t.Errorf("Expected folder ID 'test', got '%s'", folders[0].ID)
	}

	// Test GetDevices
	devices := engine.GetDevices()
	if len(devices) != 1 {
		t.Errorf("Expected 1 device, got %d", len(devices))
	}
	if devices[0].Name != "Test Device" {
		t.Errorf("Expected device name 'Test Device', got '%s'", devices[0].Name)
	}
}

// TestRESTEngineRetry tests retry logic
func TestRESTEngineRetry(t *testing.T) {
	attempts := 0
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		attempts++
		if attempts < 2 {
			panic(http.ErrAbortHandler)
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"status": "Online", "offline": false}`))
	}))
	defer server.Close()

	engine := syncweb.NewRESTEngine("/tmp/test", server.URL, "test-token")

	// Should succeed after retry
	if !engine.IsRunning() {
		t.Error("Expected IsRunning to return true after retry")
	}

	if attempts < 2 {
		t.Logf("Note: Retry occurred but attempts=%d (retry logic depends on network errors)", attempts)
	}
}

// TestRESTEngineCache tests caching behavior
func TestRESTEngineCache(t *testing.T) {
	requestCount := 0
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requestCount++
		if r.URL.Path == "/api/syncweb/devices" {
			w.Header().Set("Content-Type", "application/json")
			w.Write(
				[]byte(
					`[{"id": "TEST", "name": "Test", "addresses": ["dynamic"], "introducer": false, "paused": false}]`,
				),
			)
		}
	}))
	defer server.Close()

	engine := syncweb.NewRESTEngine("/tmp/test", server.URL, "test-token")

	// First call should hit the server
	devices1 := engine.GetDevices()
	if len(devices1) != 1 {
		t.Fatalf("Expected 1 device, got %d", len(devices1))
	}
	firstRequestCount := requestCount

	// Second call within cache window should use cache
	devices2 := engine.GetDevices()
	if len(devices2) != 1 {
		t.Fatalf("Expected 1 device, got %d", len(devices2))
	}

	// Request count should not have increased (cached)
	if requestCount != firstRequestCount {
		t.Errorf("Expected cached request (count=%d), but got new request (count=%d)", firstRequestCount, requestCount)
	}
}

// TestRESTEngineFolderOperations tests folder pause/resume operations
func TestRESTEngineFolderOperations(t *testing.T) {
	var lastPath string
	var lastBody map[string]any

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		lastPath = r.URL.Path
		// Decode body for POST requests
		if r.Method == http.MethodPost {
			decoder := json.NewDecoder(r.Body)
			decoder.Decode(&lastBody)
		}
		w.WriteHeader(http.StatusAccepted)
		w.Write([]byte(`{"status": "accepted"}`))
	}))
	defer server.Close()

	engine := syncweb.NewRESTEngine("/tmp/test", server.URL, "test-token")

	// Test PauseFolder
	if err := engine.PauseFolder("test-folder"); err != nil {
		t.Errorf("PauseFolder failed: %v", err)
	}
	if lastPath != "/api/syncweb/folders/pause" {
		t.Errorf("Expected path /api/syncweb/folders/pause, got %s", lastPath)
	}
	if lastBody["id"] != "test-folder" {
		t.Errorf("Expected folder ID 'test-folder' in body, got %v", lastBody["id"])
	}

	// Test ResumeFolder
	if err := engine.ResumeFolder("test-folder"); err != nil {
		t.Errorf("ResumeFolder failed: %v", err)
	}
	if lastPath != "/api/syncweb/folders/resume" {
		t.Errorf("Expected path /api/syncweb/folders/resume, got %s", lastPath)
	}
}

// TestRESTEngineDeviceOperations tests device pause/resume operations
func TestRESTEngineDeviceOperations(t *testing.T) {
	var lastPath string
	var lastBody map[string]any

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		lastPath = r.URL.Path
		if r.Method == http.MethodPost {
			decoder := json.NewDecoder(r.Body)
			decoder.Decode(&lastBody)
		}
		w.WriteHeader(http.StatusAccepted)
		w.Write([]byte(`{"status": "accepted"}`))
	}))
	defer server.Close()

	engine := syncweb.NewRESTEngine("/tmp/test", server.URL, "test-token")

	// Test PauseDevice
	if err := engine.PauseDevice("TESTDEVICE"); err != nil {
		t.Errorf("PauseDevice failed: %v", err)
	}
	if lastPath != "/api/syncweb/devices/pause" {
		t.Errorf("Expected path /api/syncweb/devices/pause, got %s", lastPath)
	}

	// Test ResumeDevice
	if err := engine.ResumeDevice("TESTDEVICE"); err != nil {
		t.Errorf("ResumeDevice failed: %v", err)
	}
	if lastPath != "/api/syncweb/devices/resume" {
		t.Errorf("Expected path /api/syncweb/devices/resume, got %s", lastPath)
	}
}

// TestRESTEngineSetDeviceAddresses tests setting device addresses
func TestRESTEngineSetDeviceAddresses(t *testing.T) {
	var lastBody map[string]any

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method == http.MethodPost {
			decoder := json.NewDecoder(r.Body)
			decoder.Decode(&lastBody)
		}
		w.WriteHeader(http.StatusAccepted)
		w.Write([]byte(`{"status": "accepted"}`))
	}))
	defer server.Close()

	engine := syncweb.NewRESTEngine("/tmp/test", server.URL, "test-token")

	addresses := []string{"tcp://192.168.1.1:22000", "dynamic"}
	if err := engine.SetDeviceAddresses("TESTDEVICE", addresses); err != nil {
		t.Errorf("SetDeviceAddresses failed: %v", err)
	}

	if lastBody["id"] != "TESTDEVICE" {
		t.Errorf("Expected device ID 'TESTDEVICE', got %v", lastBody["id"])
	}

	bodyAddresses, ok := lastBody["addresses"].([]any)
	if !ok {
		t.Fatalf("Expected addresses array in body")
	}
	if len(bodyAddresses) != 2 {
		t.Errorf("Expected 2 addresses, got %d", len(bodyAddresses))
	}
}

// TestRESTEngineWaitUntilIdle tests the WaitUntilIdle method
func TestRESTEngineWaitUntilIdle(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/api/syncweb/idle" {
			w.Header().Set("Content-Type", "application/json")
			w.Write([]byte(`{"idle": true}`))
		}
	}))
	defer server.Close()

	engine := syncweb.NewRESTEngine("/tmp/test", server.URL, "test-token")

	// Should return nil when idle
	if err := engine.WaitUntilIdle("test-folder", 5*time.Second); err != nil {
		t.Errorf("WaitUntilIdle failed: %v", err)
	}
}

// TestRESTEngineReadSeeker tests the RESTReadSeeker implementation
func TestRESTEngineReadSeeker(t *testing.T) {
	testContent := "Hello, Syncweb!"

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/api/raw" {
			w.Header().Set("Content-Type", "application/octet-stream")
			w.Header().Set("Content-Length", strconv.Itoa(len(testContent)))
			w.Write([]byte(testContent))
		}
	}))
	defer server.Close()

	engine := syncweb.NewRESTEngine("/tmp/test", server.URL, "test-token")

	rs, err := engine.NewReadSeeker(context.Background(), "test-folder", "test.txt")
	if err != nil {
		t.Fatalf("NewReadSeeker failed: %v", err)
	}

	// Read all content
	buf := make([]byte, 100)
	n, err := rs.Read(buf)
	if err != nil && !errors.Is(err, io.EOF) {
		t.Errorf("Read failed: %v", err)
	}
	if string(buf[:n]) != testContent {
		t.Errorf("Expected '%s', got '%s'", testContent, string(buf[:n]))
	}

	// Test Seek
	offset, err := rs.Seek(0, io.SeekStart)
	if err != nil {
		t.Errorf("Seek failed: %v", err)
	}
	if offset != 0 {
		t.Errorf("Expected offset 0, got %d", offset)
	}
}

// TestRESTEngineIgnoresOperations tests ignore-related operations
func TestRESTEngineIgnoresOperations(t *testing.T) {
	var lastPath string
	var lastBody map[string]any

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		lastPath = r.URL.Path
		if r.Method == http.MethodPost {
			decoder := json.NewDecoder(r.Body)
			decoder.Decode(&lastBody)
		}

		if r.URL.Path == "/api/syncweb/ignores" {
			w.Header().Set("Content-Type", "application/json")
			w.Write([]byte(`{"ignore": []}`))
		} else {
			w.WriteHeader(http.StatusAccepted)
			w.Write([]byte(`{"status": "accepted"}`))
		}
	}))
	defer server.Close()

	engine := syncweb.NewRESTEngine("/tmp/test", server.URL, "test-token")

	// Test GetIgnores
	ignores, err := engine.GetIgnores("test-folder")
	if err != nil {
		t.Errorf("GetIgnores failed: %v", err)
	}
	if ignores == nil {
		t.Error("Expected non-nil ignores slice")
	}

	// Test AddIgnores
	if err := engine.AddIgnores("test-folder", []string{"*.tmp"}); err != nil {
		t.Errorf("AddIgnores failed: %v", err)
	}
	if lastPath != "/api/syncweb/ignores/add" {
		t.Errorf("Expected path /api/syncweb/ignores/add, got %s", lastPath)
	}

	// Test SetIgnores
	if err := engine.SetIgnores("test-folder", []string{"*.log"}); err != nil {
		t.Errorf("SetIgnores failed: %v", err)
	}
	if lastPath != "/api/syncweb/ignores" {
		t.Errorf("Expected path /api/syncweb/ignores, got %s", lastPath)
	}
}

// TestRESTEngineFolderDevicesOperations tests folder device management
func TestRESTEngineFolderDevicesOperations(t *testing.T) {
	var lastPath string
	var lastBody map[string]any

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		lastPath = r.URL.Path
		if r.Method == http.MethodPost {
			decoder := json.NewDecoder(r.Body)
			decoder.Decode(&lastBody)
		}
		w.WriteHeader(http.StatusAccepted)
		w.Write([]byte(`{"status": "accepted"}`))
	}))
	defer server.Close()

	engine := syncweb.NewRESTEngine("/tmp/test", server.URL, "test-token")

	// Test AddFolderDevice
	if err := engine.AddFolderDevice("test-folder", "DEVICE1"); err != nil {
		t.Errorf("AddFolderDevice failed: %v", err)
	}
	if lastPath != "/api/syncweb/folders/join" {
		t.Errorf("Expected path /api/syncweb/folders/join, got %s", lastPath)
	}

	// Test AddFolderDevices
	if err := engine.AddFolderDevices("test-folder", []string{"DEVICE2", "DEVICE3"}); err != nil {
		t.Errorf("AddFolderDevices failed: %v", err)
	}

	// Test RemoveFolderDevices
	if err := engine.RemoveFolderDevices("test-folder", []string{"DEVICE1"}); err != nil {
		t.Errorf("RemoveFolderDevices failed: %v", err)
	}
	if lastPath != "/api/syncweb/folders/remove-devices" {
		t.Errorf("Expected path /api/syncweb/folders/remove-devices, got %s", lastPath)
	}
}

// TestRESTEngineResolveLocalPath tests ResolveLocalPath
func TestRESTEngineResolveLocalPath(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/api/syncweb/folders" {
			w.Header().Set("Content-Type", "application/json")
			w.Write(
				[]byte(
					`[{"id": "test", "label": "Test", "path": "/tmp/test", "type": "sendreceive", "paused": false, "devices": [], "globalSize": {"Files": 0, "Bytes": 0}, "localSize": {"Files": 0, "Bytes": 0}, "needSize": {"Files": 0, "Bytes": 0}, "state": "idle", "completed": 0}]`,
				),
			)
		}
	}))
	defer server.Close()

	engine := syncweb.NewRESTEngine("/tmp/test", server.URL, "test-token")

	folderID, localPath, err := engine.ResolveLocalPath("sync://test/file.txt")
	if err != nil {
		t.Errorf("ResolveLocalPath failed: %v", err)
	}
	if folderID != "test" {
		t.Errorf("Expected folder ID 'test', got '%s'", folderID)
	}
	if localPath != "/tmp/test" {
		t.Errorf("Expected local path '/tmp/test', got '%s'", localPath)
	}
}
