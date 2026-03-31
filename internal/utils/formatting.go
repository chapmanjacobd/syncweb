package utils

import (
	"fmt"
	"strings"
	"time"
)

func FormatDuration(seconds int) string {
	if seconds == 0 {
		return "-"
	}
	h := seconds / 3600
	m := (seconds % 3600) / 60
	s := seconds % 60
	if h > 0 {
		return fmt.Sprintf("%d:%02d:%02d", h, m, s)
	}
	return fmt.Sprintf("%d:%02d", m, s)
}

func FormatDurationShort(seconds int) string {
	if seconds <= 0 {
		return "<1s"
	}

	const (
		Minute = 60
		Hour   = 3600
		Day    = 86400
		Year   = 31536000
	)

	if seconds < Minute {
		return fmt.Sprintf("%ds", seconds)
	}

	if seconds < Hour {
		m := seconds / Minute
		s := seconds % Minute
		if s == 0 {
			return fmt.Sprintf("%dm", m)
		}
		return fmt.Sprintf("%dm%ds", m, s)
	}

	if seconds < Day {
		h := seconds / Hour
		m := (seconds % Hour) / Minute
		if m == 0 {
			return fmt.Sprintf("%dh", h)
		}
		return fmt.Sprintf("%dh%dm", h, m)
	}

	if seconds < Year {
		d := seconds / Day
		h := (seconds % Day) / Hour
		if h == 0 {
			return fmt.Sprintf("%dd", d)
		}
		return fmt.Sprintf("%dd%dh", d, h)
	}

	y := seconds / Year
	d := (seconds % Year) / Day
	if d == 0 {
		return fmt.Sprintf("%dy", y)
	}
	return fmt.Sprintf("%dy%dd", y, d)
}

func FormatSize(bytes int64) string {
	if bytes == 0 {
		return "-"
	}
	const unit = 1024
	if bytes < unit {
		return fmt.Sprintf("%d B", bytes)
	}
	div, exp := int64(unit), 0
	for n := bytes / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f %cB", float64(bytes)/float64(div), "KMGTPE"[exp])
}

func FormatTime(timestamp int64) string {
	if timestamp == 0 {
		return "-"
	}
	t := time.Unix(timestamp, 0)
	return t.Format("2006-01-02 15:04")
}

func RelativeDatetime(timestamp int64) string {
	if timestamp == 0 {
		return "-"
	}
	t := time.Unix(timestamp, 0)
	now := time.Now()
	diff := now.Sub(t)

	// Past
	if diff > 0 {
		if diff.Hours() < 24 && t.Day() == now.Day() {
			return t.Format("today, 15:04")
		}
		if diff.Hours() < 48 && t.Day() == now.AddDate(0, 0, -1).Day() {
			return t.Format("yesterday, 15:04")
		}
		if diff.Hours() < 24*45 {
			days := int(diff.Hours() / 24)
			if days == 0 {
				days = 1
			}
			return fmt.Sprintf("%d days ago, %s", days, t.Format("15:04"))
		}
	} else {
		// Future
		absDiff := -diff
		if absDiff.Hours() < 24 && t.Day() == now.Day() {
			return t.Format("today, 15:04")
		}
		if absDiff.Hours() < 48 && t.Day() == now.AddDate(0, 0, 1).Day() {
			return t.Format("tomorrow, 15:04")
		}
		if absDiff.Hours() < 24*45 {
			days := int(absDiff.Hours() / 24)
			return fmt.Sprintf("in %d days, %s", days, t.Format("15:04"))
		}
	}

	return t.Format("2006-01-02 15:04")
}

func SecondsToHHMMSS(seconds int64) string {
	neg := false
	if seconds < 0 {
		neg = true
		seconds = -seconds
	}

	h := seconds / 3600
	m := (seconds % 3600) / 60
	s := seconds % 60

	prefix := ""
	if neg {
		prefix = "-"
	}

	if h > 0 {
		return fmt.Sprintf("%s%d:%02d:%02d", prefix, h, m, s)
	}
	return fmt.Sprintf("%s%d:%02d", prefix, m, s)
}

func FormatPlaybackDuration(duration, segmentStart, segmentEnd int64) string {
	if segmentStart > duration && segmentEnd == 0 {
		segmentEnd = segmentStart + duration
	}

	if segmentStart > segmentEnd && segmentEnd > 0 && segmentStart+segmentEnd > duration {
		segmentStart, segmentEnd = segmentEnd, segmentStart
	}

	if segmentStart != 0 || segmentEnd != 0 {
		var segmentDuration int64
		if segmentEnd > 0 {
			segmentDuration = segmentStart - segmentEnd
		} else {
			segmentDuration = duration - segmentStart
		}

		if segmentDuration < 0 {
			segmentDuration = -segmentDuration
		}

		startStr := strings.TrimSpace(SecondsToHHMMSS(segmentStart))
		endVal := segmentEnd
		if endVal == 0 {
			endVal = duration
		}
		endStr := strings.TrimSpace(SecondsToHHMMSS(endVal))
		durationStr := strings.TrimSpace(SecondsToHHMMSS(segmentDuration))

		return fmt.Sprintf("Duration: %s (%s to %s)", durationStr, startStr, endStr)
	}

	durationStr := strings.TrimSpace(SecondsToHHMMSS(duration))
	return fmt.Sprintf("Duration: %s", durationStr)
}
