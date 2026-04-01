package commands

import (
	"bytes"
	"io"
	"os"
	"testing"
)

func TestSyncwebCmd_AfterApply(t *testing.T) {
	c := &SyncwebCmd{}
	if err := c.AfterApply(); err != nil {
		t.Errorf("AfterApply() error = %v", err)
	}
	if c.SyncwebHome == "" {
		t.Errorf("SyncwebHome should not be empty")
	}
}

func TestMatchesFilters(t *testing.T) {
	tests := []struct {
		s       string
		include []string
		exclude []string
		want    bool
	}{
		{"test", nil, nil, true},
		{"test", []string{"es"}, nil, true},
		{"test", []string{"abc"}, nil, false},
		{"test", nil, []string{"es"}, false},
		{"test", nil, []string{"abc"}, true},
		{"test", []string{"es"}, []string{"t"}, false},
	}
	for _, tt := range tests {
		if got := matchesFilters(tt.s, tt.include, tt.exclude); got != tt.want {
			t.Errorf("matchesFilters(%q, %v, %v) = %v, want %v", tt.s, tt.include, tt.exclude, got, tt.want)
		}
	}
}

func TestSyncwebVersionCmd_Run(t *testing.T) {
	c := &SyncwebVersionCmd{}
	g := &SyncwebCmd{}

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
