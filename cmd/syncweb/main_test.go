package main

import (
	"strings"
	"testing"

	"github.com/alecthomas/kong"
	"github.com/chapmanjacobd/syncweb/internal/commands"
)

func TestSyncwebCLI_Structure(t *testing.T) {
	cli := &commands.SyncwebCmd{}

	_, err := kong.New(cli,
		kong.Name("syncweb"),
		kong.Description("Syncweb: an offline-first distributed web"),
		kong.UsageOnError(),
	)
	if err != nil {
		t.Fatalf("Failed to create kong parser: %v", err)
	}
}

func TestSyncwebCLI_Subcommands(t *testing.T) {
	cli := &commands.SyncwebCmd{}

	parser, err := kong.New(cli,
		kong.Name("syncweb"),
		kong.Description("Syncweb: an offline-first distributed web"),
		kong.UsageOnError(),
	)
	if err != nil {
		t.Fatalf("Failed to create kong parser: %v", err)
	}

	tests := []struct {
		args []string
		cmd  string
	}{
		{[]string{"ls"}, "ls"},
		{[]string{"list"}, "ls"},
		{[]string{"find", "test"}, "find"},
		{[]string{"fd", "test"}, "find"},
		{[]string{"stat", "test"}, "stat"},
		{[]string{"folders"}, "folders"},
		{[]string{"lsf"}, "folders"},
		{[]string{"devices"}, "devices"},
		{[]string{"lsd"}, "devices"},
		{[]string{"download", "test"}, "download"},
		{[]string{"dl", "test"}, "download"},
		{[]string{"automatic"}, "automatic"},
		{[]string{"version"}, "version"},
	}

	for _, tt := range tests {
		ctx, err := parser.Parse(tt.args)
		if err != nil {
			t.Errorf("Failed to parse args %v: %v", tt.args, err)
			continue
		}
		if !strings.HasPrefix(ctx.Command(), tt.cmd) {
			t.Errorf("Expected command %s for args %v, got %s", tt.cmd, tt.args, ctx.Command())
		}
	}
}
