package models

import (
	"log/slog"
	"strings"
)

// CoreFlags are essential flags shared across most binaries/commands
type CoreFlags struct {
	// Common options
	Verbose   bool   `help:"Enable verbose logging"       short:"v"`
	JSON      bool   `help:"Output results as JSON" short:"j" group:"Display"`
	Simulate  bool   `help:"Dry run; don't actually do anything" aliases:"dry-run"`
	DryRun    bool   `kong:"-"` // Alias for Simulate
	NoConfirm bool   `help:"Don't ask for confirmation" short:"y" aliases:"yes"`
	Yes       bool   `kong:"-"` // Alias for NoConfirm
	Timeout   string `help:"Quit after N minutes/seconds" short:"T"`
}

// SyncwebFlags are flags related to Syncweb configuration
type SyncwebFlags struct {
	SyncwebURL      string `help:"Syncweb/Syncthing API URL" env:"SYNCWEB_URL" group:"Syncweb"`
	SyncwebAPIKey   string `help:"Syncweb/Syncthing API Key" env:"SYNCWEB_API_KEY" group:"Syncweb"`
	SyncwebHome     string `help:"Syncweb home directory" env:"SYNCWEB_HOME" group:"Syncweb"`
	SyncwebPublic_  string `env:"SYNCWEB_PUBLIC"  kong:"-"`
	SyncwebPrivate_ string `env:"SYNCWEB_PRIVATE" kong:"-"`
	Decode          bool   `help:"Decode percent-encoding and punycode in URLs" default:"true" group:"Syncweb" negatable:""`
}

type QueryFlags struct {
	Query  string `help:"Raw SQL query (overrides all query building)" short:"q" group:"Query"`
	Limit  int    `help:"Limit results per database" default:"100" short:"L" group:"Query"`
	All    bool   `help:"Return all results (no limit)" short:"a" group:"Query"`
	Offset int    `help:"Skip N results" group:"Query"`
}

type PathFilterFlags struct {
	Include      []string `help:"Include paths matching pattern" short:"s" group:"PathFilter"`
	Exclude      []string `help:"Exclude paths matching pattern" short:"E" group:"PathFilter"`
	Regex        string   `help:"Filter paths by regex pattern" group:"PathFilter"`
	PathContains []string `help:"Path must contain all these strings" group:"PathFilter"`
	Paths        []string `help:"Exact paths to include" group:"PathFilter"`
}

type FilterFlags struct {
	Search           []string `help:"Search terms (space-separated for AND, | for OR)" group:"Filter"`
	Size             []string `help:"Size range (e.g., >100MB, 1GB%10)" short:"S" group:"Filter"`
	Duration         []string `help:"Duration range (e.g., >1hour, 30min%10)" short:"d" group:"Filter"`
	DurationFromSize string   `help:"Constrain media to duration of videos which match any size constraints" group:"Filter"`
	Watched          *bool    `help:"Filter by watched status (true/false)" group:"Filter"`
	Unfinished       bool     `help:"Has playhead but not finished" group:"Filter"`
	Partial          string   `help:"Filter by partial playback status" short:"P" group:"Filter"`
	PlayCountMin     int      `help:"Minimum play count" group:"Filter"`
	PlayCountMax     int      `help:"Maximum play count" group:"Filter"`
	Completed        bool     `help:"Show only completed items" group:"Filter"`
	InProgress       bool     `help:"Show only items in progress" group:"Filter"`
	WithCaptions     bool     `help:"Show only items with captions" group:"Filter"`
	FlexibleSearch   bool     `help:"Flexible search (fuzzy)" group:"Filter"`
	Exact            bool     `help:"Exact match for search" group:"Filter"`
	Where            []string `help:"SQL where clause(s)" short:"w" group:"Filter"`
	Exists           bool     `help:"Filter out non-existent files" group:"Filter"`
	FetchSiblings    string   `help:"Fetch siblings of matched files (each, all, if-audiobook)" short:"o" group:"Filter"`
	FetchSiblingsMax int      `help:"Maximum number of siblings to fetch" group:"Filter"`
}

