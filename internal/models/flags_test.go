package models

import (
	"log/slog"
	"testing"
)

func TestCoreFlags_AfterApply(t *testing.T) {
	c := &CoreFlags{Simulate: true, NoConfirm: true}
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

func TestMediaFilterFlags_AfterApply(t *testing.T) {
	m := &MediaFilterFlags{Ext: []string{"mp4", ".mkv"}}
	if err := m.AfterApply(); err != nil {
		t.Errorf("AfterApply() error = %v", err)
	}
	if m.Ext[0] != ".mp4" {
		t.Errorf("Ext[0] should be .mp4, got %s", m.Ext[0])
	}
	if m.Ext[1] != ".mkv" {
		t.Errorf("Ext[1] should be .mkv, got %s", m.Ext[1])
	}
}

func TestMergeFlags_AfterApply(t *testing.T) {
	m := &MergeFlags{Ignore: true}
	if err := m.AfterApply(); err != nil {
		t.Errorf("AfterApply() error = %v", err)
	}
	if !m.OnlyNewRows {
		t.Errorf("OnlyNewRows should be true")
	}
}

func TestSetupLogging(t *testing.T) {
	SetupLogging(true)
	if LogLevel.Level() != slog.LevelDebug {
		t.Errorf("LogLevel should be Debug")
	}
	SetupLogging(false)
	if LogLevel.Level() != slog.LevelInfo {
		t.Errorf("LogLevel should be Info")
	}
}

func TestGlobalFlags_AfterApply(t *testing.T) {
	g := &GlobalFlags{
		CoreFlags:        CoreFlags{Simulate: true},
		MediaFilterFlags: MediaFilterFlags{Ext: []string{"mp4"}},
		MergeFlags:       MergeFlags{Ignore: true},
	}
	if err := g.AfterApply(); err != nil {
		t.Errorf("AfterApply() error = %v", err)
	}
	if !g.DryRun {
		t.Errorf("DryRun should be true")
	}
	if g.Ext[0] != ".mp4" {
		t.Errorf("Ext[0] should be .mp4")
	}
	if !g.OnlyNewRows {
		t.Errorf("OnlyNewRows should be true")
	}
}
