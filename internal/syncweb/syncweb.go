package syncweb

import (
	"context"
	"fmt"
	"io"
	"log/slog"
	"math"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"sync"
	"time"

	"github.com/chapmanjacobd/syncweb/internal/models"
	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/events"
	stmodel "github.com/syncthing/syncthing/lib/model"
	"github.com/syncthing/syncthing/lib/protocol"
)

// Constants for syncweb configuration
const (
	// EventBufferLimit is the maximum number of events to keep in memory
	EventBufferLimit = 100
)

type Measurement struct {
	TotalTime time.Duration
	Count     int64
	Errors    int64
}

type DeviceInfo struct {
	ID         string   `json:"id"`
	Name       string   `json:"name"`
	Addresses  []string `json:"addresses"`
	Introducer bool     `json:"introducer"`
	Paused     bool     `json:"paused"`
}

type FolderInfo struct {
	ID      string   `json:"id"`
	Label   string   `json:"label"`
	Path    string   `json:"path"`
	Type    string   `json:"type"`
	Paused  bool     `json:"paused"`
	Devices []string `json:"devices"`
}

type Measurements struct {
	mutex sync.RWMutex
	data  map[protocol.DeviceID]*Measurement
}

func NewMeasurements() *Measurements {
	return &Measurements{
		data: make(map[protocol.DeviceID]*Measurement),
	}
}

func (m *Measurements) Record(id protocol.DeviceID, duration time.Duration, err error) {
	m.mutex.Lock()
	defer m.mutex.Unlock()

	if _, ok := m.data[id]; !ok {
		m.data[id] = &Measurement{}
	}
	meas := m.data[id]
	if err != nil {
		meas.Errors++
	} else {
		meas.TotalTime += duration
		meas.Count++
	}
}

func (m *Measurements) Score(id protocol.DeviceID) float64 {
	m.mutex.RLock()
	defer m.mutex.RUnlock()

	meas, ok := m.data[id]
	if !ok || meas.Count == 0 {
		return 0 // Neutral score for new peers
	}

	avgTime := float64(meas.TotalTime) / float64(meas.Count)
	errorRate := float64(meas.Errors) / float64(meas.Count+meas.Errors)

	// Lower is better. Penalty for errors
	return avgTime * (1.0 + errorRate*10.0)
}

type Syncweb struct {
	Node           *Node
	Measurements   *Measurements
	pendingDevices sync.Map // map[protocol.DeviceID]time.Time
	events         []models.SyncEvent
	eventsMu       sync.RWMutex
	eventsCache    []models.SyncEvent
	eventsSeq      uint64
	cacheSeq       uint64
	eventSub       events.Subscription
	eventSubMu     sync.Mutex
}

func NewSyncweb(homeDir string, name string, listenAddr string) (*Syncweb, error) {
	node, err := NewNode(homeDir, name, listenAddr)
	if err != nil {
		return nil, err
	}

	s := &Syncweb{
		Node:         node,
		Measurements: NewMeasurements(),
		events:       make([]models.SyncEvent, 0),
	}

	go s.watchEvents()

	return s, nil
}

func (s *Syncweb) addEvent(evType string, message string, data any) {
	s.eventsMu.Lock()
	defer s.eventsMu.Unlock()

	event := models.SyncEvent{
		Time:    time.Now().Format(time.RFC3339),
		Type:    evType,
		Message: message,
		Data:    data,
	}

	s.events = append(s.events, event)
	if len(s.events) > EventBufferLimit {
		s.events = s.events[1:]
	}
	s.eventsSeq++
}

func (s *Syncweb) GetEvents() []models.SyncEvent {
	s.eventsMu.RLock()
	if s.eventsCache != nil && s.cacheSeq == s.eventsSeq {
		res := s.eventsCache
		s.eventsMu.RUnlock()
		return res
	}
	s.eventsMu.RUnlock()

	s.eventsMu.Lock()
	defer s.eventsMu.Unlock()

	// Re-check after acquiring write lock
	if s.eventsCache != nil && s.cacheSeq == s.eventsSeq {
		return s.eventsCache
	}

	// Return a copy to avoid data races
	res := make([]models.SyncEvent, len(s.events))
	copy(res, s.events)
	s.eventsCache = res
	s.cacheSeq = s.eventsSeq
	return res
}

