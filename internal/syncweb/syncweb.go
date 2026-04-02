package syncweb

import (
	"cmp"
	"context"
	"errors"
	"fmt"
	"io"
	"iter"
	"log/slog"
	"math"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/events"
	stmodel "github.com/syncthing/syncthing/lib/model"
	"github.com/syncthing/syncthing/lib/protocol"

	"github.com/chapmanjacobd/syncweb/internal/models"
)

// Constants for syncweb configuration
const (
	// EventBufferLimit is the maximum number of events to keep in memory
	EventBufferLimit = 100
)

type DeviceInfo struct {
	ID         string   `json:"id"`
	Name       string   `json:"name"`
	Addresses  []string `json:"addresses"`
	Introducer bool     `json:"introducer"`
	Paused     bool     `json:"paused"`
}

type FolderInfo struct {
	ID         string   `json:"id"`
	Label      string   `json:"label"`
	Path       string   `json:"path"`
	Type       string   `json:"type"`
	Paused     bool     `json:"paused"`
	Devices    []string `json:"devices"`
	GlobalSize Counts   `json:"globalSize"`
	LocalSize  Counts   `json:"localSize"`
	NeedSize   Counts   `json:"needSize"`
	State      string   `json:"state"`
	Completed  int64    `json:"completed"`
}

type Measurements struct {
	mutex sync.RWMutex
	data  map[protocol.DeviceID]*Measurement
}

type Measurement struct {
	TotalTime  time.Duration
	TotalBytes int
	Count      int
	Errors     int
}

func NewMeasurements() *Measurements {
	return &Measurements{
		data: make(map[protocol.DeviceID]*Measurement),
	}
}

func (m *Measurements) Record(deviceID protocol.DeviceID, duration time.Duration, bytes int, err error) {
	m.mutex.Lock()
	defer m.mutex.Unlock()

	meas, ok := m.data[deviceID]
	if !ok {
		meas = &Measurement{}
		m.data[deviceID] = meas
	}

	meas.TotalTime += duration
	meas.TotalBytes += bytes
	meas.Count++
	if err != nil {
		meas.Errors++
	}
}

func (m *Measurements) Score(deviceID protocol.DeviceID) float64 {
	m.mutex.RLock()
	defer m.mutex.RUnlock()

	meas, ok := m.data[deviceID]
	if !ok || (meas.Count == 0 && meas.Errors == 0) {
		return 1.0 // Neutral score
	}

	if meas.Count == 0 && meas.Errors > 0 {
		return 1e9 // Extremely high penalty for peers with only errors
	}

	// Calculate average time per byte (inverse of bandwidth)
	var timePerByte float64
	if meas.TotalBytes > 0 {
		timePerByte = float64(meas.TotalTime) / float64(meas.TotalBytes)
	} else {
		// Use a conservative estimate if we have counts but no bytes (should be rare)
		timePerByte = float64(meas.TotalTime) / float64(meas.Count) / 128e3
	}

	errorRate := float64(meas.Errors) / float64(meas.Count+meas.Errors)

	// Lower is better. Strong penalty for errors.
	return timePerByte * (1.0 + errorRate*100.0)
}

type Syncweb struct {
	Node           *Node
	Measurements   *Measurements
	pendingDevices sync.Map // map[protocol.DeviceID]time.Time
	events         []models.SyncEvent
	eventsMu       sync.Mutex
	eventsCache    atomic.Value // Stores []models.SyncEvent
	eventSub       events.Subscription
	eventSubMu     sync.Mutex
}

func NewSyncweb(homeDir, name, listenAddr string) (*Syncweb, error) {
	node, err := NewNode(homeDir, name, listenAddr)
	if err != nil {
		return nil, err
	}

	s := &Syncweb{
		Node:         node,
		Measurements: NewMeasurements(),
		events:       make([]models.SyncEvent, 0),
	}
	s.eventsCache.Store(s.events)

	go s.watchEvents()

	return s, nil
}

func (s *Syncweb) addEvent(evType, message string, data any) {
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

	// Update the snapshot for readers
	s.eventsCache.Store(slices.Clone(s.events))
}

func (s *Syncweb) GetEvents() []models.SyncEvent {
	v := s.eventsCache.Load()
	if v == nil {
		return nil
	}
	return append([]models.SyncEvent(nil), v.([]models.SyncEvent)...) //nolint:errcheck // append doesn't return error
}

