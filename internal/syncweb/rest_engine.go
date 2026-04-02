package syncweb

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"time"
	"iter"
	"crypto/tls"

	"github.com/araddon/dateparse"
	"github.com/syncthing/syncthing/lib/config"
	"github.com/syncthing/syncthing/lib/protocol"
	stmodel "github.com/syncthing/syncthing/lib/model"

	"github.com/chapmanjacobd/syncweb/internal/models"
)

// RESTEngine implements the Engine interface by communicating with a running Syncweb server
// for live data, but reading configuration locally for better security and efficiency.
type RESTEngine struct {
	HomeDir       string
	BaseURL       string
	APIToken      string
	client        *http.Client
	cachedFolders []FolderInfo
	lastFetch     time.Time
}

func NewRESTEngine(homeDir, baseURL, apiToken string) *RESTEngine {
	if !strings.HasPrefix(baseURL, "http") {
		baseURL = "http://" + baseURL
	}
	return &RESTEngine{
		HomeDir:  homeDir,
		BaseURL:  baseURL,
		APIToken: apiToken,
		client: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
}

func (e *RESTEngine) do(method, path string, body any) (*http.Response, error) {
	var bodyReader io.Reader
	if body != nil {
		jsonData, err := json.Marshal(body)
		if err != nil {
			return nil, err
		}
		bodyReader = bytes.NewBuffer(jsonData)
	}

	req, err := http.NewRequest(method, e.BaseURL+path, bodyReader)
	if err != nil {
		return nil, err
	}

	if e.APIToken != "" {
		req.Header.Set("X-Syncweb-Token", e.APIToken)
	}
	req.Header.Set("Content-Type", "application/json")

	return e.client.Do(req)
}

func (e *RESTEngine) getJSON(path string, target any) error {
	resp, err := e.do("GET", path, nil)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("API request failed with status %d", resp.StatusCode)
	}

	return json.NewDecoder(resp.Body).Decode(target)
}

// Implement Engine interface

func (e *RESTEngine) Start() error {
	return nil
}

func (e *RESTEngine) Stop() {
}

func (e *RESTEngine) IsRunning() bool {
	var status struct {
		Offline bool `json:"offline"`
	}
	if err := e.getJSON("/api/syncweb/status", &status); err != nil {
		return false
	}
	return !status.Offline
}

func (e *RESTEngine) MyID() protocol.DeviceID {
	certPath := filepath.Join(e.HomeDir, "cert.pem")
	keyPath := filepath.Join(e.HomeDir, "key.pem")
	
	cert, err := tls.LoadX509KeyPair(certPath, keyPath)
	if err != nil {
		return protocol.EmptyDeviceID
	}
	return protocol.NewDeviceID(cert.Certificate[0])
}

func (e *RESTEngine) RawConfig() config.Configuration {
	cfgPath := filepath.Join(e.HomeDir, "config.xml")
	f, err := os.Open(cfgPath)
	if err != nil {
		return config.Configuration{}
	}
	defer f.Close()

	// We need an ID to read the XML, but it's only used for some defaults
	myID := e.MyID()
	cfg, _, err := config.ReadXML(f, myID)
	if err != nil {
		return config.Configuration{}
	}
	return cfg
}

func (e *RESTEngine) SaveConfig() error {
	return fmt.Errorf("cannot save config in REST mode (server is running)")
}

func (e *RESTEngine) GetFolders() []FolderInfo {
	var folders []FolderInfo
	_ = e.getJSON("/api/syncweb/folders", &folders)
	e.cachedFolders = folders
	e.lastFetch = time.Now()
	return folders
}

func (e *RESTEngine) getCachedFolder(id string) (FolderInfo, bool) {
	if time.Since(e.lastFetch) > 5*time.Second || e.cachedFolders == nil {
		e.GetFolders()
	}
	for _, f := range e.cachedFolders {
		if f.ID == id {
			return f, true
		}
	}
	return FolderInfo{}, false
}

func (e *RESTEngine) GlobalSize(folderID string) (Counts, error) {
	if f, ok := e.getCachedFolder(folderID); ok {
		return f.GlobalSize, nil
	}
	return Counts{}, nil
}

func (e *RESTEngine) LocalSize(folderID string) (Counts, error) {
	if f, ok := e.getCachedFolder(folderID); ok {
		return f.LocalSize, nil
	}
	return Counts{}, nil
}

func (e *RESTEngine) NeedSize(folderID string, deviceID protocol.DeviceID) (Counts, error) {
	if f, ok := e.getCachedFolder(folderID); ok {
		return f.NeedSize, nil
	}
	return Counts{}, nil
}

func (e *RESTEngine) FolderState(folderID string) (string, time.Time, error) {
	if f, ok := e.getCachedFolder(folderID); ok {
		return f.State, time.Time{}, nil
	}
	return "", time.Time{}, nil
}

