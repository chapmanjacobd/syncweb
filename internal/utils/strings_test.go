package utils

import (
	"testing"
)

func TestCompareBlockStrings(t *testing.T) {
	tests := []struct {
		name     string
		pattern  string
		value    string
		expected bool
	}{
		{"exact match", "hello", "hello", true},
		{"exact match case insensitive", "Hello", "hello", true},
		{"no match", "hello", "world", false},
		{"prefix match", "hel%", "hello", true},
		{"prefix no match", "hel%", "world", false},
		{"suffix match", "%llo", "hello", true},
		{"suffix no match", "%llo", "world", false},
		{"contains match", "%ell%", "hello", true},
		{"contains no match", "%ell%", "world", false},
		{"wildcard only", "%", "anything", true},
		{"empty pattern", "", "", true},
		{"complex pattern", "hel%o", "hello", true},
		{"complex pattern no match", "hel%o", "help", false},
		{"multiple wildcards", "%he%ll%", "hello", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := CompareBlockStrings(tt.pattern, tt.value)
			if result != tt.expected {
				t.Errorf("CompareBlockStrings(%q, %q) = %v, expected %v", tt.pattern, tt.value, result, tt.expected)
			}
		})
	}
}

func TestMatchesAny(t *testing.T) {
	patterns := []string{"*.txt", "test%", "%file%"}

	tests := []struct {
		name     string
		path     string
		expected bool
	}{
		{"glob match", "document.txt", true},
		{"block match prefix", "test123", true},
		{"block match contains", "myfile.txt", true},
		{"no match", "image.png", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := MatchesAny(tt.path, patterns)
			if result != tt.expected {
				t.Errorf("MatchesAny(%q, %v) = %v, expected %v", tt.path, patterns, result, tt.expected)
			}
		})
	}
}

