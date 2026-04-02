package models

// Mountpoint represents a mountpoint in the system
type Mountpoint struct {
	Name        string   `json:"name"`
	Mountpoints []string `json:"mountpoints"`
	Size        string   `json:"size"`
	Type        string   `json:"type"`
	Label       string   `json:"label"`
	FSType      string   `json:"fstype"`
}

type BlockDevice struct {
	Name        string        `json:"name"`
	Mountpoints []string      `json:"mountpoints"`
	Size        string        `json:"size"`
	Type        string        `json:"type"`
	Label       string        `json:"label"`
	FSType      string        `json:"fstype"`
	Children    []BlockDevice `json:"children,omitempty"`
}

type ErrorResponse struct {
	Error string `json:"error"`
}

type SyncEvent struct {
	Time    string `json:"time"`
	Type    string `json:"type"`
	Message string `json:"message"`
	Data    any    `json:"data,omitempty"`
}

type LsEntry struct {
	Name     string `json:"name"`
	Path     string `json:"path"`
	IsDir    bool   `json:"is_dir"`
	Local    bool   `json:"local"`
	Size     int64  `json:"size"`
	Type     string `json:"type,omitempty"`
	Modified string `json:"modified,omitempty"`
}
