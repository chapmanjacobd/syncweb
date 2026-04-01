package utils

import (
	"reflect"
	"testing"
)

func TestSafeMean(t *testing.T) {
	tests := []struct {
		input []int
		want  float64
	}{
		{[]int{1, 2, 3}, 2.0},
		{[]int{10, 20}, 15.0},
		{[]int{}, 0.0},
		{nil, 0.0},
	}
	for _, tt := range tests {
		if got := SafeMean(tt.input); got != tt.want {
			t.Errorf("SafeMean(%v) = %v, want %v", tt.input, got, tt.want)
		}
	}
}

func TestSafeMedian(t *testing.T) {
	tests := []struct {
		input []int
		want  float64
	}{
		{[]int{1, 2, 3}, 2.0},
		{[]int{1, 2, 3, 4}, 2.5},
		{[]int{10}, 10.0},
		{[]int{}, 0.0},
	}
	for _, tt := range tests {
		if got := SafeMedian(tt.input); got != tt.want {
			t.Errorf("SafeMedian(%v) = %v, want %v", tt.input, got, tt.want)
		}
	}
}

func TestPercentile(t *testing.T) {
	tests := []struct {
		input []int
		p     float64
		want  float64
	}{
		{[]int{1, 2, 3, 4, 5}, 50, 3.0},
		{[]int{1, 2, 3, 4, 5}, 0, 1.0},
		{[]int{1, 2, 3, 4, 5}, 100, 5.0},
		{[]int{1, 10}, 50, 5.5},
		{[]int{}, 50, 0.0},
	}
	for _, tt := range tests {
		if got := Percentile(tt.input, tt.p); got != tt.want {
			t.Errorf("Percentile(%v, %v) = %v, want %v", tt.input, tt.p, got, tt.want)
		}
	}
}

func TestHumanToBits(t *testing.T) {
	tests := []struct {
		input   string
		want    int64
		wantErr bool
	}{
		{"1KBIT", 1000, false},
		{"1 MBIT", 1000 * 1000, false},
		{"1GBIT", 1000 * 1000 * 1000, false},
		{"1TBIT", 1000 * 1000 * 1000 * 1000, false},
		{"1K", 1000, false},
		{"1M", 1000 * 1000, false},
		{"1G", 1000 * 1000 * 1000, false},
		{"1T", 1000 * 1000 * 1000 * 1000, false},
		{"1000BIT", 1000, false},
		{"1000", 1000, false},
		{"abc", 0, true},
	}
	for _, tt := range tests {
		got, err := HumanToBits(tt.input)
		if (err != nil) != tt.wantErr {
			t.Errorf("HumanToBits(%q) error = %v, wantErr %v", tt.input, err, tt.wantErr)
			continue
		}
		if got != tt.want {
			t.Errorf("HumanToBits(%q) = %v, want %v", tt.input, got, tt.want)
		}
	}
}

func TestLinearInterpolation(t *testing.T) {
	dataPoints := [][2]float64{
		{0, 0},
		{10, 100},
		{20, 200},
	}
	tests := []struct {
		x    float64
		want float64
	}{
		{-1, 0},
		{0, 0},
		{5, 50},
		{10, 100},
		{15, 150},
		{20, 200},
		{21, 200},
	}
	for _, tt := range tests {
		if got := LinearInterpolation(tt.x, dataPoints); got != tt.want {
			t.Errorf("LinearInterpolation(%v) = %v, want %v", tt.x, got, tt.want)
		}
	}
	if got := LinearInterpolation(5, nil); got != 0 {
		t.Errorf("LinearInterpolation(5, nil) = %v, want 0", got)
	}
}

func TestCalculatePercentiles(t *testing.T) {
	values := []int64{10, 20, 30, 40, 50}
	got := CalculatePercentiles(values)
	if len(got) != 101 {
		t.Errorf("CalculatePercentiles() returned slice of length %d, want 101", len(got))
	}
	// Check some values
	if got[0] != 10 {
		t.Errorf("got[0] = %v, want 10", got[0])
	}
	if got[100] != 50 {
		t.Errorf("got[100] = %v, want 50", got[100])
	}

	empty := CalculatePercentiles(nil)
	if len(empty) != 101 {
		t.Errorf("CalculatePercentiles(nil) returned slice of length %d, want 101", len(empty))
	}
}

func TestPercentageDifference(t *testing.T) {
	tests := []struct {
		v1, v2 float64
		want   float64
	}{
		{100, 110, 9.523809523809524},
		{100, 100, 0.0},
		{0, 0, 100.0},
	}
	for _, tt := range tests {
		got := PercentageDifference(tt.v1, tt.v2)
		if reflect.DeepEqual(got, tt.want) == false && (got-tt.want) > 0.000001 {
			t.Errorf("PercentageDifference(%v, %v) = %v, want %v", tt.v1, tt.v2, got, tt.want)
		}
	}
}

func TestSQLHumanTime(t *testing.T) {
	tests := []struct {
		input string
		want  string
	}{
		{"10", "10 minutes"},
		{"10s", "10 seconds"},
		{"10min", "10 minutes"},
		{"10.5h", "10.5 h"}, // regex is (\d+\.?\d*)([a-zA-Z]+), but 'h' is not in unitMapping
		{"10.5hr", "10.5 hr"},
	}
	for _, tt := range tests {
		if got := SQLHumanTime(tt.input); got != tt.want {
			t.Errorf("SQLHumanTime(%q) = %q, want %q", tt.input, got, tt.want)
		}
	}
}