func TestCleanString(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"remove special chars", "hello&world", "helloworld"},
		{"remove brackets", "hello (world)", "hello"},
		{"remove html entities", "hello &amp; world", "hello world"},
		{"clean dashes", "hello - world", "hello world"},
		{"clean underscores", "hello _ world", "hello_world"},
		{"remove backslashes", "hello\\world", "hello world"},
		{"remove slashes", "hello/world", "hello world"},
		{"remove consecutive dots", "hello..world", "hello.world"},
		{"complex cleanup", "Hello (World) & Test", "Hello Test"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := CleanString(tt.input)
			if result != tt.expected {
				t.Errorf("CleanString(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestRemoveTextInsideBrackets(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"parentheses", "hello (world) test", "hello  test"},
		{"square brackets", "hello [world] test", "hello  test"},
		{"curly braces", "hello {world} test", "hello  test"},
		{"nested brackets", "hello (world (nested)) test", "hello  test"},
		{"no brackets", "hello world", "hello world"},
		{"empty brackets", "hello () test", "hello  test"},
		{"multiple brackets", "hello (a) test (b)", "hello  test "},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := RemoveTextInsideBrackets(tt.input)
			if result != tt.expected {
				t.Errorf("RemoveTextInsideBrackets(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestPathToSentence(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"simple path", "/home/user/file.txt", "file txt"},
		{"path with dashes", "/home/user/my-file.txt", "my file txt"},
		{"path with underscores", "/home/user/my_file.txt", "my file txt"},
		{"filename only", "file.txt", "file txt"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := PathToSentence(tt.input)
			if result != tt.expected {
				t.Errorf("PathToSentence(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestIsGenericTitle(t *testing.T) {
	tests := []struct {
		name     string
		title    string
		expected bool
	}{
		{"empty string", "", true},
		{"chapter short", "chapter 1", true},
		{"chapter long", "chapter 12345", false},
		{"scene", "scene 5", true},
		{"untitled", "untitled chapter", true},
		{"timecode", "01:23:45", true},
		{"digit", "123", true},
		{"normal title", "My Movie", false},
		{"whitespace", "  ", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := IsGenericTitle(tt.title)
			if result != tt.expected {
				t.Errorf("IsGenericTitle(%q) = %v, expected %v", tt.title, result, tt.expected)
			}
		})
	}
}

func TestIsDigit(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected bool
	}{
		{"digits only", "123", true},
		{"single digit", "5", true},
		{"zero", "0", true},
		{"empty string", "", false},
		{"with letters", "123a", false},
		{"with spaces", "123 ", false},
		{"negative", "-123", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := IsDigit(tt.input)
			if result != tt.expected {
				t.Errorf("IsDigit(%q) = %v, expected %v", tt.input, result, tt.expected)
			}
		})
	}
}

func TestIsTimecodeLike(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected bool
	}{
		{"standard timecode", "01:23:45", true},
		{"with milliseconds", "01:23:45.123", true},
		{"simple numbers", "12345", true},
		{"with dashes", "01-23-45", true},
		{"with dots", "01.23.45", true},
		{"empty string", "", false},
		{"with letters", "01:23:ab", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := IsTimecodeLike(tt.input)
			if result != tt.expected {
				t.Errorf("IsTimecodeLike(%q) = %v, expected %v", tt.input, result, tt.expected)
			}
		})
	}
}

func TestRemoveConsecutiveWhitespace(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"single spaces", "hello world", "hello world"},
		{"multiple spaces", "hello  world", "hello world"},
		{"tabs and spaces", "hello\t\tworld", "hello world"},
		{"leading/trailing", "  hello world  ", "hello world"},
		{"newlines", "hello\n\nworld", "hello world"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := RemoveConsecutiveWhitespace(tt.input)
			if result != tt.expected {
				t.Errorf("RemoveConsecutiveWhitespace(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestRemoveConsecutive(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		char     string
		expected string
	}{
		{"remove dots", "hello...world", ".", "hello.world"},
		{"remove dashes", "hello--world", "-", "hello-world"},
		{"no consecutive", "hello.world", ".", "hello.world"},
		{"multiple groups", "hello...world...test", ".", "hello.world.test"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := RemoveConsecutive(tt.input, tt.char)
			if result != tt.expected {
				t.Errorf("RemoveConsecutive(%q, %q) = %q, expected %q", tt.input, tt.char, result, tt.expected)
			}
		})
	}
}

func TestRemoveConsecutives(t *testing.T) {
	input := "hello...world---test"
	chars := []string{".", "-"}
	expected := "hello.world-test"

	result := RemoveConsecutives(input, chars)
	if result != expected {
		t.Errorf("RemoveConsecutives(%q, %v) = %q, expected %q", input, chars, result, expected)
	}
}

func TestRemovePrefixes(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		prefixes []string
		expected string
	}{
		{"single prefix", "hello world", []string{"hello "}, "world"},
		{"multiple prefixes", "the hello world", []string{"the ", "hello "}, "world"},
		{
			"repeated prefix",
			"hello hello world",
			[]string{"hello"},
			" hello world",
		}, //nolint:dupword // intentional test case for repeated words
		{"no prefix", "hello world", []string{"goodbye "}, "hello world"},
		{"multiple prefix types", "the a hello", []string{"the ", "a "}, "hello"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := RemovePrefixes(tt.input, tt.prefixes)
			if result != tt.expected {
				t.Errorf("RemovePrefixes(%q, %v) = %q, expected %q", tt.input, tt.prefixes, result, tt.expected)
			}
		})
	}
}

func TestRemoveSuffixes(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		suffixes []string
		expected string
	}{
		{"single suffix", "hello world", []string{" world"}, "hello"},
		{"multiple suffixes", "hello world test", []string{" world", " test"}, "hello"},
		{
			"repeated suffix",
			"hello world world",
			[]string{" world"},
			"hello world",
		}, //nolint:dupword // intentional test case for repeated words
		{"no suffix", "hello world", []string{" goodbye"}, "hello world"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := RemoveSuffixes(tt.input, tt.suffixes)
			if result != tt.expected {
				t.Errorf("RemoveSuffixes(%q, %v) = %q, expected %q", tt.input, tt.suffixes, result, tt.expected)
			}
		})
	}
}

func TestShorten(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		maxWidth int
		expected string
	}{
		{"shorter than max", "hello", 10, "hello"},
		{"exactly max", "hello", 5, "hello"},
		{"needs shortening", "hello world", 8, "hello w…"},
		{"very small max", "hello world", 3, "he…"},
		{"with emoji", "hello 🌍", 8, "hello 🌍"},
		{"trims trailing space", "hello ", 5, "hell…"},
		{"trims trailing dash", "hello-", 5, "hell…"},
		{"trims trailing dot", "hello.", 5, "hell…"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := Shorten(tt.input, tt.maxWidth)
			if result != tt.expected {
				t.Errorf("Shorten(%q, %d) = %q, expected %q", tt.input, tt.maxWidth, result, tt.expected)
			}
		})
	}
}

