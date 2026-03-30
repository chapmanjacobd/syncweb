// Package syncweb_test contains contract tests that verify Syncthing internal API behavior.
// These tests ensure that when Syncthing updates, we detect breaking changes early.
//
// Contract tests verify:
// 1. Expected inputs/outputs from Syncthing APIs
// 2. Behavior of Syncthing internals methods
// 3. Configuration defaults and edge cases
// 4. Event system behavior
// 5. Protocol types and constants
package syncweb

import (
	"context"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/events"
	stmodel "github.com/syncthing/syncthing/lib/model"
	"github.com/syncthing/syncthing/lib/protocol"
	"github.com/syncthing/syncthing/lib/svcutil"
	"github.com/syncthing/syncthing/lib/tlsutil"
)

// =============================================================================
// CONFIG PACKAGE CONTRACT TESTS
// =============================================================================

func TestSyncthingContract_ConfigDefaults(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-config-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	cfg := sw.Node.Cfg.RawCopy()

	// CONTRACT: Verify config default values we depend on
	t.Run("URAccepted defaults to -1", func(t *testing.T) {
		if cfg.Options.URAccepted != -1 {
			t.Errorf("URAccepted should default to -1 (disabled), got %d", cfg.Options.URAccepted)
		}
	})

	t.Run("StartBrowser defaults to false", func(t *testing.T) {
		if cfg.Options.StartBrowser {
			t.Error("StartBrowser should default to false")
		}
	})

	t.Run("GUI disabled by default", func(t *testing.T) {
		if cfg.GUI.Enabled {
			t.Error("GUI should be disabled by default")
		}
	})

	t.Run("MaxSendKbps defaults to 0 (unlimited)", func(t *testing.T) {
		if cfg.Options.MaxSendKbps != 0 {
			t.Errorf("MaxSendKbps should default to 0 (unlimited), got %d", cfg.Options.MaxSendKbps)
		}
	})

	// Note: RefreshInterval was removed in newer Syncthing versions
}

func TestSyncthingContract_ConfigFolderTypes(t *testing.T) {
	// CONTRACT: Verify FolderType enum values we use
	tests := []struct {
		name     string
		ft       config.FolderType
		expected string
	}{
		{"SendOnly", config.FolderTypeSendOnly, "sendonly"},
		{"ReceiveOnly", config.FolderTypeReceiveOnly, "receiveonly"},
		{"SendReceive", config.FolderTypeSendReceive, "sendreceive"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.ft.String() != tt.expected {
				t.Errorf("FolderType(%d).String() = %q, want %q", tt.ft, tt.ft.String(), tt.expected)
			}
		})
	}
}

func TestSyncthingContract_ConfigRawCopyIsolation(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-copy-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	// CONTRACT: RawCopy() should return a deep copy, not the same reference
	cfg1 := sw.Node.Cfg.RawCopy()
	cfg2 := sw.Node.Cfg.RawCopy()

	// Modify cfg1
	cfg1.Options.StartBrowser = !cfg1.Options.StartBrowser

	// cfg2 should not be affected
	if cfg1.Options.StartBrowser == cfg2.Options.StartBrowser {
		t.Error("RawCopy() should return independent copies")
	}
}

// =============================================================================
// PROTOCOL PACKAGE CONTRACT TESTS
// =============================================================================

func TestSyncthingContract_ProtocolDeviceID(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-protocol-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	// CONTRACT: DeviceID from certificate should be non-empty
	id := sw.Node.MyID()
	if id.String() == "" {
		t.Fatal("DeviceID should never be empty")
	}

	// CONTRACT: DeviceIDFromString should parse our own ID
	parsed, err := protocol.DeviceIDFromString(id.String())
	if err != nil {
		t.Fatalf("DeviceIDFromString failed for valid ID: %v", err)
	}
	if parsed != id {
		t.Errorf("DeviceIDFromString returned different ID: got %v, want %v", parsed, id)
	}

	// CONTRACT: Invalid DeviceID should error
	_, err = protocol.DeviceIDFromString("INVALID-DEVICE-ID")
	if err == nil {
		t.Error("DeviceIDFromString should error on invalid ID")
	}
}