// MyID returns the local device ID
func (s *Syncweb) MyID() protocol.DeviceID {
	return s.Node.MyID()
}

// RawConfig returns a copy of the current Syncthing configuration
func (s *Syncweb) RawConfig() config.Configuration {
	return s.Node.Cfg.RawCopy()
}

// SaveConfig saves the current Syncthing configuration to disk
func (s *Syncweb) SaveConfig() error {
	return s.Node.Cfg.Save()
}

// IsConnectedTo returns true if the node is connected to the specified device
func (s *Syncweb) IsConnectedTo(deviceID protocol.DeviceID) bool {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return false
	}
	return s.Node.App.Internals.IsConnectedTo(deviceID)
}

// AllGlobalFiles returns an iterator that streams all global files in a folder
func (s *Syncweb) AllGlobalFiles(folderID string) (iter.Seq[FileMetadata], func() error) {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return func(func(FileMetadata) bool) {}, func() error { return nil }
	}
	seq, cancel := s.Node.App.Internals.AllGlobalFiles(folderID)
	return func(yield func(FileMetadata) bool) {
		for meta := range seq {
			if !yield(FileMetadata{
				Name:       meta.Name,
				Sequence:   meta.Sequence,
				ModNanos:   meta.ModNanos,
				Size:       meta.Size,
				LocalFlags: meta.LocalFlags,
				Type:       meta.Type,
				Deleted:    meta.Deleted,
			}) {

				return
			}
		}
	}, func() error { return cancel() }
}

// GlobalSize returns the total size of all global files in a folder
func (s *Syncweb) GlobalSize(folderID string) (Counts, error) {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return Counts{}, errors.New("internals not initialized")
	}
	c, err := s.Node.App.Internals.GlobalSize(folderID)
	return Counts{
		Files:       c.Files,
		Directories: c.Directories,
		Symlinks:    c.Symlinks,
		Deleted:     c.Deleted,
		Bytes:       c.Bytes,
		Sequence:    c.Sequence,
		DeviceID:    c.DeviceID,
		LocalFlags:  c.LocalFlags,
	}, err
}

// LocalSize returns the total size of all local files in a folder
func (s *Syncweb) LocalSize(folderID string) (Counts, error) {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return Counts{}, errors.New("internals not initialized")
	}
	c, err := s.Node.App.Internals.LocalSize(folderID)
	return Counts{
		Files:       c.Files,
		Directories: c.Directories,
		Symlinks:    c.Symlinks,
		Deleted:     c.Deleted,
		Bytes:       c.Bytes,
		Sequence:    c.Sequence,
		DeviceID:    c.DeviceID,
		LocalFlags:  c.LocalFlags,
	}, err
}

// NeedSize returns the total size of files needed from other devices
func (s *Syncweb) NeedSize(folderID string, deviceID protocol.DeviceID) (Counts, error) {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return Counts{}, errors.New("internals not initialized")
	}
	c, err := s.Node.App.Internals.NeedSize(folderID, deviceID)
	return Counts{
		Files:       c.Files,
		Directories: c.Directories,
		Symlinks:    c.Symlinks,
		Deleted:     c.Deleted,
		Bytes:       c.Bytes,
		Sequence:    c.Sequence,
		DeviceID:    c.DeviceID,
		LocalFlags:  c.LocalFlags,
	}, err
}

// FolderState returns the current state of a folder
func (s *Syncweb) FolderState(folderID string) (string, time.Time, error) {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return "unknown", time.Time{}, errors.New("internals not initialized")
	}
	return s.Node.App.Internals.FolderState(folderID)
}

// FolderProgressBytesCompleted returns the number of bytes completed for a folder sync
func (s *Syncweb) FolderProgressBytesCompleted(folderID string) int64 {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return 0
	}
	return s.Node.App.Internals.FolderProgressBytesCompleted(folderID)
}

// GetCompletion returns the completion status for a folder on a specific device
func (s *Syncweb) GetCompletion(deviceID protocol.DeviceID, folderID string) (stmodel.FolderCompletion, error) {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return stmodel.FolderCompletion{}, errors.New("internals not initialized")
	}
	return s.Node.App.Internals.Completion(deviceID, folderID)
}

