package models_test

import (
	"log/slog"
	"testing"

	"github.com/chapmanjacobd/syncweb/internal/models"
)

func TestCoreFlags_AfterApply(t *testing.T) {
	c := &models.CoreFlags{Simulate: true, NoConfirm: true}
	if err := c.AfterApply(); err != nil {
		t.Errorf("AfterApply() error = %v", err)
	}
	if !c.DryRun {
		t.Errorf("DryRun should be true")
	}
	if !c.Yes {
		t.Errorf("Yes should be true")
	}
}

func TestSetupLogging(t *testing.T) {
	models.SetupLogging(true)
	if models.LogLevel.Level() != slog.LevelDebug {
		t.Errorf("LogLevel should be Debug")
	}
	models.SetupLogging(false)
	if models.LogLevel.Level() != slog.LevelInfo {
		t.Errorf("LogLevel should be Info")
	}
}