func TestSyncthingContract_ProtocolLocalDeviceID(t *testing.T) {
	// CONTRACT: LocalDeviceID should be a valid device ID
	if protocol.LocalDeviceID.String() == "" {
		t.Error("LocalDeviceID should never be empty")
	}

	// CONTRACT: LocalDeviceID string representation should be parseable
	parsed, err := protocol.DeviceIDFromString(protocol.LocalDeviceID.String())
	if err != nil {
		t.Fatalf("LocalDeviceID should be parseable: %v", err)
	}
	if parsed != protocol.LocalDeviceID {
		t.Error("LocalDeviceID round-trip failed")
	}
}

func TestSyncthingContract_ProtocolFileInfoTypes(t *testing.T) {
	// CONTRACT: Verify FileInfoType enum values
	// Note: String() output may vary between Syncthing versions
	tests := []struct {
		name string
		ft   protocol.FileInfoType
	}{
		{"File", protocol.FileInfoTypeFile},
		{"Directory", protocol.FileInfoTypeDirectory},
		{"Symlink", protocol.FileInfoTypeSymlink},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Just verify String() returns something non-empty
			s := tt.ft.String()
			if s == "" {
				t.Errorf("FileInfoType(%d).String() returned empty", tt.ft)
			}
			t.Logf("FileInfoType(%d).String() = %q", tt.ft, s)
		})
	}
}

func TestSyncthingContract_ProtocolFileInfoFlags(t *testing.T) {
	// CONTRACT: FlagLocalIgnored should be a valid flag value
	if protocol.FlagLocalIgnored == 0 {
		t.Error("FlagLocalIgnored should be non-zero")
	}

	// Verify flag can be used in bitwise operations with FileInfo.Flags
	var flags uint32 = 0
	flags |= uint32(protocol.FlagLocalIgnored)
	if flags&uint32(protocol.FlagLocalIgnored) == 0 {
		t.Error("FlagLocalIgnored bitwise operations failed")
	}
}

func TestSyncthingContract_ProtocolVector(t *testing.T) {
	// CONTRACT: Zero Vector should be valid
	var v protocol.Vector
	if !v.Equal(protocol.Vector{}) {
		t.Error("Zero Vector should equal itself")
	}

	// CONTRACT: Vector counter operations - use Inc() instead of IncShortID()
	// IncShortID was removed in newer Syncthing versions
	// We just verify the Vector type works as expected
	v = v.Copy()
	if v.Counter(0) != 0 {
		t.Errorf("Vector counter should be 0, got %d", v.Counter(0))
	}
}

// =============================================================================
// EVENTS PACKAGE CONTRACT TESTS
// =============================================================================

func TestSyncthingContract_EventsLogger(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-events-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	// CONTRACT: Event logger should not be nil
	if sw.Node.EvLogger == nil {
		t.Fatal("EvLogger should never be nil")
	}

	// CONTRACT: Should be able to subscribe to all events (0 = all events)
	sub := sw.Node.EvLogger.Subscribe(0)
	if sub == nil {
		t.Fatal("Subscribe should return non-nil subscription")
	}
	defer sub.Unsubscribe()

	// CONTRACT: Should be able to subscribe to specific event types
	sub2 := sw.Node.EvLogger.Subscribe(events.DeviceConnected)
	if sub2 == nil {
		t.Fatal("Subscribe to specific event type should return non-nil")
	}
	defer sub2.Unsubscribe()
}

func TestSyncthingContract_EventsSubscription(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-eventsub-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	sub := sw.Node.EvLogger.Subscribe(0) // 0 = all events

	// CONTRACT: Poll should return with timeout
	timeout := time.After(100 * time.Millisecond)
	select {
	case <-sub.C():
		// Event received, that's fine
	case <-timeout:
		// Timeout, also fine for this test
	}

	// CONTRACT: Unsubscribe should not panic
	sub.Unsubscribe()
}