func (s *Syncweb) watchEvents() {
	mask := events.DeviceRejected | events.PendingDevicesChanged | events.DeviceConnected |
		events.FolderSummary | events.ItemStarted | events.ItemFinished | events.LocalIndexUpdated

	sub := s.Node.Subscribe(mask)

	s.eventSubMu.Lock()
	s.eventSub = sub
	s.eventSubMu.Unlock()

	defer func() {
		s.eventSubMu.Lock()
		if s.eventSub != nil {
			s.eventSub.Unsubscribe()
			s.eventSub = nil
		}
		s.eventSubMu.Unlock()
	}()

	for {
		select {
		case ev := <-sub.C():
			switch ev.Type { //nolint:exhaustive // only handle specific events
			case events.DeviceRejected:
				var deviceIDStr string
				if m, ok := ev.Data.(map[string]any); ok {
					if idStr, ok := m["device"].(string); ok {
						deviceIDStr = idStr
					}
				} else if m, ok := ev.Data.(map[string]string); ok {
					deviceIDStr = m["device"]
				}

				if deviceIDStr != "" {
					if id, err := protocol.DeviceIDFromString(deviceIDStr); err == nil {
						s.pendingDevices.Store(id, ev.Time)
						slog.Info("Device rejected (pending)", "id", id)
						s.addEvent("DeviceRejected", "New device request: "+deviceIDStr, ev.Data)
					}
				}
			case events.DeviceConnected:
				var deviceIDStr string
				if m, ok := ev.Data.(map[string]any); ok {
					if idStr, ok := m["id"].(string); ok {
						deviceIDStr = idStr
					}
				} else if m, ok := ev.Data.(map[string]string); ok {
					deviceIDStr = m["id"]
				}

				if deviceIDStr != "" {
					if id, err := protocol.DeviceIDFromString(deviceIDStr); err == nil {
						s.pendingDevices.Delete(id)
						s.addEvent("DeviceConnected", "Device connected: "+deviceIDStr, ev.Data)
					}
				}
			case events.ItemStarted:
				if m, ok := ev.Data.(map[string]any); ok {
					if item, ok := m["item"].(string); ok {
						s.addEvent("ItemStarted", "Syncing: "+item, ev.Data)
					}
				}
			case events.ItemFinished:
				if m, ok := ev.Data.(map[string]any); ok {
					item, _ := m["item"].(string)
					err, _ := m["error"].(string)
					msg := "Finished: " + item
					if err != "" {
						msg += " (Error: " + err + ")"
					}
					s.addEvent("ItemFinished", msg, ev.Data)
				}
			case events.FolderSummary:
				if m, ok := ev.Data.(map[string]any); ok {
					if folder, ok := m["folder"].(string); ok {
						s.addEvent("FolderSummary", "Folder summary for "+folder, ev.Data)
					}
				}
			}
		case <-s.Node.Ctx.Done():
			return
		}
	}
}

func (s *Syncweb) GetPendingDevices() map[string]time.Time {
	res := make(map[string]time.Time)
	s.pendingDevices.Range(func(key, value any) bool {
		res[key.(protocol.DeviceID).String()] = value.(time.Time)
		return true
	})
	return res
}

func (s *Syncweb) Start() error {
	return s.Node.Start()
}

func (s *Syncweb) Stop() {
	// Unsubscribe from events first to prevent race conditions
	s.eventSubMu.Lock()
	if s.eventSub != nil {
		s.eventSub.Unsubscribe()
		s.eventSub = nil
	}
	s.eventSubMu.Unlock()

	s.Node.Stop()
}

func (s *Syncweb) IsRunning() bool {
	return s.Node.IsRunning()
}

// ScanFolders triggers a scan on all folders
func (s *Syncweb) ScanFolders() map[string]error {
	return s.Node.App.Internals.ScanFolders()
}

// ScanFolderSubdirs triggers a scan on specific subdirectories of a folder
func (s *Syncweb) ScanFolderSubdirs(folderID string, paths []string) error {
	return s.Node.App.Internals.ScanFolderSubdirs(folderID, paths)
}