func (e *RESTEngine) FolderProgressBytesCompleted(folderID string) int64 {
	if f, ok := e.getCachedFolder(folderID); ok {
		return f.Completed
	}
	return 0
}

func (e *RESTEngine) GetDevices() []DeviceInfo {
	var devices []DeviceInfo
	_ = e.getJSON("/api/syncweb/devices", &devices)
	return devices
}

func (e *RESTEngine) GetFolderStats() map[string]map[string]any {
	var stats map[string]map[string]any
	_ = e.getJSON("/api/syncweb/status", &stats)
	return stats
}

func (e *RESTEngine) GetDeviceStats() map[string]map[string]any {
	return nil
}

func (e *RESTEngine) AddDevice(deviceID, name string, introducer bool) error {
	req := map[string]any{
		"id":         deviceID,
		"name":       name,
		"introducer": introducer,
	}
	resp, err := e.do("POST", "/api/syncweb/devices/add", req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (e *RESTEngine) DeleteDevice(id string) error {
	req := map[string]any{"id": id}
	resp, err := e.do("POST", "/api/syncweb/devices/delete", req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (e *RESTEngine) PauseDevice(id string) error {
	return fmt.Errorf("not implemented via REST API")
}

func (e *RESTEngine) ResumeDevice(id string) error {
	return fmt.Errorf("not implemented via REST API")
}

func (e *RESTEngine) IsConnectedTo(deviceID protocol.DeviceID) bool {
	devices := e.GetDevices()
	for _, d := range devices {
		if d.ID == deviceID.String() {
			return true
		}
	}
	return false
}

func (e *RESTEngine) SetDeviceAddresses(deviceID string, addresses []string) error {
	return fmt.Errorf("not implemented via REST API")
}

func (e *RESTEngine) GetDiscoveredDevices() map[string]time.Time {
	return nil
}

func (e *RESTEngine) AddFolder(id, label, path string, folderType config.FolderType) error {
	req := map[string]any{
		"id":   id,
		"path": path,
	}
	resp, err := e.do("POST", "/api/syncweb/folders/add", req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (e *RESTEngine) DeleteFolder(id string) error {
	req := map[string]any{"id": id}
	resp, err := e.do("POST", "/api/syncweb/folders/delete", req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (e *RESTEngine) PauseFolder(id string) error {
	return fmt.Errorf("not implemented via REST API")
}

func (e *RESTEngine) ResumeFolder(id string) error {
	return fmt.Errorf("not implemented via REST API")
}

func (e *RESTEngine) GetFolderPath(folderID string) (string, bool) {
	folders := e.GetFolders()
	for _, f := range folders {
		if f.ID == folderID {
			return f.Path, true
		}
	}
	return "", false
}

func (e *RESTEngine) ScanFolders() map[string]error {
	resp, err := e.do("POST", "/api/syncweb/scan", nil)
	if err != nil {
		return map[string]error{"all": err}
	}
	defer resp.Body.Close()
	return nil
}

func (e *RESTEngine) ScanFolderSubdirs(folderID string, paths []string) error {
	return fmt.Errorf("not implemented via REST API")
}

func (e *RESTEngine) WaitUntilIdle(folderID string, timeout time.Duration) error {
	return nil
}

func (e *RESTEngine) GetGlobalFileInfo(folderID, path string) (protocol.FileInfo, bool, error) {
	var info struct {
		Global protocol.FileInfo `json:"global"`
	}
	pathEscaped := url.QueryEscape(path)
	err := e.getJSON(fmt.Sprintf("/api/syncweb/stat?folder=%s&file=%s", folderID, pathEscaped), &info)
	if err != nil {
		return protocol.FileInfo{}, false, err
	}
	return info.Global, true, nil
}

func (e *RESTEngine) AllGlobalFiles(folderID string) (iter.Seq[FileMetadata], func() error) {
	ctx, cancel := context.WithCancel(context.Background())

	return func(yield func(FileMetadata) bool) {
		var entries []models.LsEntry
		err := e.getJSON(fmt.Sprintf("/api/syncweb/ls?folder=%s&recursive=true", folderID), &entries)
		if err != nil {
			return
		}

		for _, entry := range entries {
			select {
			case <-ctx.Done():
				return
			default:
				modNanos := int64(0)
				if entry.Modified != "" {
					if t, err := dateparse.ParseAny(entry.Modified); err == nil {
						modNanos = t.UnixNano()
					}
				}
				f := FileMetadata{
					Name:     entry.Name,
					Size:     entry.Size,
					ModNanos: modNanos,
				}
				if entry.IsDir {
					f.Type = protocol.FileInfoTypeDirectory
				} else {
					f.Type = protocol.FileInfoTypeFile
				}
				if !yield(f) {
					return
				}
			}
		}
	}, func() error { cancel(); return nil }
}

func (e *RESTEngine) ResolveLocalPath(syncPath string) (folderID, localPath string, err error) {
	return "", "", fmt.Errorf("not implemented via REST API")
}

func (e *RESTEngine) NewReadSeeker(ctx context.Context, folderID, path string) (io.ReadSeeker, error) {
	return nil, fmt.Errorf("not implemented via REST API")
}

func (e *RESTEngine) GetIgnores(folderID string) ([]string, error) {
	var res struct {
		Ignore []string `json:"ignore"`
	}
	err := e.getJSON(fmt.Sprintf("/api/syncweb/ignores?folder=%s", folderID), &res)
	return res.Ignore, err
}

func (e *RESTEngine) SetIgnores(folderID string, lines []string) error {
	req := map[string]any{
		"folder": folderID,
		"ignore": lines,
	}
	resp, err := e.do("POST", "/api/syncweb/ignores", req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (e *RESTEngine) AddIgnores(folderID string, unignores []string) error {
	return fmt.Errorf("not implemented via REST API")
}

func (e *RESTEngine) Unignore(folderID, relativePath string) error {
	req := map[string]any{
		"folder": folderID,
		"file":   relativePath,
	}
	resp, err := e.do("POST", "/api/syncweb/download", req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (e *RESTEngine) GetGlobalTree(folderID, prefix string, levels int, returnOnlyDirectories bool) ([]models.LsEntry, error) {
	var entries []models.LsEntry
	err := e.getJSON(fmt.Sprintf("/api/syncweb/ls?folder=%s&prefix=%s", folderID, url.QueryEscape(prefix)), &entries)
	return entries, err
}

func (e *RESTEngine) GetLocalChangedFiles(folderID string, page, perPage int) ([]map[string]any, error) {
	var files []map[string]any
	err := e.getJSON(fmt.Sprintf("/api/syncweb/folders/local-changed?folder=%s&page=%d&per_page=%d", folderID, page, perPage), &files)
	return files, err
}

func (e *RESTEngine) GetNeedFiles(folderID string, page, perPage int) (remote, local, queued []map[string]any, err error) {
	var res struct {
		Remote []map[string]any `json:"remote"`
		Local  []map[string]any `json:"local"`
		Queued []map[string]any `json:"queued"`
	}
	err = e.getJSON(fmt.Sprintf("/api/syncweb/folders/need?folder=%s&page=%d&per_page=%d", folderID, page, perPage), &res)
	return res.Remote, res.Local, res.Queued, err
}

func (e *RESTEngine) GetRemoteNeedFiles(folderID string, deviceID protocol.DeviceID, page, perPage int) ([]map[string]any, error) {
	var files []map[string]any
	err := e.getJSON(fmt.Sprintf("/api/syncweb/folders/remote-need?folder=%s&device=%s&page=%d&per_page=%d", folderID, deviceID.String(), page, perPage), &files)
	return files, err
}

func (e *RESTEngine) GetEvents() []models.SyncEvent {
	var events []models.SyncEvent
	_ = e.getJSON("/api/syncweb/events", &events)
	return events
}

func (e *RESTEngine) GetPendingDevices() map[string]time.Time {
	var res struct {
		Devices map[string]time.Time `json:"devices"`
	}
	_ = e.getJSON("/api/syncweb/pending", &res)
	return res.Devices
}

func (e *RESTEngine) GetPendingFolders() map[string]map[string]any {
	var res struct {
		Folders map[string]map[string]any `json:"folders"`
	}
	_ = e.getJSON("/api/syncweb/pending", &res)
	return res.Folders
}

func (e *RESTEngine) GetCompletion(deviceID protocol.DeviceID, folderID string) (stmodel.FolderCompletion, error) {
	var comp stmodel.FolderCompletion
	err := e.getJSON(fmt.Sprintf("/api/syncweb/completion?folder=%s&device=%s", folderID, deviceID.String()), &comp)
	return comp, err
}

func (e *RESTEngine) BlockAvailability(folderID string, info protocol.FileInfo, block protocol.BlockInfo) ([]stmodel.Availability, error) {
	return nil, nil
}

func (e *RESTEngine) CountSeeders(folderID, path string) (int, error) {
	var info struct {
		Global protocol.FileInfo `json:"global"`
	}
	err := e.getJSON(fmt.Sprintf("/api/syncweb/stat?folder=%s&file=%s", folderID, url.QueryEscape(path)), &info)
	if err != nil {
		return 0, err
	}
	return 0, nil
}

func (e *RESTEngine) AddFolderDevice(folderID, deviceID string) error {
	req := map[string]any{
		"folder": folderID,
		"device": deviceID,
	}
	resp, err := e.do("POST", "/api/syncweb/folders/join", req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (e *RESTEngine) AddFolderDevices(folderID string, deviceIDs []string) error {
	for _, did := range deviceIDs {
		if err := e.AddFolderDevice(folderID, did); err != nil {
			return err
		}
	}
	return nil
}

func (e *RESTEngine) RemoveFolderDevices(folderID string, deviceIDs []string) error {
	return fmt.Errorf("not implemented via REST API")
}