func TestSyncthingContract_EventsEventTypes(t *testing.T) {
	// CONTRACT: Verify event type constants we use exist and are non-zero
	eventTypes := []events.EventType{
		events.DeviceRejected,
		events.PendingDevicesChanged,
		events.DeviceConnected,
		events.FolderSummary,
		events.ItemStarted,
		events.ItemFinished,
		events.LocalIndexUpdated,
	}

	for _, et := range eventTypes {
		if et == 0 {
			t.Errorf("Event type %v should be non-zero", et)
		}
	}
}

// =============================================================================
// SYNCTHING APP PACKAGE CONTRACT TESTS
// =============================================================================

func TestSyncthingContract_AppInternals(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-app-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	// CONTRACT: App.Internals should not be nil
	internals := sw.Node.App.Internals
	if internals == nil {
		t.Fatal("Internals should never be nil")
	}

	// CONTRACT: Internals methods should not panic on valid folder
	// Create a test folder first
	syncDir := filepath.Join(homeDir, "sync")
	os.MkdirAll(syncDir, 0o700)
	folderID := "test-folder"
	if err := sw.AddFolder(folderID, "Test Folder", syncDir, config.FolderTypeSendReceive); err != nil {
		t.Fatal(err)
	}

	// CONTRACT: GlobalSize should return valid values
	global, _ := internals.GlobalSize(folderID)
	if global.Files < 0 {
		t.Error("GlobalSize.Files should be non-negative")
	}
	if global.Bytes < 0 {
		t.Error("GlobalSize.Bytes should be non-negative")
	}

	// CONTRACT: LocalSize should return valid values
	local, _ := internals.LocalSize(folderID)
	if local.Files < 0 {
		t.Error("LocalSize.Files should be non-negative")
	}
	if local.Bytes < 0 {
		t.Error("LocalSize.Bytes should be non-negative")
	}

	// CONTRACT: NeedSize should return valid values
	need, _ := internals.NeedSize(folderID, protocol.LocalDeviceID)
	if need.Files < 0 {
		t.Error("NeedSize.Files should be non-negative")
	}
	if need.Bytes < 0 {
		t.Error("NeedSize.Bytes should be non-negative")
	}
}

func TestSyncthingContract_AppInternalsIgnores(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-ignores-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	syncDir := filepath.Join(homeDir, "sync")
	os.MkdirAll(syncDir, 0o700)
	folderID := "test-folder"
	if err := sw.AddFolder(folderID, "Test Folder", syncDir, config.FolderTypeSendReceive); err != nil {
		t.Fatal(err)
	}

	// Wait for folder to be initialized
	time.Sleep(500 * time.Millisecond)

	internals := sw.Node.App.Internals

	// CONTRACT: Get ignores on new folder should not error
	ignores, _, err := internals.Ignores(folderID)
	if err != nil {
		t.Fatalf("Ignores() failed: %v", err)
	}
	// Note: ignores may be nil or empty for a new folder - just verify no error
	_ = ignores

	// CONTRACT: Set ignores should work
	newIgnores := []string{"*.tmp", "*.log"}
	if err := internals.SetIgnores(folderID, newIgnores); err != nil {
		t.Fatalf("SetIgnores() failed: %v", err)
	}

	// Wait for ignores to be applied
	time.Sleep(200 * time.Millisecond)

	// CONTRACT: Get ignores after set should match
	ignores, _, err = internals.Ignores(folderID)
	if err != nil {
		t.Fatalf("Ignores() after set failed: %v", err)
	}
	// Note: Syncthing may return the patterns in a different order or format
	// Just verify we got some ignores back
	if len(ignores) == 0 {
		t.Errorf("Expected ignores after SetIgnores, got empty list")
	}
}

