package commands_test

import (
	"bytes"
	"io"
	"os"
	"testing"

	"github.com/chapmanjacobd/syncweb/internal/commands"
)

func TestSyncwebCmd_AfterApply(t *testing.T) {
	c := &commands.SyncwebCmd{}
	if err := c.AfterApply(); err != nil {
		t.Errorf("AfterApply() error = %v", err)
	}
	if c.SyncwebHome == "" {
		t.Errorf("SyncwebHome should not be empty")
	}
}

func TestSyncwebVersionCmd_Run(t *testing.T) {
	c := &commands.SyncwebVersionCmd{}
	g := &commands.SyncwebCmd{}

	// Capture stdout
	old := os.Stdout
	r, w, _ := os.Pipe()
	os.Stdout = w

	err := c.Run(g)

	w.Close()
	os.Stdout = old

	if err != nil {
		t.Errorf("Run() error = %v", err)
	}

	var buf bytes.Buffer
	io.Copy(&buf, r)
	if buf.Len() == 0 {
		t.Errorf("Version info should not be empty")
	}
}
