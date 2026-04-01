package models

import (
	"log/slog"
	"strings"
)

// CoreFlags are essential flags shared across most binaries/commands.
type CoreFlags struct {
	// Common options
	Verbose   bool   `help:"Enable verbose logging"              short:"v"`
	JSON      bool   `group:"Display"                            help:"Output results as JSON" short:"j"`
	Simulate  bool   `help:"Dry run; don't actually do anything"`
	DryRun    bool   `kong:"-"` // Alias for Simulate
	NoConfirm bool   `help:"Don't ask for confirmation"          short:"y"`
	Yes       bool   `kong:"-"` // Alias for NoConfirm
	Timeout   string `help:"Quit after N minutes/seconds"        short:"T"`
}

// SyncwebFlags are flags related to Syncweb configuration.
type SyncwebFlags struct {
	SyncwebURL      string `env:"SYNCWEB_URL"     group:"Syncweb" help:"Syncweb/Syncthing API URL"`
	SyncwebAPIKey   string `env:"SYNCWEB_API_KEY" group:"Syncweb" help:"Syncweb/Syncthing API Key"`
	SyncwebHome     string `env:"SYNCWEB_HOME"    group:"Syncweb" help:"Syncweb home directory"`
	SyncwebPublic_  string `env:"SYNCWEB_PUBLIC"  kong:"-"`
	SyncwebPrivate_ string `env:"SYNCWEB_PRIVATE" kong:"-"`
}

type QueryFlags struct {
	Query  string `group:"Query" help:"Raw SQL query (overrides all query building)" short:"q"`
	Limit  int    `default:"100" group:"Query"                                       help:"Limit results per database" short:"L"`
	All    bool   `group:"Query" help:"Return all results (no limit)"                short:"a"`
	Offset int    `group:"Query" help:"Skip N results"`
}

type PathFilterFlags struct {
	Include      []string `group:"PathFilter" help:"Include paths matching pattern"      short:"s"`
	Exclude      []string `group:"PathFilter" help:"Exclude paths matching pattern"      short:"E"`
	Regex        string   `group:"PathFilter" help:"Filter paths by regex pattern"`
	PathContains []string `group:"PathFilter" help:"Path must contain all these strings"`
	Paths        []string `group:"PathFilter" help:"Exact paths to include"`
}

type FilterFlags struct {
	Search           []string `group:"Filter" help:"Search terms (space-separated for AND, | for OR)"`
	Size             []string `group:"Filter" help:"Size range (e.g., >100MB, 1GB%10)"                                      short:"S"`
	Duration         []string `group:"Filter" help:"Duration range (e.g., >1hour, 30min%10)"                                short:"d"`
	DurationFromSize string   `group:"Filter" help:"Constrain media to duration of videos which match any size constraints"`
	Watched          *bool    `group:"Filter" help:"Filter by watched status (true/false)"`
	Unfinished       bool     `group:"Filter" help:"Has playhead but not finished"`
	Partial          string   `group:"Filter" help:"Filter by partial playback status"                                      short:"P"`
	PlayCountMin     int      `group:"Filter" help:"Minimum play count"`
	PlayCountMax     int      `group:"Filter" help:"Maximum play count"`
	Completed        bool     `group:"Filter" help:"Show only completed items"`
	InProgress       bool     `group:"Filter" help:"Show only items in progress"`
	WithCaptions     bool     `group:"Filter" help:"Show only items with captions"`
	FlexibleSearch   bool     `group:"Filter" help:"Flexible search (fuzzy)"`
	Exact            bool     `group:"Filter" help:"Exact match for search"`
	Where            []string `group:"Filter" help:"SQL where clause(s)"                                                    short:"w"`
	Exists           bool     `group:"Filter" help:"Filter out non-existent files"`
	FetchSiblings    string   `group:"Filter" help:"Fetch siblings of matched files (each, all, if-audiobook)"              short:"o"`
	FetchSiblingsMax int      `group:"Filter" help:"Maximum number of siblings to fetch"`
}

type MediaFilterFlags struct {
	Category            []string `group:"MediaFilter" help:"Filter by category"`
	Genre               string   `group:"MediaFilter" help:"Filter by genre"`
	Ext                 []string `group:"MediaFilter" help:"Filter by extensions (e.g., .mp4,.mkv)"          short:"e"`
	VideoOnly           bool     `group:"MediaFilter" help:"Only video files"`
	AudioOnly           bool     `group:"MediaFilter" help:"Only audio files"`
	ImageOnly           bool     `group:"MediaFilter" help:"Only image files"`
	TextOnly            bool     `group:"MediaFilter" help:"Only text/ebook files"`
	Portrait            bool     `group:"MediaFilter" help:"Only portrait orientation files"`
	ScanSubtitles       bool     `group:"MediaFilter" help:"Scan for external subtitles during import"`
	OnlineMediaOnly     bool     `group:"MediaFilter" help:"Exclude local media"`
	LocalMediaOnly      bool     `group:"MediaFilter" help:"Exclude online media"`
	MimeType            []string `group:"MediaFilter" help:"Filter by mimetype substring (e.g., video, mp4)"`
	NoMimeType          []string `group:"MediaFilter" help:"Exclude by mimetype substring"`
	NoDefaultCategories bool     `group:"MediaFilter" help:"Disable default categories"`
}

