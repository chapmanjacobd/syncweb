package syncweb

import (
	"context"
	"io"
	"iter"
	"time"

	"github.com/syncthing/syncthing/lib/config"
	stmodel "github.com/syncthing/syncthing/lib/model"
	"github.com/syncthing/syncthing/lib/protocol"

	"github.com/chapmanjacobd/syncweb/internal/models"
)

// Counts matches Syncthing's internal/db.Counts
type Counts struct {
	Files       int
	Directories int
	Symlinks    int
	Deleted     int
	Bytes       int64
	Sequence    int64
	DeviceID    protocol.DeviceID
	LocalFlags  protocol.FlagLocal
}

// FileMetadata matches Syncthing's internal/db.FileMetadata
type FileMetadata struct {
	Name       string
	Sequence   int64
	ModNanos   int64
	Size       int64
	LocalFlags protocol.FlagLocal
	Type       protocol.FileInfoType
	Deleted    bool
}

func (f FileMetadata) ModTime() time.Time {
	return time.Unix(0, f.ModNanos)
}

// Engine is the interface that both local and remote Syncweb instances implement
type Engine interface {
	// Lifecycle
	Start() error
	Stop()
	IsRunning() bool

	// Configuration
	MyID() protocol.DeviceID
	RawConfig() config.Configuration
	SaveConfig() error

	// Folders
	GetFolders() []FolderInfo
	GetFolderStats() map[string]map[string]any
	AddFolder(id, label, path string, folderType config.FolderType) error
	DeleteFolder(id string) error
	PauseFolder(id string) error
	ResumeFolder(id string) error
	GetFolderPath(folderID string) (string, bool)
	ScanFolders() map[string]error
	ScanFolderSubdirs(folderID string, paths []string) error
	WaitUntilIdle(folderID string, timeout time.Duration) error

	// Devices
	GetDevices() []DeviceInfo
	GetDeviceStats() map[string]map[string]any
	AddDevice(deviceID, name string, introducer bool) error
	DeleteDevice(id string) error
	PauseDevice(id string) error
	ResumeDevice(id string) error
	IsConnectedTo(deviceID protocol.DeviceID) bool
	SetDeviceAddresses(deviceID string, addresses []string) error
	GetDiscoveredDevices() map[string]time.Time

	// Files
	GetGlobalFileInfo(folderID, path string) (protocol.FileInfo, bool, error)
	AllGlobalFiles(folderID string) (iter.Seq[FileMetadata], func() error)
	ResolveLocalPath(syncPath string) (folderID, localPath string, err error)
	NewReadSeeker(ctx context.Context, folderID, path string) (io.ReadSeeker, error)
	GetIgnores(folderID string) ([]string, error)
	SetIgnores(folderID string, lines []string) error
	AddIgnores(folderID string, unignores []string) error
	Unignore(folderID, relativePath string) error
	GetGlobalTree(folderID, prefix string, levels int, returnOnlyDirectories bool) ([]models.LsEntry, error)
	GetLocalChangedFiles(folderID string, page, perPage int) ([]map[string]any, error)
	GetNeedFiles(folderID string, page, perPage int) (remote, local, queued []map[string]any, err error)
	GetRemoteNeedFiles(folderID string, deviceID protocol.DeviceID, page, perPage int) ([]map[string]any, error)

	// Stats and Info
	GetEvents() []models.SyncEvent
	GetPendingDevices() map[string]time.Time
	GetPendingFolders() map[string]map[string]any
	GlobalSize(folderID string) (Counts, error)
	LocalSize(folderID string) (Counts, error)
	NeedSize(folderID string, deviceID protocol.DeviceID) (Counts, error)
	FolderState(folderID string) (string, time.Time, error)
	FolderProgressBytesCompleted(folderID string) int64
	GetCompletion(deviceID protocol.DeviceID, folderID string) (stmodel.FolderCompletion, error)

	// Internal/Low-level (needed by some CLI commands)
	BlockAvailability(
		folderID string,
		info *protocol.FileInfo,
		block protocol.BlockInfo,
	) ([]stmodel.Availability, error)
	CountSeeders(folderID, path string) (int, error)

	// Syncweb-specific high-level operations
	AddFolderDevice(folderID, deviceID string) error
	AddFolderDevices(folderID string, deviceIDs []string) error
	RemoveFolderDevices(folderID string, deviceIDs []string) error
}