// AddDevice adds a device to the Syncthing configuration
func (s *Syncweb) AddDevice(deviceID string, name string, introducer bool) error {
	id, err := protocol.DeviceIDFromString(deviceID)
	if err != nil {
		return err
	}

	waiter, err := s.Node.Cfg.Modify(func(cfg *config.Configuration) {
		for i, dev := range cfg.Devices {
			if dev.DeviceID == id {
				cfg.Devices[i].Name = name
				cfg.Devices[i].Introducer = introducer
				return
			}
		}
		device := cfg.Defaults.Device.Copy()
		device.DeviceID = id
		device.Name = name
		device.Introducer = introducer
		device.Addresses = []string{"dynamic"}
		cfg.SetDevice(device)
	})
	if err != nil {
		return err
	}
	waiter.Wait()
	return s.Node.Cfg.Save()
}

// SetDeviceAddresses sets explicit addresses for a device
func (s *Syncweb) SetDeviceAddresses(deviceID string, addresses []string) error {
	id, err := protocol.DeviceIDFromString(deviceID)
	if err != nil {
		return err
	}

	waiter, err := s.Node.Cfg.Modify(func(cfg *config.Configuration) {
		for i, dev := range cfg.Devices {
			if dev.DeviceID == id {
				cfg.Devices[i].Addresses = addresses
				return
			}
		}
	})
	if err != nil {
		return err
	}
	waiter.Wait()
	return s.Node.Cfg.Save()
}

// AddFolder adds a folder to the Syncthing configuration
func (s *Syncweb) AddFolder(id string, label string, path string, folderType config.FolderType) error {
	absPath, err := filepath.Abs(path)
	if err != nil {
		return err
	}

	if err := os.MkdirAll(absPath, 0o700); err != nil {
		return err
	}

	waiter, err := s.Node.Cfg.Modify(func(cfg *config.Configuration) {
		if _, _, ok := cfg.Folder(id); ok {
			return // Already exists
		}
		fld := cfg.Defaults.Folder.Copy()
		fld.ID = id
		fld.Label = label
		fld.Path = absPath
		fld.Type = folderType
		cfg.SetFolder(fld)
	})
	if err != nil {
		return err
	}
	waiter.Wait()

	// Save config to ensure changes are persisted
	if err := s.Node.Cfg.Save(); err != nil {
		return err
	}
	return nil
}

// AddFolderDevice shares a folder with a device
func (s *Syncweb) AddFolderDevice(folderID string, deviceID string) error {
	devID, err := protocol.DeviceIDFromString(deviceID)
	if err != nil {
		return err
	}

	waiter, err := s.Node.Cfg.Modify(func(cfg *config.Configuration) {
		for i, fld := range cfg.Folders {
			if fld.ID == folderID {
				for _, dev := range fld.Devices {
					if dev.DeviceID == devID {
						return // Already shared
					}
				}
				cfg.Folders[i].Devices = append(cfg.Folders[i].Devices, config.FolderDeviceConfiguration{
					DeviceID: devID,
				})
				return
			}
		}
	})
	if err != nil {
		return err
	}
	waiter.Wait()
	return s.Node.Cfg.Save()
}

// AddFolderDevices shares a folder with multiple devices
func (s *Syncweb) AddFolderDevices(folderID string, deviceIDs []string) error {
	ids := make([]protocol.DeviceID, 0, len(deviceIDs))
	for _, did := range deviceIDs {
		id, err := protocol.DeviceIDFromString(did)
		if err != nil {
			return err
		}
		ids = append(ids, id)
	}

	waiter, err := s.Node.Cfg.Modify(func(cfg *config.Configuration) {
		for i, fld := range cfg.Folders {
			if fld.ID == folderID {
				existing := make(map[protocol.DeviceID]bool)
				for _, dev := range fld.Devices {
					existing[dev.DeviceID] = true
				}

				for _, id := range ids {
					if !existing[id] {
						cfg.Folders[i].Devices = append(cfg.Folders[i].Devices, config.FolderDeviceConfiguration{
							DeviceID: id,
						})
					}
				}
				return
			}
		}
	})
	if err != nil {
		return err
	}
	waiter.Wait()
	return s.Node.Cfg.Save()
}

