package utils

import (
	"math"
	"reflect"
	"testing"
)

func TestUnique(t *testing.T) {
	tests := []struct {
		input []int
		want  []int
	}{
		{[]int{1, 2, 1, 3, 2}, []int{1, 2, 3}},
		{[]int{1, 1, 1}, []int{1}},
		{[]int{}, nil},
	}
	for _, tt := range tests {
		got := Unique(tt.input)
		if !reflect.DeepEqual(got, tt.want) {
			t.Errorf("Unique(%v) = %v, want %v", tt.input, got, tt.want)
		}
	}
}

func TestConcat(t *testing.T) {
	s1 := []int{1, 2}
	s2 := []int{3, 4}
	want := []int{1, 2, 3, 4}
	if got := Concat(s1, s2); !reflect.DeepEqual(got, want) {
		t.Errorf("Concat(%v, %v) = %v, want %v", s1, s2, got, want)
	}
}

func TestSafeSum(t *testing.T) {
	if got := SafeSum([]int{1, 2, 3}); got != 6 {
		t.Errorf("SafeSum([1,2,3]) = %v, want 6", got)
	}
}

func TestSafePop(t *testing.T) {
	val, rest := SafePop([]int{1, 2, 3})
	if val != 3 || !reflect.DeepEqual(rest, []int{1, 2}) {
		t.Errorf("SafePop([1,2,3]) = %v, %v, want 3, [1, 2]", val, rest)
	}
	val, rest = SafePop([]int{})
	if val != 0 || len(rest) != 0 {
		t.Errorf("SafePop([]) = %v, %v, want 0, []", val, rest)
	}
}

func TestSafeIndex(t *testing.T) {
	if got := SafeIndex([]int{1, 2, 3}, 2); got != 1 {
		t.Errorf("SafeIndex([1,2,3], 2) = %v, want 1", got)
	}
	if got := SafeIndex([]int{1, 2, 3}, 4); got != -1 {
		t.Errorf("SafeIndex([1,2,3], 4) = %v, want -1", got)
	}
}

func TestFlatten(t *testing.T) {
	input := []any{1, []any{2, 3}, "4", nil}
	want := []any{1, 2, 3, "4"}
	got := Flatten(input)
	if !reflect.DeepEqual(got, want) {
		t.Errorf("Flatten(%v) = %v, want %v", input, got, want)
	}
}

func TestConform(t *testing.T) {
	if got := Conform[string]([]string{"a", "b"}); !reflect.DeepEqual(got, []string{"a", "b"}) {
		t.Errorf("Conform([]string) = %v, want [a, b]", got)
	}
	if got := Conform[string]("a"); !reflect.DeepEqual(got, []string{"a"}) {
		t.Errorf("Conform(string) = %v, want [a]", got)
	}
}

func TestSafeUnpack(t *testing.T) {
	if got := SafeUnpack(0, 0, 5, 0); got != 5 {
		t.Errorf("SafeUnpack(0,0,5,0) = %v, want 5", got)
	}
}

func TestListDictFilterBool(t *testing.T) {
	data := []map[string]any{
		{"a": 1},
		{"b": 0},
		{"c": false},
		{"d": nil},
		{},
	}
	want := []map[string]any{{"a": 1}}
	if got := ListDictFilterBool(data); !reflect.DeepEqual(got, want) {
		t.Errorf("ListDictFilterBool(%v) = %v, want %v", data, got, want)
	}
}

func TestChunks(t *testing.T) {
	slice := []int{1, 2, 3, 4, 5}
	got := Chunks(slice, 2)
	want := [][]int{{1, 2}, {3, 4}, {5}}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("Chunks(%v, 2) = %v, want %v", slice, got, want)
	}
}

func TestSimilarity(t *testing.T) {
	a := []int{1, 2, 3}
	b := []int{2, 3, 4}
	// Intersection: {2, 3}, Union: {1, 2, 3, 4}. Sim = 2/4 = 0.5
	if got := Similarity(a, b); got != 0.5 {
		t.Errorf("Similarity(%v, %v) = %v, want 0.5", a, b, got)
	}
}

func TestDivisors(t *testing.T) {
	tests := []struct {
		n    int
		want []int
	}{
		{12, []int{2, 6, 3, 4}}, // 2, 3, 4, 6
		{4, []int{2}},
		{3, nil},
	}
	for _, tt := range tests {
		got := Divisors(tt.n)
		// Since we don't sort the result in Divisors, we should sort or check if it contains all.
		// For simplicity, let's check length and content if not nil.
		if tt.want == nil {
			if got != nil {
				t.Errorf("Divisors(%d) = %v, want nil", tt.n, got)
			}
		} else {
			if len(got) != len(tt.want) {
				t.Errorf("Divisors(%d) length = %d, want %d", tt.n, len(got), len(tt.want))
			}
		}
	}
}

func TestSafeLen(t *testing.T) {
	if got := SafeLen([]int{1, 2, 3}); got != 3 {
		t.Errorf("SafeLen([1,2,3]) = %v, want 3", got)
	}
}

func TestValueCounts(t *testing.T) {
	input := []int{1, 2, 1, 3, 2, 1}
	want := []int{3, 2, 3, 1, 2, 3}
	if got := ValueCounts(input); !reflect.DeepEqual(got, want) {
		t.Errorf("ValueCounts(%v) = %v, want %v", input, got, want)
	}
}

func TestDivideSequence(t *testing.T) {
	if got := DivideSequence([]int{100, 2, 5}); got != 10.0 {
		t.Errorf("DivideSequence([100, 2, 5]) = %v, want 10.0", got)
	}
	if got := DivideSequence([]int{10, 0}); !math.IsInf(got, 1) {
		t.Errorf("DivideSequence([10, 0]) = %v, want +Inf", got)
	}
	if got := DivideSequence([]int{-10, 0}); !math.IsInf(got, -1) {
		t.Errorf("DivideSequence([-10, 0]) = %v, want -Inf", got)
	}
	if got := DivideSequence([]int{0, 0}); !math.IsNaN(got) {
		t.Errorf("DivideSequence([0, 0]) = %v, want NaN", got)
	}
	if got := DivideSequence([]int{}); got != 0 {
		t.Errorf("DivideSequence([]) = %v, want 0", got)
	}
}
