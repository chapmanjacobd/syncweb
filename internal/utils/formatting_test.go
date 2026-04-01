package utils

import (
	"strings"
	"testing"
	"time"
)

func TestFormatDuration(t *testing.T) {
	tests := []struct {
		seconds int
		want    string
	}{
		{0, "-"},
		{59, "0:59"},
		{60, "1:00"},
		{3600, "1:00:00"},
		{3661, "1:01:01"},
	}
	for _, tt := range tests {
		if got := FormatDuration(tt.seconds); got != tt.want {
			t.Errorf("FormatDuration(%d) = %q, want %q", tt.seconds, got, tt.want)
		}
	}
}

func TestFormatDurationShort(t *testing.T) {
	tests := []struct {
		seconds int
		want    string
	}{
		{0, "<1s"},
		{30, "30s"},
		{60, "1m"},
		{125, "2m5s"},
		{3600, "1h"},
		{7260, "2h1m"},
		{86400, "1d"},
		{176400, "2d1h"},
		{31536000, "1y"},
		{31536000 + 86400, "1y1d"},
	}
	for _, tt := range tests {
		if got := FormatDurationShort(tt.seconds); got != tt.want {
			t.Errorf("FormatDurationShort(%d) = %q, want %q", tt.seconds, got, tt.want)
		}
	}
}

func TestFormatTime(t *testing.T) {
	if got := FormatTime(0); got != "-" {
		t.Errorf("FormatTime(0) = %q, want %q", got, "-")
	}
	// 2024-01-01 12:00:00 UTC
	ts := int64(1704110400)
	got := FormatTime(ts)
	if !strings.Contains(got, "2024-01-01") {
		t.Errorf("FormatTime(%d) = %q, should contain 2024-01-01", ts, got)
	}
}

func TestSecondsToHHMMSS(t *testing.T) {
	tests := []struct {
		seconds int64
		want    string
	}{
		{0, "0:00"},
		{59, "0:59"},
		{60, "1:00"},
		{3600, "1:00:00"},
		{-60, "-1:00"},
	}
	for _, tt := range tests {
		if got := SecondsToHHMMSS(tt.seconds); got != tt.want {
			t.Errorf("SecondsToHHMMSS(%d) = %q, want %q", tt.seconds, got, tt.want)
		}
	}
}

func TestFormatPlaybackDuration(t *testing.T) {
	tests := []struct {
		duration, start, end int64
		want                 string
	}{
		{100, 0, 0, "Duration: 1:40"},
		{100, 10, 20, "Duration: 0:10 (0:10 to 0:20)"},
		{100, 10, 0, "Duration: 1:30 (0:10 to 1:40)"},
	}
	for _, tt := range tests {
		if got := FormatPlaybackDuration(tt.duration, tt.start, tt.end); got != tt.want {
			t.Errorf("FormatPlaybackDuration(%d, %d, %d) = %q, want %q", tt.duration, tt.start, tt.end, got, tt.want)
		}
	}
}

func TestRelativeDatetime(t *testing.T) {
	if got := RelativeDatetime(0); got != "-" {
		t.Errorf("RelativeDatetime(0) = %q, want %q", got, "-")
	}

	now := time.Now().Unix()
	if got := RelativeDatetime(now); !strings.Contains(got, "today") {
		t.Errorf("RelativeDatetime(now) = %q, should contain 'today'", got)
	}

	yesterday := time.Now().AddDate(0, 0, -1).Unix()
	if got := RelativeDatetime(yesterday); !strings.Contains(got, "yesterday") {
		t.Errorf("RelativeDatetime(yesterday) = %q, should contain 'yesterday'", got)
	}

	tomorrow := time.Now().AddDate(0, 0, 1).Unix()
	if got := RelativeDatetime(tomorrow); !strings.Contains(got, "tomorrow") {
		t.Errorf("RelativeDatetime(tomorrow) = %q, should contain 'tomorrow'", got)
	}
}
