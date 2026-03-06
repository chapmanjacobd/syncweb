package version

// Build-time variables set via -ldflags
var (
	// Version is the git tag or commit hash
	Version = "dev"
	// BuildTime is the build timestamp
	BuildTime = "unknown"
	// GitHash is the git commit hash
	GitHash = "unknown"
	// GitDirty is set to "-dirty" if the working tree had uncommitted changes
	GitDirty = ""
)

// Info returns the full version string
func Info() string {
	v := "syncweb " + Version
	if GitDirty != "" {
		v += GitDirty
	}
	if BuildTime != "unknown" {
		v += " (" + BuildTime + ")"
	}
	return v
}

// FullInfo returns detailed version information
func FullInfo() string {
	v := "syncweb " + Version
	if GitDirty != "" {
		v += GitDirty
	}
	if GitHash != "unknown" && GitHash != "" {
		v += "\ncommit:   " + GitHash
	}
	if BuildTime != "unknown" && BuildTime != "" {
		v += "\nbuilt:    " + BuildTime
	}
	return v
}