func TestShortenMiddle(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		maxWidth int
		expected string
	}{
		{"shorter than max", "hello", 10, "hello"},
		{"exactly max", "hello", 5, "hello"},
		{"needs shortening", "hello world", 9, "hel...rld"},
		{"very small max", "hello world", 3, "..."},
		{"long string", "this is a very long string", 15, "this i...string"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := ShortenMiddle(tt.input, tt.maxWidth)
			if result != tt.expected {
				t.Errorf("ShortenMiddle(%q, %d) = %q, expected %q", tt.input, tt.maxWidth, result, tt.expected)
			}
		})
	}
}

func TestNaturalLess(t *testing.T) {
	tests := []struct {
		name     string
		s1       string
		s2       string
		expected bool
	}{
		{"numeric comparison", "file2", "file10", true},
		{"same numbers", "file10", "file10", false},
		{"reverse numeric", "file10", "file2", false},
		{"alpha comparison", "apple", "banana", true},
		{"mixed", "a2b", "a10b", true},
		{"different lengths", "file1", "file1a", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := NaturalLess(tt.s1, tt.s2)
			if result != tt.expected {
				t.Errorf("NaturalLess(%q, %q) = %v, expected %v", tt.s1, tt.s2, result, tt.expected)
			}
		})
	}
}

func TestUnParagraph(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"smart quotes", "Hello 'world'", "Hello 'world'"},
		{"smart double quotes", "Hello \"world\"", "Hello \"world\""},
		{"ellipsis", "Hello...world", "Hello...world"},
		{"multiple spaces", "Hello  world", "Hello world"},
		{"strip quotes", "\"hello\"", "hello"},
		{"complex", "  Hello  \"world\"...  ", "Hello \"world\"..."},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := UnParagraph(tt.input)
			if result != tt.expected {
				t.Errorf("UnParagraph(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestIsMimeMatch(t *testing.T) {
	tests := []struct {
		name     string
		terms    []string
		mimeType string
		expected bool
	}{
		{"exact match", []string{"video"}, "video/mp4", true},
		{"case insensitive", []string{"VIDEO"}, "video/mp4", true},
		{"no match", []string{"audio"}, "video/mp4", false},
		{"empty mime", []string{"video"}, "", false},
		{"multiple terms", []string{"audio", "video"}, "video/mp4", true},
		{"substring match", []string{"mp4"}, "video/mp4", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := IsMimeMatch(tt.terms, tt.mimeType)
			if result != tt.expected {
				t.Errorf("IsMimeMatch(%v, %q) = %v, expected %v", tt.terms, tt.mimeType, result, tt.expected)
			}
		})
	}
}