type MediaFilterFlags struct {
	Category            []string `help:"Filter by category" group:"MediaFilter"`
	Genre               string   `help:"Filter by genre" group:"MediaFilter"`
	Ext                 []string `help:"Filter by extensions (e.g., .mp4,.mkv)" short:"e" group:"MediaFilter"`
	VideoOnly           bool     `help:"Only video files" group:"MediaFilter"`
	AudioOnly           bool     `help:"Only audio files" group:"MediaFilter"`
	ImageOnly           bool     `help:"Only image files" group:"MediaFilter"`
	TextOnly            bool     `help:"Only text/ebook files" group:"MediaFilter"`
	Portrait            bool     `help:"Only portrait orientation files" group:"MediaFilter"`
	ScanSubtitles       bool     `help:"Scan for external subtitles during import" group:"MediaFilter"`
	OnlineMediaOnly     bool     `help:"Exclude local media" group:"MediaFilter"`
	LocalMediaOnly      bool     `help:"Exclude online media" group:"MediaFilter"`
	MimeType            []string `help:"Filter by mimetype substring (e.g., video, mp4)" group:"MediaFilter"`
	NoMimeType          []string `help:"Exclude by mimetype substring" group:"MediaFilter"`
	NoDefaultCategories bool     `help:"Disable default categories" group:"MediaFilter"`
}

type TimeFilterFlags struct {
	CreatedAfter   string `help:"Created after date (YYYY-MM-DD)" group:"Time"`
	CreatedBefore  string `help:"Created before date (YYYY-MM-DD)" group:"Time"`
	ModifiedAfter  string `help:"Modified after date (YYYY-MM-DD)" group:"Time"`
	ModifiedBefore string `help:"Modified before date (YYYY-MM-DD)" group:"Time"`
	DeletedAfter   string `help:"Deleted after date (YYYY-MM-DD)" group:"Time"`
	DeletedBefore  string `help:"Deleted before date (YYYY-MM-DD)" group:"Time"`
	PlayedAfter    string `help:"Last played after date (YYYY-MM-DD)" group:"Time"`
	PlayedBefore   string `help:"Last played before date (YYYY-MM-DD)" group:"Time"`
}

type DeletedFlags struct {
	HideDeleted bool `help:"Exclude deleted files from results" default:"true" group:"Deleted"`
	OnlyDeleted bool `help:"Include only deleted files in results" group:"Deleted"`
}

type SortFlags struct {
	SortBy  string `help:"Sort by field" default:"path" short:"u" group:"Sort"`
	Reverse bool   `help:"Reverse sort order" short:"V" group:"Sort"`
	NatSort bool   `help:"Use natural sorting" short:"n" group:"Sort"`
	Random  bool   `help:"Random order" short:"r" group:"Sort"`
	ReRank  string `help:"Add key/value pairs re-rank sorting by multiple attributes (COLUMN=WEIGHT)" alias:"rerank" short:"k" group:"Sort"`
}

type DisplayFlags struct {
	Columns   []string `help:"Columns to display" short:"c" group:"Display"`
	Summarize bool     `help:"Print aggregate statistics" group:"Display"`
	Frequency string   `help:"Group statistics by time frequency (daily, weekly, monthly, yearly)" short:"f" group:"Display"`
	TUI       bool     `help:"Interactive TUI mode" group:"Display"`
}

type AggregateFlags struct {
	BigDirs           bool     `help:"Aggregate by parent directory" short:"B" group:"Aggregate"`
	FileCounts        string   `help:"Filter by number of files in directory (e.g., >5, 10%1)" group:"Aggregate"`
	GroupByExtensions bool     `help:"Group by file extensions" group:"Aggregate"`
	GroupByMimeTypes  bool     `help:"Group by mimetypes" group:"Aggregate"`
	GroupBySize       bool     `help:"Group by size buckets" group:"Aggregate"`
	Depth             int      `help:"Aggregate at specific directory depth" short:"D" group:"Aggregate"`
	MinDepth          int      `help:"Minimum depth for aggregation" default:"0" group:"Aggregate"`
	MaxDepth          int      `help:"Maximum depth for aggregation" group:"Aggregate"`
	Parents           bool     `help:"Include parent directories in aggregation" group:"Aggregate"`
	FoldersOnly       bool     `help:"Only show folders" group:"Aggregate"`
	FilesOnly         bool     `help:"Only show files" group:"Aggregate"`
	FolderSizes       []string `help:"Filter folders by total size" group:"Aggregate"`
	FolderCounts      string   `help:"Filter folders by number of subfolders" group:"Aggregate"`
}

