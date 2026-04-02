package models

import "log/slog"

// CoreFlags are essential flags shared across most binaries/commands
type CoreFlags struct {
	Verbose   bool `help:"Enable verbose logging"              short:"v"`
	JSON      bool `help:"Output results as JSON"              short:"j"`
	Simulate  bool `help:"Dry run; don't actually do anything"           aliases:"dry-run"`
	DryRun    bool `                                                                       kong:"-"` // Alias for Simulate
	NoConfirm bool `help:"Don't ask for confirmation"          short:"y" aliases:"yes"`
	Yes       bool `                                                                       kong:"-"` // Alias for NoConfirm
}

// SyncwebFlags are flags related to Syncweb configuration
type SyncwebFlags struct {
	SyncwebHome string `help:"Syncweb home directory" aliases:"home" env:"SYNCWEB_HOME"`
}

// PathFilterFlags are flags for filtering paths
type PathFilterFlags struct {
	Include []string `help:"Include paths matching pattern" short:"s"`
	Exclude []string `help:"Exclude paths matching pattern" short:"E"`
	Paths   []string `help:"Exact paths to include"`
}

// FilterFlags are flags for filtering results
type FilterFlags struct {
	Size  []string `help:"Size range (e.g., >100MB, 1GB%10)" short:"S"`
	Exact bool     `help:"Exact match for search"`
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

var LogLevel = &slog.LevelVar{}

func SetupLogging(verbose bool) {
	if verbose {
		LogLevel.Set(slog.LevelDebug)
	} else {
		LogLevel.Set(slog.LevelInfo)
	}
}
