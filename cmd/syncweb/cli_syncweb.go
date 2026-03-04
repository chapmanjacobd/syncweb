//go:build syncweb

package main

import "github.com/chapmanjacobd/discotheque/internal/commands"

type SyncwebCLI struct {
	Syncweb commands.SyncwebCmd `cmd:"" help:"Syncweb: an offline-first distributed web"`
}