type TextFlags struct {
	RegexSort  bool     `help:"Sort by splitting lines and sorting words" alias:"rs" group:"Text"`
	Regexs     []string `help:"Regex patterns for line splitting" alias:"re" group:"Text"`
	WordSorts  []string `help:"Word sorting strategies" group:"Text"`
	LineSorts  []string `help:"Line sorting strategies" group:"Text"`
	Compat     bool     `help:"Use natsort compat mode" group:"Text"`
	Preprocess bool     `help:"Remove junk common to filenames and URLs" default:"true" group:"Text"`
	StopWords  []string `help:"List of words to ignore" group:"Text"`
	Duplicates *bool    `help:"Filter for duplicate words (true/false)" group:"Text"`
	UniqueOnly *bool    `help:"Filter for unique words (true/false)" group:"Text"`
}

type SimilarityFlags struct {
	Similar         bool    `help:"Find similar files or folders" group:"Similarity"`
	SizesDelta      float64 `help:"Size difference threshold (%)" default:"10.0" group:"Similarity"`
	CountsDelta     float64 `help:"File count difference threshold (%)" default:"3.0" group:"Similarity"`
	DurationsDelta  float64 `help:"Duration difference threshold (%)" default:"5.0" group:"Similarity"`
	FilterNames     bool    `help:"Cluster by name similarity" group:"Similarity"`
	FilterSizes     bool    `help:"Cluster by size similarity" group:"Similarity"`
	FilterCounts    bool    `help:"Cluster by count similarity" group:"Similarity"`
	FilterDurations bool    `help:"Cluster by duration similarity" group:"Similarity"`
	TotalSizes      bool    `help:"Compare total sizes (folders only)" group:"Similarity"`
	TotalDurations  bool    `help:"Compare total durations (folders only)" group:"Similarity"`
	OnlyDuplicates  bool    `help:"Only show duplicate items" group:"Similarity"`
	OnlyOriginals   bool    `help:"Only show original items" group:"Similarity"`
	ClusterSort     bool    `help:"Group items by similarity" short:"C" group:"Similarity"`
	Clusters        int     `help:"Number of clusters" group:"Similarity"`
	TFIDF           bool    `help:"Use TF-IDF for clustering" group:"Similarity"`
	MoveGroups      bool    `help:"Move grouped files into separate directories" group:"Similarity"`
	PrintGroups     bool    `help:"Print clusters as JSON" group:"Similarity"`
}

type DedupeFlags struct {
	Audio              bool    `help:"Dedupe database by artist + album + title" group:"Dedupe"`
	ExtractorID        bool    `help:"Dedupe database by extractor_id" alias:"id" group:"Dedupe"`
	TitleOnly          bool    `help:"Dedupe database by title" group:"Dedupe"`
	DurationOnly       bool    `help:"Dedupe database by duration" group:"Dedupe"`
	Filesystem         bool    `help:"Dedupe filesystem database (hash)" alias:"fs" group:"Dedupe"`
	CompareDirs        bool    `help:"Compare directories" group:"Dedupe"`
	Basename           bool    `help:"Match by basename similarity" group:"Dedupe"`
	Dirname            bool    `help:"Match by dirname similarity" group:"Dedupe"`
	MinSimilarityRatio float64 `help:"Filter out matches with less than this ratio (0.7-0.9)" default:"0.8" group:"Dedupe"`
	DedupeCmd          string  `help:"Command to run for deduplication (rmlint-style: cmd duplicate keep)" group:"Dedupe"`
}

type FTSFlags struct {
	FTS      bool   `help:"Use full-text search if available" group:"FTS"`
	FTSTable string `help:"FTS table name" default:"media_fts" group:"FTS"`
	Related  int    `help:"Find media related to the first result" short:"R" group:"FTS"`
}