type TimeFilterFlags struct {
	CreatedAfter   string `group:"Time" help:"Created after date (YYYY-MM-DD)"`
	CreatedBefore  string `group:"Time" help:"Created before date (YYYY-MM-DD)"`
	ModifiedAfter  string `group:"Time" help:"Modified after date (YYYY-MM-DD)"`
	ModifiedBefore string `group:"Time" help:"Modified before date (YYYY-MM-DD)"`
	DeletedAfter   string `group:"Time" help:"Deleted after date (YYYY-MM-DD)"`
	DeletedBefore  string `group:"Time" help:"Deleted before date (YYYY-MM-DD)"`
	PlayedAfter    string `group:"Time" help:"Last played after date (YYYY-MM-DD)"`
	PlayedBefore   string `group:"Time" help:"Last played before date (YYYY-MM-DD)"`
}

type DeletedFlags struct {
	HideDeleted bool `default:"true"  group:"Deleted"                              help:"Exclude deleted files from results"`
	OnlyDeleted bool `group:"Deleted" help:"Include only deleted files in results"`
}

type SortFlags struct {
	SortBy  string `default:"path" group:"Sort"               help:"Sort by field"                                                              short:"u"`
	Reverse bool   `group:"Sort"   help:"Reverse sort order"  short:"V"`
	NatSort bool   `group:"Sort"   help:"Use natural sorting" short:"n"`
	Random  bool   `group:"Sort"   help:"Random order"        short:"r"`
	ReRank  string `alias:"rerank" group:"Sort"               help:"Add key/value pairs re-rank sorting by multiple attributes (COLUMN=WEIGHT)" short:"k"`
}

type DisplayFlags struct {
	Columns   []string `group:"Display" help:"Columns to display"                                                  short:"c"`
	Summarize bool     `group:"Display" help:"Print aggregate statistics"`
	Frequency string   `group:"Display" help:"Group statistics by time frequency (daily, weekly, monthly, yearly)" short:"f"`
	TUI       bool     `group:"Display" help:"Interactive TUI mode"`
}

type AggregateFlags struct {
	BigDirs           bool     `group:"Aggregate" help:"Aggregate by parent directory"                           short:"B"`
	FileCounts        string   `group:"Aggregate" help:"Filter by number of files in directory (e.g., >5, 10%1)"`
	GroupByExtensions bool     `group:"Aggregate" help:"Group by file extensions"`
	GroupByMimeTypes  bool     `group:"Aggregate" help:"Group by mimetypes"`
	GroupBySize       bool     `group:"Aggregate" help:"Group by size buckets"`
	Depth             int      `group:"Aggregate" help:"Aggregate at specific directory depth"                   short:"D"`
	MinDepth          int      `default:"0"       group:"Aggregate"                                              help:"Minimum depth for aggregation"`
	MaxDepth          int      `group:"Aggregate" help:"Maximum depth for aggregation"`
	Parents           bool     `group:"Aggregate" help:"Include parent directories in aggregation"`
	FoldersOnly       bool     `group:"Aggregate" help:"Only show folders"`
	FilesOnly         bool     `group:"Aggregate" help:"Only show files"`
	FolderSizes       []string `group:"Aggregate" help:"Filter folders by total size"`
	FolderCounts      string   `group:"Aggregate" help:"Filter folders by number of subfolders"`
}

type TextFlags struct {
	RegexSort  bool     `alias:"rs"     group:"Text"                                   help:"Sort by splitting lines and sorting words"`
	Regexs     []string `alias:"re"     group:"Text"                                   help:"Regex patterns for line splitting"`
	WordSorts  []string `group:"Text"   help:"Word sorting strategies"`
	LineSorts  []string `group:"Text"   help:"Line sorting strategies"`
	Compat     bool     `group:"Text"   help:"Use natsort compat mode"`
	Preprocess bool     `default:"true" group:"Text"                                   help:"Remove junk common to filenames and URLs"`
	StopWords  []string `group:"Text"   help:"List of words to ignore"`
	Duplicates *bool    `group:"Text"   help:"Filter for duplicate words (true/false)"`
	UniqueOnly *bool    `group:"Text"   help:"Filter for unique words (true/false)"`
}