// RemoveFolderDevices removes devices from a folder
func (s *Syncweb) RemoveFolderDevices(folderID string, deviceIDs []string) error {
	ids := make([]protocol.DeviceID, 0, len(deviceIDs))
	for _, did := range deviceIDs {
		id, err := protocol.DeviceIDFromString(did)
		if err != nil {
			return err
		}
		ids = append(ids, id)
	}

	waiter, err := s.Node.Cfg.Modify(func(cfg *config.Configuration) {
		for i, fld := range cfg.Folders {
			if fld.ID == folderID {
				var newDevices []config.FolderDeviceConfiguration
				for _, dev := range fld.Devices {
					keep := !slices.Contains(ids, dev.DeviceID)
					if keep {
						newDevices = append(newDevices, dev)
					}
				}
				cfg.Folders[i].Devices = newDevices
				return
			}
		}
	})
	if err != nil {
		return err
	}
	waiter.Wait()
	return s.Node.Cfg.Save()
}

func (s *Syncweb) PauseFolder(id string) error {
	waiter, err := s.Node.Cfg.Modify(func(cfg *config.Configuration) {
		for i, f := range cfg.Folders {
			if f.ID == id {
				cfg.Folders[i].Paused = true
				return
			}
		}
	})
	if err != nil {
		return err
	}
	waiter.Wait()
	return s.Node.Cfg.Save()
}

func (s *Syncweb) ResumeFolder(id string) error {
	waiter, err := s.Node.Cfg.Modify(func(cfg *config.Configuration) {
		for i, f := range cfg.Folders {
			if f.ID == id {
				cfg.Folders[i].Paused = false
				return
			}
		}
	})
	if err != nil {
		return err
	}
	waiter.Wait()
	return s.Node.Cfg.Save()
}

func (s *Syncweb) DeleteFolder(id string) error {
	waiter, err := s.Node.Cfg.Modify(func(cfg *config.Configuration) {
		for i, f := range cfg.Folders {
			if f.ID == id {
				cfg.Folders = append(cfg.Folders[:i], cfg.Folders[i+1:]...)
				return
			}
		}
	})
	if err != nil {
		return err
	}
	waiter.Wait()
	return s.Node.Cfg.Save()
}

func (s *Syncweb) PauseDevice(id string) error {
	devID, err := protocol.DeviceIDFromString(id)
	if err != nil {
		return err
	}
	waiter, err := s.Node.Cfg.Modify(func(cfg *config.Configuration) {
		for i, d := range cfg.Devices {
			if d.DeviceID == devID {
				cfg.Devices[i].Paused = true
				return
			}
		}
	})
	if err != nil {
		return err
	}
	waiter.Wait()
	return s.Node.Cfg.Save()
}

func (s *Syncweb) ResumeDevice(id string) error {
	devID, err := protocol.DeviceIDFromString(id)
	if err != nil {
		return err
	}
	waiter, err := s.Node.Cfg.Modify(func(cfg *config.Configuration) {
		for i, d := range cfg.Devices {
			if d.DeviceID == devID {
				cfg.Devices[i].Paused = false
				return
			}
		}
	})
	if err != nil {
		return err
	}
	waiter.Wait()
	return s.Node.Cfg.Save()
}

func (s *Syncweb) DeleteDevice(id string) error {
	devID, err := protocol.DeviceIDFromString(id)
	if err != nil {
		return err
	}
	waiter, err := s.Node.Cfg.Modify(func(cfg *config.Configuration) {
		for i, d := range cfg.Devices {
			if d.DeviceID == devID {
				cfg.Devices = append(cfg.Devices[:i], cfg.Devices[i+1:]...)
				return
			}
		}
	})
	if err != nil {
		return err
	}
	waiter.Wait()
	return s.Node.Cfg.Save()
}

// GetFolders returns a list of folder information
func (s *Syncweb) GetFolders() []FolderInfo {
	cfg := s.Node.Cfg.RawCopy()
	folders := make([]FolderInfo, 0, len(cfg.Folders))
	for _, f := range cfg.Folders {
		devices := make([]string, 0, len(f.Devices))
		for _, d := range f.Devices {
			devices = append(devices, d.DeviceID.String())
		}
		folders = append(folders, FolderInfo{
			ID:      f.ID,
			Label:   f.Label,
			Path:    f.Path,
			Type:    f.Type.String(),
			Paused:  f.Paused,
			Devices: devices,
		})
	}
	return folders
}