func TestMinMax(t *testing.T) {
	if got := Min(1, 2); got != 1 {
		t.Errorf("Min(1, 2) = %v, want 1", got)
	}
	if got := Max(1, 2); got != 2 {
		t.Errorf("Max(1, 2) = %v, want 2", got)
	}
}

func TestParseRange(t *testing.T) {
	tests := []struct {
		input     string
		humanToX  func(string) (int64, error)
		wantMin   *int64
		wantMax   *int64
		wantValue *int64
		wantErr   bool
	}{
		{"10-20", HumanToBytes, new(int64(10)), new(int64(20)), nil, false},
		{">10", HumanToBytes, new(int64(11)), nil, nil, false},
		{"<20", HumanToBytes, nil, new(int64(19)), nil, false},
		{"+10", HumanToBytes, new(int64(10)), nil, nil, false},
		{"-20", HumanToBytes, nil, new(int64(20)), nil, false},
		{"100%10", HumanToBytes, new(int64(90)), new(int64(110)), nil, false},
		{"10,20", HumanToBytes, nil, nil, new(int64(20)), false}, // Split by ',' last one wins in ParseRange implementation
		{"1KB-2KB", HumanToBytes, new(int64(1024)), new(int64(2048)), nil, false},
		{"100", HumanToBytes, nil, nil, new(int64(100)), false},
		{"", HumanToBytes, nil, nil, nil, false},
	}
	for _, tt := range tests {
		got, err := ParseRange(tt.input, tt.humanToX)
		if (err != nil) != tt.wantErr {
			t.Errorf("ParseRange(%q) error = %v, wantErr %v", tt.input, err, tt.wantErr)
			continue
		}
		if !ptrEqual(got.Min, tt.wantMin) || !ptrEqual(got.Max, tt.wantMax) || !ptrEqual(got.Value, tt.wantValue) {
			t.Errorf("ParseRange(%q) = %+v, want Min:%v Max:%v Value:%v", tt.input, got, tt.wantMin, tt.wantMax, tt.wantValue)
		}
	}
}

//nolint:gocheckcompilerdirectives // inline directive for test helper
//go:fix inline

func int64Ptr(i int64) *int64 { return new(i) }

func ptrEqual(a, b *int64) bool {
	if a == nil && b == nil {
		return true
	}
	if a == nil || b == nil {
		return false
	}
	return *a == *b
}

func TestPercent(t *testing.T) {
	if got := Percent(10, 100); got != 10.0 {
		t.Errorf("Percent(10, 100) = %v, want 10.0", got)
	}
	if got := Percent(10, 0); got != 0.0 {
		t.Errorf("Percent(10, 0) = %v, want 0.0", got)
	}
}

func TestFloatFromPercent(t *testing.T) {
	tests := []struct {
		input string
		want  float64
	}{
		{"50%", 0.5},
		{"100%", 1.0},
		{"0.5", 0.5},
	}
	for _, tt := range tests {
		got, _ := FloatFromPercent(tt.input)
		if got != tt.want {
			t.Errorf("FloatFromPercent(%q) = %v, want %v", tt.input, got, tt.want)
		}
	}
}

func TestParsePercentileRange(t *testing.T) {
	tests := []struct {
		input   string
		wantMin float64
		wantMax float64
		wantOk  bool
	}{
		{"p10-20", 10.0, 20.0, true},
		{"10-20", 0, 0, false},
		{"p10", 0, 0, false},
	}
	for _, tt := range tests {
		minVal, maxVal, ok := ParsePercentileRange(tt.input)
		if ok != tt.wantOk || minVal != tt.wantMin || maxVal != tt.wantMax {
			t.Errorf("ParsePercentileRange(%q) = %v, %v, %v, want %v, %v, %v", tt.input, minVal, maxVal, ok, tt.wantMin, tt.wantMax, tt.wantOk)
		}
	}
}

func TestCalculateSegments(t *testing.T) {
	got := CalculateSegments(110, 30, 10)
	want := []float64{0, 40, 80}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("CalculateSegments(110, 30, 10) = %v, want %v", got, want)
	}
}

func TestCalculateSegmentsInt(t *testing.T) {
	got := CalculateSegmentsInt(110, 30, 10)
	want := []int64{0, 40, 80}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("CalculateSegmentsInt(110, 30, 10) = %v, want %v", got, want)
	}
}

func TestSafeIntFloat(t *testing.T) {
	if got := SafeInt("10"); *got != 10 {
		t.Errorf("SafeInt('10') = %v, want 10", *got)
	}
	if got := SafeInt(""); got != nil {
		t.Errorf("SafeInt('') = %v, want nil", got)
	}
	if got := SafeFloat("10.5"); *got != 10.5 {
		t.Errorf("SafeFloat('10.5') = %v, want 10.5", *got)
	}
	if got := SafeFloat(""); got != nil {
		t.Errorf("SafeFloat('') = %v, want nil", got)
	}
}
