// Package utils provides common utility functions
package utils

import (
	"database/sql"
	"fmt"
	"strconv"
	"strings"
)

// GetString returns the string value of v, or empty string if v is not a string
func GetString(v any) string {
	if s, ok := v.(string); ok {
		return s
	}
	return ""
}

// GetInt returns the int value of v, or 0 if v is not an integer type
func GetInt(v any) int {
	switch i := v.(type) {
	case int:
		return i
	case int32:
		return int(i)
	case int64:
		return int(i)
	}
	return 0
}

// GetInt64 returns the int64 value of v, or 0 if v is not an integer type
func GetInt64(v any) int64 {
	switch i := v.(type) {
	case int:
		return int64(i)
	case int32:
		return int64(i)
	case int64:
		return i
	}
	return 0
}

// StringValue returns the value of s, or empty string if s is nil
func StringValue(s *string) string {
	if s == nil {
		return ""
	}
	return *s
}

// Int64Value returns the value of i, or 0 if i is nil
func Int64Value(i *int64) int64 {
	if i == nil {
		return 0
	}
	return *i
}

// Range represents a numeric range with optional min, max, and exact value
type Range struct {
	Min   *int64
	Max   *int64
	Value *int64
}

// Matches returns true if val matches the range criteria
func (r Range) Matches(val int64) bool {
	if r.Value != nil && val != *r.Value {
		return false
	}
	if r.Min != nil && val < *r.Min {
		return false
	}
	if r.Max != nil && val > *r.Max {
		return false
	}
	return true
}

// ToNullInt64 converts int64 to [sql.NullInt64]
func ToNullInt64(i int64) sql.NullInt64 {
	return sql.NullInt64{Int64: i, Valid: i != 0}
}

// ToNullString converts string to [sql.NullString]
func ToNullString(s string) sql.NullString {
	return sql.NullString{String: s, Valid: s != ""}
}

// ToNullFloat64 converts float64 to [sql.NullFloat64]
func ToNullFloat64(f float64) sql.NullFloat64 {
	return sql.NullFloat64{Float64: f, Valid: f != 0}
}

// Slice represents a Python-style slice with start, stop, and step
type Slice struct {
	Start *int
	Stop  *int
	Step  *int
}

// ParseSlice parses a slice string (e.g., "1:10:2") into start, stop, and step pointers
func ParseSlice(s string) (Slice, error) {
	parts := strings.Split(s, ":")
	if len(parts) > 3 {
		return Slice{}, fmt.Errorf("invalid slice: %s", s)
	}

	var res Slice
	if len(parts) >= 1 && parts[0] != "" {
		if val, err := strconv.Atoi(parts[0]); err == nil {
			res.Start = &val
		}
	}
	if len(parts) >= 2 && parts[1] != "" {
		if val, err := strconv.Atoi(parts[1]); err == nil {
			res.Stop = &val
		}
	}
	if len(parts) == 3 && parts[2] != "" {
		if val, err := strconv.Atoi(parts[2]); err == nil {
			res.Step = &val
		}
	}

	return res, nil
}

// DictFilterBool filters a map to remove empty values (nil, "", 0, false)
func DictFilterBool(d map[string]any) map[string]any {
	if d == nil {
		return nil
	}
	res := make(map[string]any)
	for k, v := range d {
		if v != nil && v != "" && v != 0 && !isFalse(v) {
			res[k] = v
		}
	}
	if len(res) == 0 {
		return nil
	}
	return res
}

func isFalse(v any) bool {
	b, ok := v.(bool)
	return ok && !b
}