// GetDevices returns a list of device information
func (s *Syncweb) GetDevices() []DeviceInfo {
	cfg := s.Node.Cfg.RawCopy()
	devices := make([]DeviceInfo, 0, len(cfg.Devices))
	for _, d := range cfg.Devices {
		devices = append(devices, DeviceInfo{
			ID:         d.DeviceID.String(),
			Name:       d.Name,
			Addresses:  d.Addresses,
			Introducer: d.Introducer,
			Paused:     d.Paused,
		})
	}
	return devices
}

// ResolveLocalPath resolves a sync:// or syncweb:// URL to a local filesystem path,
// ensuring the path is within the folder's root directory
func (s *Syncweb) ResolveLocalPath(syncPath string) (string, string, error) {
	var trimmed string
	if after, ok := strings.CutPrefix(syncPath, "sync://"); ok {
		trimmed = after
	} else if after, ok := strings.CutPrefix(syncPath, "syncweb://"); ok {
		trimmed = after
	} else {
		return "", "", fmt.Errorf("invalid sync path: %s", syncPath)
	}

	parts := strings.SplitN(trimmed, "/", 2)
	if len(parts) < 2 {
		return "", "", fmt.Errorf("invalid sync path: %s", syncPath)
	}

	folderID := parts[0]
	relativePath := filepath.Clean(parts[1])

	if strings.HasPrefix(relativePath, "..") || filepath.IsAbs(relativePath) {
		return "", "", fmt.Errorf("invalid relative path: %s", relativePath)
	}

	cfg := s.Node.Cfg.RawCopy()
	for _, f := range cfg.Folders {
		if f.ID == folderID {
			fullPath := filepath.Join(f.Path, relativePath)
			// Final safety check: ensure the joined path is still within f.Path
			rel, err := filepath.Rel(f.Path, fullPath)
			if err != nil || strings.HasPrefix(rel, "..") {
				return "", "", fmt.Errorf("traversal detected: %s", relativePath)
			}
			return fullPath, folderID, nil
		}
	}

	return "", "", fmt.Errorf("folder not found: %s", folderID)
}

// GetFolderPath returns the local path for a folder ID (internal use only)
func (s *Syncweb) GetFolderPath(folderID string) (string, bool) {
	cfg := s.Node.Cfg.RawCopy()
	for _, f := range cfg.Folders {
		if f.ID == folderID {
			return f.Path, true
		}
	}
	return "", false
}

// Unignore removes a file from the ignore list by adding an unignore (!) pattern
func (s *Syncweb) Unignore(folderID, relativePath string) error {
	lines, _, err := s.Node.App.Internals.Ignores(folderID)
	if err != nil {
		return err
	}

	pattern := "!" + relativePath
	if slices.Contains(lines, pattern) {
		return nil // Already unignored
	}

	lines = append(lines, pattern)
	return s.Node.App.Internals.SetIgnores(folderID, lines)
}

// GetGlobalFileInfo returns information about a file across the cluster
func (s *Syncweb) GetGlobalFileInfo(folderID, path string) (protocol.FileInfo, bool, error) {
	return s.Node.App.Internals.GlobalFileInfo(folderID, path)
}

// SyncwebReadSeeker implements io.ReadSeeker by fetching blocks from Syncthing peers
type SyncwebReadSeeker struct {
	s        *Syncweb
	folderID string
	info     protocol.FileInfo
	offset   int64
	//nolint:containedctx // Context is used for read operations lifecycle
	ctx context.Context
}

func (s *Syncweb) NewReadSeeker(ctx context.Context, folderID, path string) (*SyncwebReadSeeker, error) {
	info, ok, err := s.GetGlobalFileInfo(folderID, path)
	if err != nil {
		return nil, err
	}
	if !ok {
		return nil, fmt.Errorf("file not found in cluster: %s", path)
	}

	return &SyncwebReadSeeker{
		s:        s,
		folderID: folderID,
		info:     info,
		offset:   0,
		ctx:      ctx,
	}, nil
}