func TestTitle(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"simple", "hello world", "Hello World"},
		{"empty", "", ""},
		{"single word", "hello", "Hello"},
		{"already titled", "Hello World", "Hello World"},
		{"mixed case", "hElLo wOrLd", "HElLo WOrLd"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := Title(tt.input)
			if result != tt.expected {
				t.Errorf("Title(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestStripEnclosingQuotes(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"double quotes", "\"hello\"", "hello"},
		{"single quotes", "'hello'", "hello"},
		{"no quotes", "hello", "hello"},
		{"nested quotes", "\"'hello'\"", "hello"},
		{"smart quotes", "\u201chello\u201d", "hello"},
		{"guillemets", "«hello»", "hello"},
		{"partial quotes", "\"hello", "\"hello"},
		{"empty", "", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := StripEnclosingQuotes(tt.input)
			if result != tt.expected {
				t.Errorf("StripEnclosingQuotes(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestSplitAndTrim(t *testing.T) {
	input := "hello,  world , test"
	expected := []string{"hello", "world", "test"}

	result := SplitAndTrim(input, ",")
	if len(result) != len(expected) {
		t.Errorf("SplitAndTrim returned %d elements, expected %d", len(result), len(expected))
	}
	for i, v := range result {
		if v != expected[i] {
			t.Errorf("Element %d: got %q, expected %q", i, v, expected[i])
		}
	}
}

func TestCombine(t *testing.T) {
	tests := []struct {
		name     string
		inputs   []any
		expected string
	}{
		{"strings", []any{"hello", "world"}, "hello;world"},
		{"with unknown", []any{"hello", "unknown", "world"}, "hello;world"},
		{"with none", []any{"hello", "none", "world"}, "hello;world"},
		{"with und", []any{"hello", "und", "world"}, "hello;world"},
		{"string slice", []any{[]string{"a", "b"}}, "a;b"},
		{"duplicates", []any{"hello", "hello"}, "hello"},
		{"case insensitive dedup", []any{"Hello", "hello"}, "Hello"},
		{"empty result", []any{"unknown", "none"}, ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := Combine(tt.inputs...)
			if result != tt.expected {
				t.Errorf("Combine(%v) = %q, expected %q", tt.inputs, result, tt.expected)
			}
		})
	}
}

func TestFromTimestampSeconds(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected float64
	}{
		{"seconds only", "30", 30},
		{"minutes:seconds", "1:30", 90},
		{"hours:minutes:seconds", "1:30:00", 5400},
		{"with decimal", "1:30.5", 90.5},
		{"empty", "", 0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := FromTimestampSeconds(tt.input)
			if result != tt.expected {
				t.Errorf("FromTimestampSeconds(%q) = %f, expected %f", tt.input, result, tt.expected)
			}
		})
	}
}

func TestPartialStartswith(t *testing.T) {
	list := []string{"hello", "world", "test"}

	tests := []struct {
		name        string
		input       string
		expectedVal string
		expectErr   bool
	}{
		{"exact match", "hello", "hello", false},
		{"prefix match", "hel", "hello", false},
		{"no match", "xyz", "", true},
		{"empty input", "", "", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := PartialStartswith(tt.input, list)
			if tt.expectErr {
				if err == nil {
					t.Error("Expected error, got nil")
				}
			} else {
				if err != nil {
					t.Errorf("Unexpected error: %v", err)
				}
				if result != tt.expectedVal {
					t.Errorf("Expected %q, got %q", tt.expectedVal, result)
				}
			}
		})
	}
}

func TestGlobMatchAny(t *testing.T) {
	patterns := []string{"*.txt", "*.md", "test*"}

	tests := []struct {
		name     string
		path     string
		expected bool
	}{
		{"match txt", "file.txt", true},
		{"match md", "readme.md", true},
		{"match prefix", "test123", true},
		{"no match", "image.png", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := GlobMatchAny(tt.path, patterns)
			if result != tt.expected {
				t.Errorf("GlobMatchAny(%q, %v) = %v, expected %v", tt.path, patterns, result, tt.expected)
			}
		})
	}
}

func TestGlobMatchAll(t *testing.T) {
	patterns := []string{"*.txt", "test*"}

	tests := []struct {
		name     string
		path     string
		expected bool
	}{
		{"matches all", "test.txt", true},
		{"matches one", "file.txt", false},
		{"matches none", "image.png", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := GlobMatchAll(tt.path, patterns)
			if result != tt.expected {
				t.Errorf("GlobMatchAll(%q, %v) = %v, expected %v", tt.path, patterns, result, tt.expected)
			}
		})
	}
}

