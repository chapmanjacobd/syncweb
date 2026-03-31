package commands

import (
	"testing"
)

func TestGlobToRegex(t *testing.T) {
	tests := []struct {
		name     string
		glob     string
		expected string
	}{
		{"simple", "hello", "^hello$"},
		{"star wildcard", "hel*", "^hel.*$"},
		{"question mark", "hel?", "^hel.$"},
		{"star and question", "h*l?o", "^h.*l.o$"},
		{"with dots", "*.txt", "^.*\\.txt$"},
		{"with special chars", "file[1].txt", "^file\\[1\\]\\.txt$"},
		{"with parens", "file(1).txt", "^file\\(1\\)\\.txt$"},
		{"with plus", "file+1.txt", "^file\\+1\\.txt$"},
		{"with caret", "file^1.txt", "^file\\^1\\.txt$"},
		{"with dollar", "file$1.txt", "^file\\$1\\.txt$"},
		{"with pipes", "file|1.txt", "^file\\|1\\.txt$"},
		{"with braces", "file{1}.txt", "^file\\{1\\}\\.txt$"},
		{"with backslash", "file\\1.txt", "^file\\\\1\\.txt$"},
		{"complex", "*.mp[34]", "^.*\\.mp\\[34\\]$"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := globToRegex(tt.glob)
			if result != tt.expected {
				t.Errorf("globToRegex(%q) = %q, expected %q", tt.glob, result, tt.expected)
			}
		})
	}
}