func (r *SyncwebReadSeeker) Seek(offset int64, whence int) (int64, error) {
	var newOffset int64
	switch whence {
	case io.SeekStart:
		newOffset = offset
	case io.SeekCurrent:
		newOffset = r.offset + offset
	case io.SeekEnd:
		newOffset = r.info.Size + offset
	default:
		return 0, fmt.Errorf("invalid whence: %d", whence)
	}

	if newOffset < 0 {
		return 0, fmt.Errorf("negative offset: %d", newOffset)
	}
	r.offset = newOffset
	return r.offset, nil
}

func (r *SyncwebReadSeeker) Read(p []byte) (n int, err error) {
	if r.offset >= r.info.Size {
		return 0, io.EOF
	}

	wantedSize := int64(len(p))
	// Check for overflow before addition
	if r.offset > r.info.Size-wantedSize {
		wantedSize = r.info.Size - r.offset
	}

	if wantedSize <= 0 {
		return 0, io.EOF
	}

	// Calculate which blocks we need
	blockSize := int64(r.info.BlockSize())
	startBlock := r.offset / blockSize
	// Check for overflow in endBlock calculation
	endOffset := r.offset + wantedSize - 1
	if endOffset < r.offset { // Overflow check
		endOffset = r.info.Size - 1
	}
	endBlock := endOffset / blockSize

	var totalRead int64
	for i := startBlock; i <= endBlock; i++ {
		block := r.info.Blocks[i]

		// Determine which peers have this block
		availables, err := r.s.Node.App.Internals.BlockAvailability(r.folderID, r.info, block)
		if err != nil {
			return int(totalRead), err
		}
		if len(availables) == 0 {
			return int(totalRead), fmt.Errorf("no peers available for block %d", i)
		}

		// Sort available peers by their performance score (lower is better)
		slices.SortFunc(availables, func(a, b stmodel.Availability) int {
			scoreA := r.s.Measurements.Score(a.ID)
			scoreB := r.s.Measurements.Score(b.ID)
			if scoreA < scoreB {
				return -1
			}
			if scoreA > scoreB {
				return 1
			}
			return 0
		})

		var data []byte
		var downloadErr error
		for _, peer := range availables {
			startTime := time.Now()
			data, downloadErr = r.s.Node.App.Internals.DownloadBlock(r.ctx, peer.ID, r.folderID, r.info.Name, int(i), block, peer.FromTemporary)
			r.s.Measurements.Record(peer.ID, time.Since(startTime), downloadErr)
			if downloadErr == nil {
				break
			}
			slog.Warn("Failed to download block from peer, trying next", "peer", peer.ID, "error", downloadErr)
		}

		if downloadErr != nil {
			return int(totalRead), fmt.Errorf("all peers failed to provide block %d: %w", i, downloadErr)
		}

		// Calculate how much of this block we actually need
		blockOffset := r.offset + totalRead - block.Offset
		dataStart := max(blockOffset, 0)

		dataEnd := int64(len(data))
		remainingNeeded := wantedSize - totalRead
		if dataEnd-dataStart > remainingNeeded {
			dataEnd = dataStart + remainingNeeded
		}

		copied := copy(p[totalRead:], data[dataStart:dataEnd])
		totalRead += int64(copied)
	}

	r.offset += totalRead
	// Safe conversion to int for 32-bit systems
	if totalRead > math.MaxInt {
		return math.MaxInt, nil
	}
	return int(totalRead), nil
}

// GetIgnores returns the ignore patterns for a folder
func (s *Syncweb) GetIgnores(folderID string) ([]string, error) {
	lines, _, err := s.Node.App.Internals.Ignores(folderID)
	return lines, err
}

// SetIgnores sets the ignore patterns for a folder
func (s *Syncweb) SetIgnores(folderID string, lines []string) error {
	return s.Node.App.Internals.SetIgnores(folderID, lines)
}