func TestRemoveExcessiveLinebreaks(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"normal text", "hello\nworld", "hello\nworld"},
		{"double newline", "hello\n\nworld", "hello\n\nworld"},
		{"triple newline", "hello\n\n\nworld", "hello\n\nworld"},
		{"many newlines", "hello\n\n\n\n\nworld", "hello\n\nworld"},
		{"with spaces", "hello\n  \n  \nworld", "hello\n\nworld"},
		{"windows line endings", "hello\r\n\r\n\r\nworld", "hello\n\nworld"},
		{"trimmed", "\n\nhello\n\n", "hello"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := RemoveExcessiveLinebreaks(tt.input)
			if result != tt.expected {
				t.Errorf("RemoveExcessiveLinebreaks(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestLastChars(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"with extension", "file.txt", "txt"},
		{"multiple dots", "file.tar.gz", "gz"},
		{"no dots", "file", "file"},
		{"empty", "", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := LastChars(tt.input)
			if result != tt.expected {
				t.Errorf("LastChars(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestExtractWords(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected []string
	}{
		{"simple", "hello world", []string{"hello", "world"}},
		{"with punctuation", "hello, world!", []string{"hello", "world"}},
		{"with numbers", "hello123 world", []string{"hello123", "world"}},
		{"empty", "", nil},
		{"only special chars", "!@#$", nil},
		{"mixed case", "Hello World", []string{"hello", "world"}},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := ExtractWords(tt.input)
			if len(result) != len(tt.expected) {
				t.Errorf("ExtractWords(%q) returned %d elements, expected %d", tt.input, len(result), len(tt.expected))
				return
			}
			for i, v := range result {
				if v != tt.expected[i] {
					t.Errorf("Element %d: got %q, expected %q", i, v, tt.expected[i])
				}
			}
		})
	}
}

func TestSafeJSONLoads(t *testing.T) {
	tests := []struct {
		name      string
		input     string
		expectNil bool
	}{
		{"valid object", `{"key": "value"}`, false},
		{"valid array", `[1, 2, 3]`, false},
		{"valid string", `"hello"`, false},
		{"valid number", "123", false},
		{"invalid json", `{invalid}`, true},
		{"empty string", "", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := SafeJSONLoads(tt.input)
			if tt.expectNil && result != nil {
				t.Errorf("SafeJSONLoads(%q) expected nil, got %v", tt.input, result)
			}
			if !tt.expectNil && result == nil {
				t.Errorf("SafeJSONLoads(%q) expected non-nil result", tt.input)
			}
		})
	}
}

func TestLoadString(t *testing.T) {
	tests := []struct {
		name      string
		input     string
		expectNil bool
	}{
		{"valid json", `{"key": "value"}`, false},
		{"python dict with single quotes", "{'key': 'value'}", false},
		{"empty string", "", true},
		{"plain string", "hello", false}, // Returns the string itself
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := LoadString(tt.input)
			if tt.expectNil && result != nil {
				t.Errorf("LoadString(%q) expected nil, got %v", tt.input, result)
			}
			if !tt.expectNil && result == nil {
				t.Errorf("LoadString(%q) expected non-nil result", tt.input)
			}
		})
	}
}

func TestFtsQuote(t *testing.T) {
	tests := []struct {
		name     string
		input    []string
		expected []string
	}{
		{"simple terms", []string{"hello", "world"}, []string{`"hello"`, `"world"`}},
		{"with AND operator", []string{"hello AND world"}, []string{"hello AND world"}},
		{"with OR operator", []string{"hello OR world"}, []string{"hello OR world"}},
		{"with NOT operator", []string{"hello NOT world"}, []string{"hello NOT world"}},
		{"with wildcard", []string{"hello*"}, []string{"hello*"}},
		{"with field", []string{"title:hello"}, []string{"title:hello"}},
		{"with NEAR", []string{"NEAR(hello, world)"}, []string{"NEAR(hello, world)"}},
		{"empty", []string{}, []string{}},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := FtsQuote(tt.input)
			if len(result) != len(tt.expected) {
				t.Errorf("FtsQuote(%v) returned %d elements, expected %d", tt.input, len(result), len(tt.expected))
				return
			}
			for i, v := range result {
				if v != tt.expected[i] {
					t.Errorf("Element %d: got %q, expected %q", i, v, tt.expected[i])
				}
			}
		})
	}
}

func TestEscapeXML(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"ampersand", "hello & world", "hello &amp; world"},
		{"less than", "hello < world", "hello &lt; world"},
		{"greater than", "hello > world", "hello &gt; world"},
		{"quotes", "hello \"world\"", "hello &quot;world&quot;"},
		{"apostrophe", "hello 'world'", "hello &apos;world&apos;"},
		{"multiple", "hello & <world>", "hello &amp; &lt;world&gt;"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := EscapeXML(tt.input)
			if result != tt.expected {
				t.Errorf("EscapeXML(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestPluralize(t *testing.T) {
	tests := []struct {
		name     string
		n        int
		singular string
		plural   string
		expected string
	}{
		{"one", 1, "cat", "cats", "cat"},
		{"zero", 0, "cat", "cats", "cats"},
		{"multiple", 5, "cat", "cats", "cats"},
		{"negative", -1, "cat", "cats", "cats"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := Pluralize(tt.n, tt.singular, tt.plural)
			if result != tt.expected {
				t.Errorf("Pluralize(%d, %q, %q) = %q, expected %q", tt.n, tt.singular, tt.plural, result, tt.expected)
			}
		})
	}
}