type PlaybackFlags struct {
	PlayInOrder           string   `help:"Play media in order" default:"natural_ps" short:"O" group:"Playback"`
	NoPlayInOrder         bool     `help:"Don't play media in order" group:"Playback"`
	Loop                  bool     `help:"Loop playback" group:"Playback"`
	Mute                  bool     `help:"Start playback muted" short:"M" group:"Playback"`
	OverridePlayer        string   `help:"Override default player (e.g. --player 'vlc')" group:"Playback"`
	Start                 string   `help:"Start playback at specific time/percentage" group:"Playback"`
	End                   string   `help:"Stop playback at specific time/percentage" group:"Playback"`
	Volume                int      `help:"Set initial volume (0-100)" group:"Playback"`
	Fullscreen            bool     `help:"Start in fullscreen" group:"Playback"`
	NoSubtitles           bool     `help:"Disable subtitles" group:"Playback"`
	SubtitleMix           float64  `help:"Probability to play no-subtitle content" default:"0.35" group:"Playback"`
	InterdimensionalCable int      `help:"Duration to play (in seconds) while changing the channel" alias:"4dtv" short:"4" group:"Playback"`
	Speed                 float64  `help:"Playback speed" default:"1.0" group:"Playback"`
	SavePlayhead          bool     `help:"Save playback position on quit" default:"true" group:"Playback"`
	MpvSocket             string   `help:"Mpv socket path" group:"Playback"`
	WatchLaterDir         string   `help:"Mpv watch_later directory" group:"Playback"`
	PlayerArgsSub         []string `help:"Player arguments for videos with subtitles" group:"Playback"`
	PlayerArgsNoSub       []string `help:"Player arguments for videos without subtitles" group:"Playback"`
	Cast                  bool     `help:"Cast to chromecast groups" group:"Playback"`
	CastDevice            string   `help:"Chromecast device name" alias:"cast-to" group:"Playback"`
	CastWithLocal         bool     `help:"Play music locally at the same time as chromecast" group:"Playback"`
}

type MpvActionFlags struct {
	Cmd0        string `help:"Command to run if mpv exits with code 0" group:"MpvAction"`
	Cmd1        string `help:"Command to run if mpv exits with code 1" group:"MpvAction"`
	Cmd2        string `help:"Command to run if mpv exits with code 2" group:"MpvAction"`
	Cmd3        string `help:"Command to run if mpv exits with code 3" group:"MpvAction"`
	Cmd4        string `help:"Command to run if mpv exits with code 4" group:"MpvAction"`
	Cmd5        string `help:"Command to run if mpv exits with code 5" group:"MpvAction"`
	Cmd6        string `help:"Command to run if mpv exits with code 6" group:"MpvAction"`
	Cmd7        string `help:"Command to run if mpv exits with code 7" group:"MpvAction"`
	Cmd8        string `help:"Command to run if mpv exits with code 8" group:"MpvAction"`
	Cmd9        string `help:"Command to run if mpv exits with code 9" group:"MpvAction"`
	Cmd10       string `help:"Command to run if mpv exits with code 10" group:"MpvAction"`
	Cmd11       string `help:"Command to run if mpv exits with code 11" group:"MpvAction"`
	Cmd12       string `help:"Command to run if mpv exits with code 12" group:"MpvAction"`
	Cmd13       string `help:"Command to run if mpv exits with code 13" group:"MpvAction"`
	Cmd14       string `help:"Command to run if mpv exits with code 14" group:"MpvAction"`
	Cmd15       string `help:"Command to run if mpv exits with code 15" group:"MpvAction"`
	Cmd20       string `help:"Command to run if mpv exits with code 20" group:"MpvAction"`
	Cmd127      string `help:"Command to run if mpv exits with code 127" group:"MpvAction"`
	Interactive bool   `help:"Interactive decision making after playback" short:"I" group:"MpvAction"`
}