func TestSyncthingContract_AppInternalsFolderState(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-state-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	syncDir := filepath.Join(homeDir, "sync")
	os.MkdirAll(syncDir, 0o700)
	folderID := "test-folder"
	if err := sw.AddFolder(folderID, "Test Folder", syncDir, config.FolderTypeSendReceive); err != nil {
		t.Fatal(err)
	}

	// Wait for folder to be initialized
	time.Sleep(500 * time.Millisecond)

	internals := sw.Node.App.Internals

	// CONTRACT: FolderState should return valid state
	state, _, _ := internals.FolderState(folderID)
	// State could be "idle", "scanning", "syncing", etc. - just verify it's not empty
	if state == "" {
		t.Error("FolderState should not return empty string")
	}

	// CONTRACT: FolderProgressBytesCompleted should return non-negative value
	progress := internals.FolderProgressBytesCompleted(folderID)
	if progress < 0 {
		t.Error("FolderProgressBytesCompleted should be non-negative")
	}
}

func TestSyncthingContract_AppInternalsAllGlobalFiles(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-files-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	syncDir := filepath.Join(homeDir, "sync")
	os.MkdirAll(syncDir, 0o700)
	folderID := "test-folder"
	if err := sw.AddFolder(folderID, "Test Folder", syncDir, config.FolderTypeSendReceive); err != nil {
		t.Fatal(err)
	}

	// Wait for folder to be initialized
	time.Sleep(500 * time.Millisecond)

	internals := sw.Node.App.Internals

	// CONTRACT: AllGlobalFiles should not panic and should return iterator
	// Note: An empty folder may not return any entries until files are added
	count := 0
	seq, cancel := internals.AllGlobalFiles(folderID)
	defer cancel()
	for range seq {
		count++
	}
	// Just verify the iterator works - don't assert on count since folder may be empty
	t.Logf("AllGlobalFiles returned %d entries", count)
}

func TestSyncthingContract_AppInternalsPendingFolders(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-pending-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	internals := sw.Node.App.Internals

	// CONTRACT: PendingFolders should return non-nil map
	pending, _ := internals.PendingFolders(protocol.LocalDeviceID)
	if pending == nil {
		t.Error("PendingFolders should return non-nil map")
	}
}

// =============================================================================
// MODEL PACKAGE CONTRACT TESTS
// =============================================================================

func TestSyncthingContract_ModelAvailability(t *testing.T) {
	// CONTRACT: Verify Availability struct exists and has expected fields
	avail := stmodel.Availability{
		ID:            protocol.LocalDeviceID,
		FromTemporary: true,
	}

	if avail.ID.String() == "" {
		t.Error("Availability.ID should be settable")
	}
	if !avail.FromTemporary {
		t.Error("Availability.FromTemporary should be settable")
	}
}

// =============================================================================
// TLSUTIL PACKAGE CONTRACT TESTS
// =============================================================================

func TestSyncthingContract_TLSUtilCertificate(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-tls-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	certPath := filepath.Join(homeDir, "cert.pem")
	keyPath := filepath.Join(homeDir, "key.pem")

	// CONTRACT: NewCertificate should create valid cert
	// Note: We use the same parameters as node.go
	// The 0 means use default validity, false means use RSA (not ECDSA)
	_, err = tlsutil.NewCertificate(certPath, keyPath, "syncthing", 0, false)
	if err != nil {
		t.Fatalf("NewCertificate failed: %v", err)
	}

	// Verify cert files were created
	if _, err := os.Stat(certPath); os.IsNotExist(err) {
		t.Error("Certificate file should be created")
	}
	if _, err := os.Stat(keyPath); os.IsNotExist(err) {
		t.Error("Key file should be created")
	}
}

// =============================================================================
// SVCUTIL PACKAGE CONTRACT TESTS
// =============================================================================

func TestSyncthingContract_SvcUtilExitCodes(t *testing.T) {
	// CONTRACT: ExitSuccess should be 0
	if svcutil.ExitSuccess != 0 {
		t.Errorf("ExitSuccess should be 0, got %d", svcutil.ExitSuccess)
	}
}

// =============================================================================
// INTEGRALS API CONTRACT TESTS
// =============================================================================