// AddIgnores adds unignore patterns to a folder's ignore list
// This is used to mark files for download in receiveonly folders
func (s *Syncweb) AddIgnores(folderID string, unignores []string) error {
	existing, _, err := s.Node.App.Internals.Ignores(folderID)
	if err != nil {
		return err
	}

	// Keep existing patterns that are not Syncweb-managed
	var preserved []string
	for _, p := range existing {
		if !strings.HasPrefix(p, "// Syncweb-managed") {
			preserved = append(preserved, p)
		}
	}

	// Build new unignore patterns
	var newPatterns []string
	for _, p := range unignores {
		if strings.HasPrefix(p, "//") {
			continue
		}
		if !strings.HasPrefix(p, "!/") {
			p = "!/" + p
		}
		newPatterns = append(newPatterns, p)
	}

	// Combine: Syncweb header + unignores (sorted) + ignores (sorted) + wildcard
	combined := make(map[string]bool)
	for _, p := range preserved {
		combined[p] = true
	}
	for _, p := range newPatterns {
		combined[p] = true
	}

	var unignoreList, ignoreList []string
	for p := range combined {
		if strings.HasPrefix(p, "!") {
			unignoreList = append(unignoreList, p)
		} else if p != "*" {
			ignoreList = append(ignoreList, p)
		}
	}

	slices.Sort(unignoreList)
	slices.Sort(ignoreList)

	final := append([]string{"// Syncweb-managed"}, unignoreList...)
	final = append(final, ignoreList...)
	final = append(final, "*")

	return s.Node.App.Internals.SetIgnores(folderID, final)
}

// GetFolderStats returns statistics for all folders
func (s *Syncweb) GetFolderStats() map[string]map[string]any {
	stats := make(map[string]map[string]any)
	cfg := s.Node.Cfg.RawCopy()

	for _, f := range cfg.Folders {
		stats[f.ID] = map[string]any{
			"id":     f.ID,
			"label":  f.Label,
			"path":   f.Path,
			"type":   f.Type.String(),
			"paused": f.Paused,
		}
	}

	return stats
}

// GetDeviceStats returns statistics for all devices
func (s *Syncweb) GetDeviceStats() map[string]map[string]any {
	stats := make(map[string]map[string]any)
	cfg := s.Node.Cfg.RawCopy()

	for _, d := range cfg.Devices {
		stats[d.DeviceID.String()] = map[string]any{
			"id":         d.DeviceID.String(),
			"name":       d.Name,
			"paused":     d.Paused,
			"introducer": d.Introducer,
			"addresses":  d.Addresses,
		}
	}

	return stats
}

// GetPendingFolders returns pending folder invitations from other devices
func (s *Syncweb) GetPendingFolders() map[string]map[string]any {
	pending := make(map[string]map[string]any)
	cfg := s.Node.Cfg.RawCopy()

	for _, d := range cfg.Devices {
		devPending, err := s.Node.App.Internals.PendingFolders(d.DeviceID)
		if err != nil {
			continue
		}

		for folderID := range devPending {
			if _, exists := pending[folderID]; !exists {
				pending[folderID] = map[string]any{
					"offeredBy": make(map[string]map[string]any),
				}
			}
			pending[folderID]["offeredBy"].(map[string]map[string]any)[d.DeviceID.String()] = map[string]any{}
		}
	}

	return pending
}

// GetDiscoveredDevices returns devices from the discovery cache
func (s *Syncweb) GetDiscoveredDevices() map[string]map[string]any {
	discovered := make(map[string]map[string]any)
	cfg := s.Node.Cfg.RawCopy()

	for _, d := range cfg.Devices {
		// For now, just return device info
		// Discovery cache access would require additional API calls
		discovered[d.DeviceID.String()] = map[string]any{
			"id":        d.DeviceID.String(),
			"name":      d.Name,
			"addresses": d.Addresses,
		}
	}

	return discovered
}

// GetPendingDevices returns devices waiting to be accepted
func (s *Syncweb) GetPendingDevicesMap() map[string]map[string]any {
	pending := make(map[string]map[string]any)
	cfg := s.Node.Cfg.RawCopy()

	// Check for rejected/pending devices
	for _, d := range cfg.Devices {
		if d.Paused && d.Name == "" {
			pending[d.DeviceID.String()] = map[string]any{
				"id":   d.DeviceID.String(),
				"time": time.Now().Format(time.RFC3339),
			}
		}
	}

	return pending
}

