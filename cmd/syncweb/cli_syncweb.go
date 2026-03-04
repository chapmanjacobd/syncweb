package main

import "github.com/chapmanjacobd/syncweb/internal/commands"

type SyncwebCLI struct {
	Syncweb commands.SyncwebCmd `cmd:"" help:"Syncweb: an offline-first distributed web"`
}