// BlockAvailability returns a list of devices that have the specified block
func (s *Syncweb) BlockAvailability(
	folderID string,
	info *protocol.FileInfo,
	block protocol.BlockInfo,
) ([]stmodel.Availability, error) {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return nil, errors.New("internals not initialized")
	}
	return s.Node.App.Internals.BlockAvailability(folderID, *info, block)
}

func (s *Syncweb) watchEvents() {
	logger := slog.Default().With("component", "syncweb")
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
			s.handleEvent(ev, logger)
		case <-s.Node.Ctx.Done():
			return
		}
	}
}

func (s *Syncweb) handleEvent(ev events.Event, logger *slog.Logger) {
	switch ev.Type { //nolint:exhaustive // only handle specific events
	case events.DeviceRejected:
		s.handleDeviceRejected(ev, logger)
	case events.DeviceConnected:
		s.handleDeviceConnected(ev, logger)
	case events.ItemStarted:
		s.handleItemStarted(ev)
	case events.ItemFinished:
		s.handleItemFinished(ev)
	case events.FolderSummary:
		s.handleFolderSummary(ev)
	}
}

func (s *Syncweb) handleDeviceRejected(ev events.Event, logger *slog.Logger) {
	var deviceIDStr string
	if m, ok := ev.Data.(map[string]any); ok {
		if idStr, ok2 := m["device"].(string); ok2 {
			deviceIDStr = idStr
		}
	} else if m, ok2 := ev.Data.(map[string]string); ok2 {
		deviceIDStr = m["device"]
	}

	if deviceIDStr != "" {
		if id, err := protocol.DeviceIDFromString(deviceIDStr); err == nil {
			s.pendingDevices.Store(id, ev.Time)
			logger.Info("Device rejected (pending)", "id", id)
			s.addEvent("DeviceRejected", "New device request: "+deviceIDStr, ev.Data)
		}
	}
}

func (s *Syncweb) handleDeviceConnected(ev events.Event, logger *slog.Logger) {
	var deviceIDStr string
	if m, ok := ev.Data.(map[string]any); ok {
		if idStr, ok2 := m["id"].(string); ok2 {
			deviceIDStr = idStr
		}
	} else if m, ok2 := ev.Data.(map[string]string); ok2 {
		deviceIDStr = m["id"]
	}

	if deviceIDStr != "" {
		if id, err := protocol.DeviceIDFromString(deviceIDStr); err == nil {
			s.pendingDevices.Delete(id)
			logger.Debug("Device connected", "id", id)
			s.addEvent("DeviceConnected", "Device connected: "+deviceIDStr, ev.Data)
		}
	}
}

func (s *Syncweb) handleItemStarted(ev events.Event) {
	if m, ok := ev.Data.(map[string]any); ok {
		if item, ok2 := m["item"].(string); ok2 {
			s.addEvent("ItemStarted", "Syncing: "+item, ev.Data)
		}
	}
}

func (s *Syncweb) handleItemFinished(ev events.Event) {
	if m, ok := ev.Data.(map[string]any); ok {
		item, _ := m["item"].(string)
		errMsg, _ := m["error"].(string)
		msg := "Finished: " + item
		if errMsg != "" {
			msg += " (Error: " + errMsg + ")"
		}
		s.addEvent("ItemFinished", msg, ev.Data)
	}
}

func (s *Syncweb) handleFolderSummary(ev events.Event) {
	if m, ok := ev.Data.(map[string]any); ok {
		if folder, ok2 := m["folder"].(string); ok2 {
			s.addEvent("FolderSummary", "Folder summary for "+folder, ev.Data)
		}
	}
}

func (s *Syncweb) GetPendingDevices() map[string]time.Time {
	res := make(map[string]time.Time)
	s.pendingDevices.Range(func(key, value any) bool {
		deviceID, ok := key.(protocol.DeviceID)
		if !ok {
			return true
		}
		t, ok := value.(time.Time)
		if !ok {
			return true
		}
		res[deviceID.String()] = t
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
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return map[string]error{"all": errors.New("internals not initialized")}
	}
	return s.Node.App.Internals.ScanFolders()
}

// ScanFolderSubdirs triggers a scan on specific subdirectories of a folder
func (s *Syncweb) ScanFolderSubdirs(folderID string, paths []string) error {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return errors.New("internals not initialized")
	}
	return s.Node.App.Internals.ScanFolderSubdirs(folderID, paths)
}

