package main

import (
	"log/slog"
	"os"

	"github.com/alecthomas/kong"
	"github.com/chapmanjacobd/syncweb/internal/commands"
	"github.com/chapmanjacobd/syncweb/internal/models"
	"github.com/chapmanjacobd/syncweb/internal/utils"
)

func main() {
	cli := &commands.SyncwebCmd{}

	parser, err := kong.New(cli,
		kong.Name("syncweb"),
		kong.Description("Syncweb: an offline-first distributed web"),
		kong.UsageOnError(),
	)
	if err != nil {
		panic(err)
	}

	ctx, err := parser.Parse(os.Args[1:])
	if err != nil {
		parser.FatalIfErrorf(err)
	}

	// Configure logger
	models.SetupLogging(cli.Verbose)
	logger := slog.New(&utils.PlainHandler{
		Level: models.LogLevel,
		Out:   os.Stderr,
	})
	slog.SetDefault(logger)

	err = ctx.Run()
	if err != nil {
		slog.Error("Syncweb command failed", "error", err)
		os.Exit(1)
	}
}
