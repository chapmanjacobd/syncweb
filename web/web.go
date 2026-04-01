// Package web provides the embedded static web assets.
package web

import (
	"embed"
	"fmt"
	"io/fs"
)

// fsRaw embeds the static web assets from the dist folder
//
//go:embed dist/*
var fsRaw embed.FS

// FS is the web asset file system with "dist" prefix removed.
var FS fs.FS

func init() {
	var err error
	FS, err = fs.Sub(fsRaw, "dist")
	if err != nil {
		panic(fmt.Sprintf("Failed to initialize embedded filesystem: %v. Ensure the 'dist' directory exists before building.", err))
	}
}