type SimilarityFlags struct {
	Similar         bool    `group:"Similarity" help:"Find similar files or folders"`
	SizesDelta      float64 `default:"10.0"     group:"Similarity"                                  help:"Size difference threshold (%)"`
	CountsDelta     float64 `default:"3.0"      group:"Similarity"                                  help:"File count difference threshold (%)"`
	DurationsDelta  float64 `default:"5.0"      group:"Similarity"                                  help:"Duration difference threshold (%)"`
	FilterNames     bool    `group:"Similarity" help:"Cluster by name similarity"`
	FilterSizes     bool    `group:"Similarity" help:"Cluster by size similarity"`
	FilterCounts    bool    `group:"Similarity" help:"Cluster by count similarity"`
	FilterDurations bool    `group:"Similarity" help:"Cluster by duration similarity"`
	TotalSizes      bool    `group:"Similarity" help:"Compare total sizes (folders only)"`
	TotalDurations  bool    `group:"Similarity" help:"Compare total durations (folders only)"`
	OnlyDuplicates  bool    `group:"Similarity" help:"Only show duplicate items"`
	OnlyOriginals   bool    `group:"Similarity" help:"Only show original items"`
	ClusterSort     bool    `group:"Similarity" help:"Group items by similarity"                    short:"C"`
	Clusters        int     `group:"Similarity" help:"Number of clusters"`
	TFIDF           bool    `group:"Similarity" help:"Use TF-IDF for clustering"`
	MoveGroups      bool    `group:"Similarity" help:"Move grouped files into separate directories"`
	PrintGroups     bool    `group:"Similarity" help:"Print clusters as JSON"`
}

type DedupeFlags struct {
	Audio              bool    `group:"Dedupe" help:"Dedupe database by artist + album + title"`
	ExtractorID        bool    `alias:"id"     group:"Dedupe"                                                             help:"Dedupe database by extractor_id"`
	TitleOnly          bool    `group:"Dedupe" help:"Dedupe database by title"`
	DurationOnly       bool    `group:"Dedupe" help:"Dedupe database by duration"`
	Filesystem         bool    `alias:"fs"     group:"Dedupe"                                                             help:"Dedupe filesystem database (hash)"`
	CompareDirs        bool    `group:"Dedupe" help:"Compare directories"`
	Basename           bool    `group:"Dedupe" help:"Match by basename similarity"`
	Dirname            bool    `group:"Dedupe" help:"Match by dirname similarity"`
	MinSimilarityRatio float64 `default:"0.8"  group:"Dedupe"                                                             help:"Filter out matches with less than this ratio (0.7-0.9)"`
	DedupeCmd          string  `group:"Dedupe" help:"Command to run for deduplication (rmlint-style: cmd duplicate keep)"`
}

type FTSFlags struct {
	FTS      bool   `group:"FTS"         help:"Use full-text search if available"`
	FTSTable string `default:"media_fts" group:"FTS"                                   help:"FTS table name"`
	Related  int    `group:"FTS"         help:"Find media related to the first result" short:"R"`
}

type PlaybackFlags struct {
	PlayInOrder           string   `default:"natural_ps" group:"Playback"                                         help:"Play media in order"                                      short:"O"`
	NoPlayInOrder         bool     `group:"Playback"     help:"Don't play media in order"`
	Loop                  bool     `group:"Playback"     help:"Loop playback"`
	Mute                  bool     `group:"Playback"     help:"Start playback muted"                              short:"M"`
	OverridePlayer        string   `group:"Playback"     help:"Override default player (e.g. --player 'vlc')"`
	Start                 string   `group:"Playback"     help:"Start playback at specific time/percentage"`
	End                   string   `group:"Playback"     help:"Stop playback at specific time/percentage"`
	Volume                int      `group:"Playback"     help:"Set initial volume (0-100)"`
	Fullscreen            bool     `group:"Playback"     help:"Start in fullscreen"`
	NoSubtitles           bool     `group:"Playback"     help:"Disable subtitles"`
	SubtitleMix           float64  `default:"0.35"       group:"Playback"                                         help:"Probability to play no-subtitle content"`
	InterdimensionalCable int      `alias:"4dtv"         group:"Playback"                                         help:"Duration to play (in seconds) while changing the channel" short:"4"`
	Speed                 float64  `default:"1.0"        group:"Playback"                                         help:"Playback speed"`
	SavePlayhead          bool     `default:"true"       group:"Playback"                                         help:"Save playback position on quit"`
	MpvSocket             string   `group:"Playback"     help:"Mpv socket path"`
	WatchLaterDir         string   `group:"Playback"     help:"Mpv watch_later directory"`
	PlayerArgsSub         []string `group:"Playback"     help:"Player arguments for videos with subtitles"`
	PlayerArgsNoSub       []string `group:"Playback"     help:"Player arguments for videos without subtitles"`
	Cast                  bool     `group:"Playback"     help:"Cast to chromecast groups"`
	CastDevice            string   `alias:"cast-to"      group:"Playback"                                         help:"Chromecast device name"`
	CastWithLocal         bool     `group:"Playback"     help:"Play music locally at the same time as chromecast"`
}

