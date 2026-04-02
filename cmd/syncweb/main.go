package main

import (
	"context"
	"fmt"
	"log/slog"
	"os"
	"os/signal"
	"syscall"

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
		fmt.Fprintf(os.Stderr, "Failed to initialize CLI parser: %v\n", err)
		os.Exit(1)
	}

	parserCtx, err := parser.Parse(os.Args[1:])
	if err != nil {
		parser.FatalIfErrorf(err)
	}

	// Set up signal handling
	sigCtx, cancel := context.WithCancel(context.Background())
	defer cancel()

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigChan
		cancel()
	}()

	cli.Ctx = sigCtx

	// Configure logger
	models.SetupLogging(cli.Verbose)
	logger := slog.New(&utils.PlainHandler{
		Level: models.LogLevel,
		Out:   os.Stderr,
	})
	slog.SetDefault(logger)

	err = parserCtx.Run()
	if err != nil {
		logger.Error("Syncweb command failed", "error", err)
		cancel()
		os.Exit(1) //nolint:gocritic // Need to exit after logging error
	}
}