// CountSeeders returns the number of unique devices that have blocks for a file
func (s *Syncweb) CountSeeders(folderID, path string) (int, error) {
	info, ok, err := s.GetGlobalFileInfo(folderID, path)
	if err != nil || !ok {
		return 0, fmt.Errorf("file not found: %s", path)
	}

	seederSet := make(map[protocol.DeviceID]bool)
	for _, block := range info.Blocks {
		availables, err := s.Node.App.Internals.BlockAvailability(folderID, info, block)
		if err != nil {
			continue
		}
		for _, av := range availables {
			seederSet[av.ID] = true
		}
	}

	return len(seederSet), nil
}

// GetCompletion returns folder completion percentage for a device
func (s *Syncweb) GetCompletion(deviceID protocol.DeviceID, folderID string) (map[string]any, error) {
	comp, err := s.Node.App.Internals.Completion(deviceID, folderID)
	if err != nil {
		return nil, err
	}

	return map[string]any{
		"completion_pct": comp.CompletionPct,
		"global_bytes":   comp.GlobalBytes,
		"need_bytes":     comp.NeedBytes,
		"global_items":   comp.GlobalItems,
		"need_items":     comp.NeedItems,
		"need_deletes":   comp.NeedDeletes,
		"sequence":       comp.Sequence,
	}, nil
}

// GetGlobalTree returns folder tree structure for browsing
// levels: -1 for all levels, 0 for root only, etc
// returnOnlyDirectories: if true, only return directory entries
func (s *Syncweb) GetGlobalTree(folderID, prefix string, levels int, returnOnlyDirectories bool) ([]map[string]any, error) {
	tree, err := s.Node.App.Internals.GlobalTree(folderID, prefix, levels, returnOnlyDirectories)
	if err != nil {
		return nil, err
	}

	result := make([]map[string]any, len(tree))
	for i, entry := range tree {
		result[i] = map[string]any{
			"name":    entry.Name,
			"modTime": entry.ModTime,
			"size":    entry.Size,
			"type":    entry.Type,
		}
	}
	return result, nil
}

// GetLocalChangedFiles returns locally changed files for a folder (paginated)
func (s *Syncweb) GetLocalChangedFiles(folderID string, page, perPage int) ([]map[string]any, error) {
	files, err := s.Node.App.Internals.LocalChangedFolderFiles(folderID, page, perPage)
	if err != nil {
		return nil, err
	}

	result := make([]map[string]any, len(files))
	for i, f := range files {
		result[i] = map[string]any{
			"name":       f.Name,
			"size":       f.Size,
			"modified":   f.ModTime,
			"type":       f.Type.String(),
			"version":    f.Version.String(),
			"permission": f.Permissions,
		}
	}
	return result, nil
}

// GetNeedFiles returns paginated list of needed files for a folder
// Returns three lists: remote (needed from remote), local (local changes), queued (queued for sync)
func (s *Syncweb) GetNeedFiles(folderID string, page, perPage int) (remote, local, queued []map[string]any, err error) {
	remoteFiles, localFiles, queuedFiles, err := s.Node.App.Internals.NeedFolderFiles(folderID, page, perPage)
	if err != nil {
		return nil, nil, nil, err
	}

	convertFiles := func(files []protocol.FileInfo) []map[string]any {
		result := make([]map[string]any, len(files))
		for i, f := range files {
			result[i] = map[string]any{
				"name":       f.Name,
				"size":       f.Size,
				"modified":   f.ModTime,
				"type":       f.Type.String(),
				"version":    f.Version.String(),
				"permission": f.Permissions,
			}
		}
		return result
	}

	return convertFiles(remoteFiles), convertFiles(localFiles), convertFiles(queuedFiles), nil
}

// GetRemoteNeedFiles returns files needed by a specific remote device
func (s *Syncweb) GetRemoteNeedFiles(folderID string, deviceID protocol.DeviceID, page, perPage int) ([]map[string]any, error) {
	files, err := s.Node.App.Internals.RemoteNeedFolderFiles(folderID, deviceID, page, perPage)
	if err != nil {
		return nil, err
	}

	result := make([]map[string]any, len(files))
	for i, f := range files {
		result[i] = map[string]any{
			"name":       f.Name,
			"size":       f.Size,
			"modified":   f.ModTime,
			"type":       f.Type.String(),
			"version":    f.Version.String(),
			"permission": f.Permissions,
		}
	}
	return result, nil
}