type MpvActionFlags struct {
	Cmd0        string `group:"MpvAction" help:"Command to run if mpv exits with code 0"`
	Cmd1        string `group:"MpvAction" help:"Command to run if mpv exits with code 1"`
	Cmd2        string `group:"MpvAction" help:"Command to run if mpv exits with code 2"`
	Cmd3        string `group:"MpvAction" help:"Command to run if mpv exits with code 3"`
	Cmd4        string `group:"MpvAction" help:"Command to run if mpv exits with code 4"`
	Cmd5        string `group:"MpvAction" help:"Command to run if mpv exits with code 5"`
	Cmd6        string `group:"MpvAction" help:"Command to run if mpv exits with code 6"`
	Cmd7        string `group:"MpvAction" help:"Command to run if mpv exits with code 7"`
	Cmd8        string `group:"MpvAction" help:"Command to run if mpv exits with code 8"`
	Cmd9        string `group:"MpvAction" help:"Command to run if mpv exits with code 9"`
	Cmd10       string `group:"MpvAction" help:"Command to run if mpv exits with code 10"`
	Cmd11       string `group:"MpvAction" help:"Command to run if mpv exits with code 11"`
	Cmd12       string `group:"MpvAction" help:"Command to run if mpv exits with code 12"`
	Cmd13       string `group:"MpvAction" help:"Command to run if mpv exits with code 13"`
	Cmd14       string `group:"MpvAction" help:"Command to run if mpv exits with code 14"`
	Cmd15       string `group:"MpvAction" help:"Command to run if mpv exits with code 15"`
	Cmd20       string `group:"MpvAction" help:"Command to run if mpv exits with code 20"`
	Cmd127      string `group:"MpvAction" help:"Command to run if mpv exits with code 127"`
	Interactive bool   `group:"MpvAction" help:"Interactive decision making after playback" short:"I"`
}

type PostActionFlags struct {
	Trash        bool   `group:"PostAction" help:"Trash files after action"`
	PostAction   string `group:"PostAction" help:"Post-action: none, delete, mark-deleted, move, copy"`
	DeleteFiles  bool   `group:"PostAction" help:"Delete files after action"`
	DeleteRows   bool   `group:"PostAction" help:"Delete rows from database"`
	MarkDeleted  bool   `group:"PostAction" help:"Mark as deleted in database"`
	MoveTo       string `group:"PostAction" help:"Move files to directory"`
	CopyTo       string `group:"PostAction" help:"Copy files to directory"`
	ActionLimit  int    `group:"PostAction" help:"Stop after N files"`
	ActionSize   string `group:"PostAction" help:"Stop after N bytes (e.g., 10GB)"`
	TrackHistory bool   `default:"true"     group:"PostAction"                                         help:"Track playback history"`
}

type HashingFlags struct {
	HashGap       float64 `default:"0.1"   group:"Hashing"                     help:"Gap between segments (0.0-1.0 as percentage of file size, or absolute bytes if >1)"`
	HashChunkSize int64   `group:"Hashing" help:"Size of each segment to hash"`
	HashThreads   int     `default:"1"     group:"Hashing"                     help:"Number of threads to use for hashing a single file"`
}

type MergeFlags struct {
	OnlyTables        []string `group:"Merge" help:"Comma separated specific table(s)"       short:"t"`
	PrimaryKeys       []string `group:"Merge" help:"Comma separated primary keys"`
	BusinessKeys      []string `group:"Merge" help:"Comma separated business keys"`
	Upsert            bool     `group:"Merge" help:"Upsert rows on conflict"`
	Ignore            bool     `group:"Merge" help:"Ignore rows on conflict (only-new-rows)"`
	OnlyNewRows       bool     `kong:"-"` // Alias for Ignore
	OnlyTargetColumns bool     `group:"Merge" help:"Only copy columns that exist in target"`
	SkipColumns       []string `group:"Merge" help:"Columns to skip during merge"`
}

// GlobalFlags are flags available to disco data commands (print, search, du, etc).
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

// ControlFlags are a subset of flags for simple control commands.
type ControlFlags struct {
	MpvSocket  string `group:"Playback"              help:"Mpv socket path"`
	CastDevice string `alias:"cast-to"               group:"Playback"       help:"Chromecast device name"`
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