// AddDevice adds a device to the Syncthing configuration
func (s *Syncweb) AddDevice(deviceID, name string, introducer bool) error {
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

func (s *Syncweb) GetDiscoveredDevices() map[string]time.Time {
	// Syncthing doesn't easily expose this via Internals yet
	return nil
}

// AddFolder adds a folder to the Syncthing configuration
func (s *Syncweb) AddFolder(id, label, path string, folderType config.FolderType) error {
	absPath, err := filepath.Abs(path)
	if err != nil {
		return err
	}

	if err = os.MkdirAll(absPath, 0o700); err != nil {
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
	return s.Node.Cfg.Save()
}

// AddFolderDevice shares a folder with a device
func (s *Syncweb) AddFolderDevice(folderID, deviceID string) error {
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

		globalSize, _ := s.GlobalSize(f.ID)
		localSize, _ := s.LocalSize(f.ID)
		needSize, _ := s.NeedSize(f.ID, protocol.LocalDeviceID)
		state, _, _ := s.FolderState(f.ID)
		completed := s.FolderProgressBytesCompleted(f.ID)

		folders = append(folders, FolderInfo{
			ID:         f.ID,
			Label:      f.Label,
			Path:       f.Path,
			Type:       f.Type.String(),
			Paused:     f.Paused,
			Devices:    devices,
			GlobalSize: globalSize,
			LocalSize:  localSize,
			NeedSize:   needSize,
			State:      state,
			Completed:  completed,
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
func (s *Syncweb) ResolveLocalPath(syncPath string) (folderID, localPath string, err error) {
	var trimmed string
	if after, ok := strings.CutPrefix(syncPath, "sync://"); ok {
		trimmed = after
	} else if after2, ok2 := strings.CutPrefix(syncPath, "syncweb://"); ok2 {
		trimmed = after2
	} else {
		return "", "", fmt.Errorf("invalid sync path: %s", syncPath)
	}

	parts := strings.SplitN(trimmed, "/", 2)
	if len(parts) < 2 {
		return "", "", fmt.Errorf("invalid sync path: %s", syncPath)
	}

	folderID = parts[0]
	relativePath := filepath.Clean(parts[1])

	if strings.HasPrefix(relativePath, "..") || filepath.IsAbs(relativePath) {
		return "", "", fmt.Errorf("invalid relative path: %s", relativePath)
	}

	cfg := s.Node.Cfg.RawCopy()
	for _, f := range cfg.Folders {
		if f.ID == folderID {
			localPath = filepath.Join(f.Path, relativePath)
			// Final safety check: ensure the joined path is still within f.Path
			rel, relErr := filepath.Rel(f.Path, localPath)
			if relErr != nil || strings.HasPrefix(rel, "..") {
				return "", "", fmt.Errorf("traversal detected: %s", relativePath)
			}
			return localPath, folderID, nil
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
	return s.AddIgnores(folderID, []string{relativePath})
}

// GetGlobalFileInfo returns information about a file across the cluster
func (s *Syncweb) GetGlobalFileInfo(folderID, path string) (protocol.FileInfo, bool, error) {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return protocol.FileInfo{}, false, errors.New("internals not initialized")
	}
	return s.Node.App.Internals.GlobalFileInfo(folderID, path)
}

// SyncwebReadSeeker implements [io.ReadSeeker] by fetching blocks from Syncthing peers
type SyncwebReadSeeker struct {
	s        *Syncweb
	folderID string
	info     protocol.FileInfo
	offset   int64
	//nolint:containedctx // Context is used for read operations lifecycle
	ctx context.Context
}

func (s *Syncweb) NewReadSeeker(ctx context.Context, folderID, path string) (io.ReadSeeker, error) {
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

// Read reads bytes from the remote file
//
//nolint:maintidx // Read is complex but needed for efficient streaming
func (r *SyncwebReadSeeker) Read(p []byte) (n int, err error) {
	if r.offset >= r.info.Size {
		return 0, io.EOF
	}

	wantedSize := int64(len(p))
	// Cap wantedSize to remaining file size
	if r.offset+wantedSize > r.info.Size {
		wantedSize = r.info.Size - r.offset
	}

	if wantedSize <= 0 {
		return 0, io.EOF
	}

	blockSize := int64(r.info.BlockSize())
	startBlock := r.offset / blockSize
	endBlock := (r.offset + wantedSize - 1) / blockSize

	// Sanity check for block indices
	if startBlock >= int64(len(r.info.Blocks)) {
		return 0, io.EOF
	}
	if endBlock >= int64(len(r.info.Blocks)) {
		endBlock = int64(len(r.info.Blocks)) - 1
	}

	logger := slog.Default().With("folderID", r.folderID, "file", r.info.Name)
	var totalRead int64

	for i := startBlock; i <= endBlock; i++ {
		// Check context before each block to stop early if needed
		if err := r.ctx.Err(); err != nil {
			return int(totalRead), err
		}

		block := r.info.Blocks[i]

		// Determine which peers have this block
		if r.s.Node == nil || r.s.Node.App == nil || r.s.Node.App.Internals == nil {
			return int(totalRead), errors.New("internals not initialized")
		}
		availables, availErr := r.s.Node.App.Internals.BlockAvailability(r.folderID, r.info, block)
		if availErr != nil {
			return int(totalRead), availErr
		}
		if len(availables) == 0 {
			return int(totalRead), fmt.Errorf("no peers available for block %d", i)
		}

		// Sort available peers by their performance score (lower is better)
		slices.SortFunc(availables, func(a, b stmodel.Availability) int {
			scoreA := r.s.Measurements.Score(a.ID)
			scoreB := r.s.Measurements.Score(b.ID)
			return cmp.Compare(scoreA, scoreB)
		})

		var data []byte
		var downloadErr error
		for _, peer := range availables {
			startTime := time.Now()
			data, downloadErr = r.s.Node.App.Internals.DownloadBlock(
				r.ctx,
				peer.ID,
				r.folderID,
				r.info.Name,
				int(i),
				block,
				peer.FromTemporary,
			)
			r.s.Measurements.Record(peer.ID, time.Since(startTime), len(data), downloadErr)
			if downloadErr == nil {
				break
			}
			logger.Warn("Failed to download block from peer, trying next", "peer", peer.ID, "error", downloadErr)
		}

		if downloadErr != nil {
			return int(totalRead), fmt.Errorf("all peers failed to provide block %d: %w", i, downloadErr)
		}

		// Calculate how much of this block we actually need
		// offsetWithinBlock is where we start reading from THIS block
		offsetWithinBlock := (r.offset + totalRead) - block.Offset
		dataStart := max(offsetWithinBlock, 0)

		dataEnd := int64(len(data))
		remainingInRequest := wantedSize - totalRead
		if dataEnd-dataStart > remainingInRequest {
			dataEnd = dataStart + remainingInRequest
		}

		if dataStart >= dataEnd {
			continue // Should not happen if logic is correct
		}

		copied := copy(p[totalRead:], data[dataStart:dataEnd])
		totalRead += int64(copied)
	}

	r.offset += totalRead
	// Safe conversion to int for return
	if totalRead > math.MaxInt {
		return math.MaxInt, nil
	}
	return int(totalRead), nil
}

// GetIgnores returns the ignore patterns for a folder
func (s *Syncweb) GetIgnores(folderID string) ([]string, error) {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return nil, errors.New("internals not initialized")
	}
	lines, _, err := s.Node.App.Internals.Ignores(folderID)
	return lines, err
}

// SetIgnores sets the ignore patterns for a folder
func (s *Syncweb) SetIgnores(folderID string, lines []string) error {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return errors.New("internals not initialized")
	}
	return s.Node.App.Internals.SetIgnores(folderID, lines)
}

// AddIgnores adds unignore patterns to a folder's ignore list
// This is used to mark files for download in receiveonly folders
func (s *Syncweb) AddIgnores(folderID string, unignores []string) error {
	existing, err := s.GetIgnores(folderID)
	if err != nil {
		return err
	}

	var userPatterns []string
	var managedPatterns []string
	inBlock := false

	for _, p := range existing {
		if p == "// BEGIN Syncweb-managed" {
			inBlock = true
			continue
		}
		if p == "// END Syncweb-managed" {
			inBlock = false
			continue
		}

		if inBlock {
			if p != "*" {
				managedPatterns = append(managedPatterns, p)
			}
		} else {
			userPatterns = append(userPatterns, p)
		}
	}

	// Add new unignore patterns
	for _, p := range unignores {
		if strings.HasPrefix(p, "//") {
			continue
		}
		if !strings.HasPrefix(p, "!/") {
			p = "!/" + p
		}
		managedPatterns = append(managedPatterns, p)
	}

	// Deduplicate and sort managed patterns
	combined := make(map[string]bool)
	for _, p := range managedPatterns {
		combined[p] = true
	}

	var finalManaged []string
	for p := range combined {
		finalManaged = append(finalManaged, p)
	}
	slices.Sort(finalManaged)

	final := make([]string, 0, len(userPatterns)+len(finalManaged)+3)
	final = append(final, userPatterns...)
	final = append(final, "// BEGIN Syncweb-managed")
	final = append(final, finalManaged...)
	final = append(final, "*")
	final = append(final, "// END Syncweb-managed")

	return s.SetIgnores(folderID, final)
}

// WaitUntilIdle waits until the specified folder is idle (fully synced)
func (s *Syncweb) WaitUntilIdle(folderID string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
			return errors.New("internals not initialized")
		}
		comp, err := s.Node.App.Internals.Completion(protocol.LocalDeviceID, folderID)
		if err == nil && comp.CompletionPct >= 100 {
			return nil
		}
		time.Sleep(100 * time.Millisecond)
	}
	return fmt.Errorf("timeout waiting for folder %s to become idle", folderID)
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
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return pending
	}
	cfg := s.Node.Cfg.RawCopy()

	for _, d := range cfg.Devices {
		devPending, err := s.Node.App.Internals.PendingFolders(d.DeviceID)
		if err != nil {
			continue
		}

		for id, f := range devPending {
			// Find the label from OfferedBy map
			label := id
			for _, observed := range f.OfferedBy {
				if observed.Label != "" {
					label = observed.Label
					break
				}
			}

			pending[id] = map[string]any{
				"id":        id,
				"label":     label,
				"offeredBy": d.DeviceID.String(),
			}
		}
	}

	return pending
}

// GetGlobalTree returns a list of entries at a specific prefix
func (s *Syncweb) GetGlobalTree(
	folderID, prefix string,
	levels int,
	returnOnlyDirectories bool,
) ([]models.LsEntry, error) {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return nil, errors.New("internals not initialized")
	}
	tree, err := s.Node.App.Internals.GlobalTree(folderID, prefix, levels, returnOnlyDirectories)
	if err != nil {
		return nil, err
	}

	return s.flattenTree(tree, prefix), nil
}

func (s *Syncweb) flattenTree(tree []*stmodel.TreeEntry, prefix string) []models.LsEntry {
	var res []models.LsEntry
	for _, entry := range tree {
		entryPath := filepath.Join(prefix, entry.Name)
		res = append(res, models.LsEntry{
			Name:     entry.Name,
			Path:     entryPath,
			IsDir:    entry.Type == "directory",
			Size:     entry.Size,
			Modified: entry.ModTime.Format(time.RFC3339),
		})
		if len(entry.Children) > 0 {
			res = append(res, s.flattenTree(entry.Children, entryPath)...)
		}
	}
	return res
}

// GetLocalChangedFiles returns locally changed files for a folder (paginated)
func (s *Syncweb) GetLocalChangedFiles(folderID string, page, perPage int) ([]map[string]any, error) {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return nil, errors.New("internals not initialized")
	}
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
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return nil, nil, nil, errors.New("internals not initialized")
	}
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
func (s *Syncweb) GetRemoteNeedFiles(
	folderID string,
	deviceID protocol.DeviceID,
	page, perPage int,
) ([]map[string]any, error) {
	if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
		return nil, errors.New("internals not initialized")
	}
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

func (s *Syncweb) CountSeeders(folderID, path string) (int, error) {
	info, ok, err := s.GetGlobalFileInfo(folderID, path)
	if err != nil || !ok {
		return 0, err
	}

	deviceSet := make(map[string]bool)
	for _, block := range info.Blocks {
		if s.Node == nil || s.Node.App == nil || s.Node.App.Internals == nil {
			return 0, errors.New("internals not initialized")
		}
		availables, err := s.Node.App.Internals.BlockAvailability(folderID, info, block)
		if err != nil {
			continue
		}
		for _, av := range availables {
			deviceSet[av.ID.String()] = true
		}
	}

	return len(deviceSet), nil
}
