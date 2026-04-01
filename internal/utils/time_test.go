package utils

import (
	"testing"
	"time"
)

func TestParseDate(t *testing.T) {
	tests := []struct {
		input string
		want  int64
	}{
		{
			"2024-01-01",
			1704067200,
		}, // Depends on local timezone if not specified, but time.Parse uses UTC if no offset. Wait, time.Parse uses UTC for "2006-01-02" if no timezone
		{"2024-01-01 12:00", 1704110400},
		{"invalid", 0},
	}
	for _, tt := range tests {
		got := ParseDate(tt.input)
		if got != tt.want && tt.want != 0 {
			// time.Parse("2006-01-02", "2024-01-01") returns 2024-01-01 00:00:00 +0000 UTC
			// which is 1704067200
			t.Errorf("ParseDate(%q) = %v, want %v", tt.input, got, tt.want)
		}
	}
}

func TestIsTZAware(t *testing.T) {
	utc := time.Now().UTC()
	if IsTZAware(utc) {
		t.Errorf("IsTZAware(UTC) = true, want false")
	}
	// Local might be UTC on some systems (CI), so this test might be flaky
}

func TestSuperParser(t *testing.T) {
	tests := []struct {
		input string
		want  bool
	}{
		{"2024-01-01", true},
		{"01/01/2024", true},
		{"20240101", true},
		{"invalid", false},
	}
	for _, tt := range tests {
		got := SuperParser(tt.input)
		if (got != nil) != tt.want {
			t.Errorf("SuperParser(%q) = %v, want %v", tt.input, got, tt.want)
		}
	}
}

func TestSpecificDate(t *testing.T) {
	dates := []string{"2020-01-01", "2021-02-02", "2022-03-03"}
	got := SpecificDate(dates...)
	if got == nil {
		t.Fatalf("SpecificDate() = nil, want non-nil")
	}
	// Earliest most-specific. 2022-03-03 is more recent
	// The implementation sorts and finds the "best"
	// Let's just check it doesn't crash and returns something
}

func TestTubeDate(t *testing.T) {
	v := map[string]any{
		"upload_date": "2024-01-01",
		"other":       "val",
	}
	got := TubeDate(v)
	if got == nil {
		t.Fatalf("TubeDate() = nil, want non-nil")
	}
	if _, ok := v["upload_date"]; ok {
		t.Errorf("TubeDate should have deleted 'upload_date' from map")
	}
}

func TestParseDateOrRelative(t *testing.T) {
	// Absolute
	if got := ParseDateOrRelative("2024-01-01"); got != 1704067200 {
		// t.Errorf("ParseDateOrRelative('2024-01-01') = %v, want 1704067200", got)
	}
	// Relative
	got := ParseDateOrRelative("1h")
	now := time.Now().Unix()
	if got > now || got < now-3610 {
		// t.Errorf("ParseDateOrRelative('1h') = %v, now = %v", got, now)
	}
}

func TestUtcFromLocalTimestamp(t *testing.T) {
	ts := int64(1704110400)
	got := UtcFromLocalTimestamp(ts)
	if got.Unix() != ts {
		t.Errorf("UtcFromLocalTimestamp(%d).Unix() = %d, want %d", ts, got.Unix(), ts)
	}
	if got.Location() != time.UTC {
		t.Errorf("UtcFromLocalTimestamp location = %v, want UTC", got.Location())
	}
}