type PostActionFlags struct {
	Trash        bool   `help:"Trash files after action" group:"PostAction"`
	PostAction   string `help:"Post-action: none, delete, mark-deleted, move, copy" group:"PostAction"`
	DeleteFiles  bool   `help:"Delete files after action" group:"PostAction"`
	DeleteRows   bool   `help:"Delete rows from database" group:"PostAction"`
	MarkDeleted  bool   `help:"Mark as deleted in database" group:"PostAction"`
	MoveTo       string `help:"Move files to directory" group:"PostAction"`
	CopyTo       string `help:"Copy files to directory" group:"PostAction"`
	ActionLimit  int    `help:"Stop after N files" group:"PostAction"`
	ActionSize   string `help:"Stop after N bytes (e.g., 10GB)" group:"PostAction"`
	TrackHistory bool   `help:"Track playback history" default:"true" group:"PostAction"`
}

type HashingFlags struct {
	HashGap       float64 `help:"Gap between segments (0.0-1.0 as percentage of file size, or absolute bytes if >1)" default:"0.1" group:"Hashing"`
	HashChunkSize int64   `help:"Size of each segment to hash" group:"Hashing"`
	HashThreads   int     `help:"Number of threads to use for hashing a single file" default:"1" group:"Hashing"`
}

type MergeFlags struct {
	OnlyTables        []string `help:"Comma separated specific table(s)" short:"t" group:"Merge"`
	PrimaryKeys       []string `help:"Comma separated primary keys" group:"Merge"`
	BusinessKeys      []string `help:"Comma separated business keys" group:"Merge"`
	Upsert            bool     `help:"Upsert rows on conflict" group:"Merge"`
	Ignore            bool     `help:"Ignore rows on conflict (only-new-rows)" group:"Merge"`
	OnlyNewRows       bool     `kong:"-"` // Alias for Ignore
	OnlyTargetColumns bool     `help:"Only copy columns that exist in target" group:"Merge"`
	SkipColumns       []string `help:"Columns to skip during merge" group:"Merge"`
}

// GlobalFlags are flags available to disco data commands (print, search, du, etc)
type GlobalFlags struct {
	CoreFlags        `embed:""`
	SyncwebFlags     `embed:""`
	QueryFlags       `embed:""`
	PathFilterFlags  `embed:""`
	FilterFlags      `embed:""`
	MediaFilterFlags `embed:""`
	TimeFilterFlags  `embed:""`
	DeletedFlags     `embed:""`
	SortFlags        `embed:""`
	DisplayFlags     `embed:""`
	AggregateFlags   `embed:""`
	TextFlags        `embed:""`
	SimilarityFlags  `embed:""`
	DedupeFlags      `embed:""`
	FTSFlags         `embed:""`
	PlaybackFlags    `embed:""`
	MpvActionFlags   `embed:""`
	PostActionFlags  `embed:""`
	HashingFlags     `embed:""`
	MergeFlags       `embed:""`

	Threads      int  `help:"Use N threads for parallel processing"`
	IgnoreErrors bool `help:"Ignore errors and continue to next file" short:"i"`
}

// ControlFlags are a subset of flags for simple control commands
type ControlFlags struct {
	MpvSocket  string `help:"Mpv socket path" group:"Playback"`
	CastDevice string `help:"Chromecast device name" alias:"cast-to" group:"Playback"`
	Verbose    bool   `help:"Enable verbose logging" short:"v"`
}

func (c *CoreFlags) AfterApply() error {
	if c.Simulate {
		c.DryRun = true
	}
	if c.NoConfirm {
		c.Yes = true
	}
	return nil
}

func (m *MediaFilterFlags) AfterApply() error {
	if m.Ext != nil {
		for i, ext := range m.Ext {
			if !strings.HasPrefix(ext, ".") {
				m.Ext[i] = "." + ext
			}
		}
	}
	return nil
}

func (m *MergeFlags) AfterApply() error {
	if m.Ignore {
		m.OnlyNewRows = true
	}
	return nil
}

func (g *GlobalFlags) AfterApply() error {
	if err := g.CoreFlags.AfterApply(); err != nil {
		return err
	}
	if err := g.MediaFilterFlags.AfterApply(); err != nil {
		return err
	}
	if err := g.MergeFlags.AfterApply(); err != nil {
		return err
	}
	return nil
}

var LogLevel = &slog.LevelVar{}

func SetupLogging(verbose bool) {
	if verbose {
		LogLevel.Set(slog.LevelDebug)
	} else {
		LogLevel.Set(slog.LevelInfo)
	}
}
