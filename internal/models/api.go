package models

type CatStat struct {
	Category string `json:"category"`
	Count    int64  `json:"count"`
}

type RatStat struct {
	Rating int64 `json:"rating"`
	Count  int64 `json:"count"`
}

type GenreStat struct {
	Genre string `json:"genre"`
	Count int64  `json:"count"`
}

type DatabaseInfo struct {
	Databases []string `json:"databases"`
	Trashcan  bool     `json:"trashcan"`
	ReadOnly  bool     `json:"read_only"`
	Dev       bool     `json:"dev"`
}

type PlayResponse struct {
	Path string `json:"path"`
}

type DeleteRequest struct {
	Path    string `json:"path"`
	Restore bool   `json:"restore"`
}

type ProgressRequest struct {
	Path      string `json:"path"`
	Playhead  int64  `json:"playhead"`
	Duration  int64  `json:"duration"`
	Completed bool   `json:"completed"`
}

type FilterBin struct {
	Label string `json:"label"`
	Min   int64  `json:"min,omitempty"`
	Max   int64  `json:"max,omitempty"`
	Value int64  `json:"value,omitempty"`
}

type FilterBinsResponse struct {
	Episodes []FilterBin `json:"episodes"`
	Size     []FilterBin `json:"size"`
	Duration []FilterBin `json:"duration"`

	EpisodesMin int64 `json:"episodes_min"`
	EpisodesMax int64 `json:"episodes_max"`
	SizeMin     int64 `json:"size_min"`
	SizeMax     int64 `json:"size_max"`
	DurationMin int64 `json:"duration_min"`
	DurationMax int64 `json:"duration_max"`

	EpisodesPercentiles []int64 `json:"episodes_percentiles"`
	SizePercentiles     []int64 `json:"size_percentiles"`
	DurationPercentiles []int64 `json:"duration_percentiles"`
}

type PlaylistResponse []string

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