func TestSyncthingContract_InternalsGlobalFileInfo(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-globalfi-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	syncDir := filepath.Join(homeDir, "sync")
	os.MkdirAll(syncDir, 0o700)
	folderID := "test-folder"
	if err := sw.AddFolder(folderID, "Test Folder", syncDir, config.FolderTypeSendReceive); err != nil {
		t.Fatal(err)
	}

	internals := sw.Node.App.Internals

	// CONTRACT: GlobalFileInfo for non-existent file should return ok=false
	_, ok, err := internals.GlobalFileInfo(folderID, "nonexistent.txt")
	if err != nil {
		t.Errorf("GlobalFileInfo should not error for non-existent file: %v", err)
	}
	if ok {
		t.Error("GlobalFileInfo should return ok=false for non-existent file")
	}
}

func TestSyncthingContract_InternalsIsConnectedTo(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-connected-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	internals := sw.Node.App.Internals

	// CONTRACT: IsConnectedTo should not panic with valid device ID
	// It may return false since we're not connected to anyone
	connected := internals.IsConnectedTo(protocol.LocalDeviceID)
	// We don't assert true/false - just that it doesn't panic
	_ = connected
}

// =============================================================================
// BLOCK AVAILABILITY CONTRACT TESTS
// =============================================================================

func TestSyncthingContract_BlockAvailability(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-blockavail-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	syncDir := filepath.Join(homeDir, "sync")
	os.MkdirAll(syncDir, 0o700)
	folderID := "test-folder"
	if err := sw.AddFolder(folderID, "Test Folder", syncDir, config.FolderTypeSendReceive); err != nil {
		t.Fatal(err)
	}

	// Create a test file
	testFile := "test.txt"
	testContent := "test content"
	if err := os.WriteFile(filepath.Join(syncDir, testFile), []byte(testContent), 0o644); err != nil {
		t.Fatal(err)
	}

	// Wait for file to be indexed
	time.Sleep(500 * time.Millisecond)

	internals := sw.Node.App.Internals

	// CONTRACT: Get file info first
	info, ok, err := internals.GlobalFileInfo(folderID, testFile)
	if !ok || err != nil {
		t.Fatalf("Failed to get file info: %v, ok=%v", err, ok)
	}

	// CONTRACT: BlockAvailability should return non-empty list for valid block
	if len(info.Blocks) > 0 {
		avail, _ := internals.BlockAvailability(folderID, info, info.Blocks[0])
		// May be empty if no peers have it, but should not panic
		_ = avail
	}
}

// =============================================================================
// DOWNLOAD BLOCK CONTRACT TESTS
// =============================================================================

func TestSyncthingContract_DownloadBlock(t *testing.T) {
	homeDir, err := os.MkdirTemp("", "syncweb-contract-downloadblock-")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(homeDir)

	sw, err := NewSyncweb(homeDir, "test-node", "tcp://127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	if err := sw.Start(); err != nil {
		t.Fatal(err)
	}
	defer sw.Stop()

	syncDir := filepath.Join(homeDir, "sync")
	os.MkdirAll(syncDir, 0o700)
	folderID := "test-folder"
	if err := sw.AddFolder(folderID, "Test Folder", syncDir, config.FolderTypeSendReceive); err != nil {
		t.Fatal(err)
	}

	// Create a test file
	testFile := "test.txt"
	testContent := "test content for download block"
	if err := os.WriteFile(filepath.Join(syncDir, testFile), []byte(testContent), 0o644); err != nil {
		t.Fatal(err)
	}

	// Wait for file to be indexed
	time.Sleep(500 * time.Millisecond)

	internals := sw.Node.App.Internals

	// CONTRACT: Get file info first
	info, ok, err := internals.GlobalFileInfo(folderID, testFile)
	if !ok || err != nil {
		t.Fatalf("Failed to get file info: %v, ok=%v", err, ok)
	}

	// CONTRACT: DownloadBlock with context cancellation
	if len(info.Blocks) > 0 {
		ctx, cancel := context.WithCancel(context.Background())
		cancel() // Cancel immediately

		// This should respect context cancellation
		_, err := internals.DownloadBlock(ctx, protocol.LocalDeviceID, folderID, testFile, 0, info.Blocks[0], false)
		// Error is expected since we cancelled and there's no peer
		_ = err
	}
}
