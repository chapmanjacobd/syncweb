package utils

import "math"

func Unique[T comparable](slice []T) []T {
	m := make(map[T]bool)
	var res []T
	for _, v := range slice {
		if !m[v] {
			m[v] = true
			res = append(res, v)
		}
	}
	return res
}

func Concat[T any](slices ...[]T) []T {
	var totalLen int
	for _, s := range slices {
		totalLen += len(s)
	}
	res := make([]T, 0, totalLen)
	for _, s := range slices {
		res = append(res, s...)
	}
	return res
}

func SafeSum[T Number](slice []T) T {
	var sum T
	for _, v := range slice {
		sum += v
	}
	return sum
}

func SafeLen[T any](slice []T) int {
	return len(slice)
}

func SafePop[T any](slice []T) (T, []T) {
	if len(slice) == 0 {
		var zero T
		return zero, slice
	}
	return slice[len(slice)-1], slice[:len(slice)-1]
}

func SafeIndex[T comparable](slice []T, val T) int {
	for i, v := range slice {
		if v == val {
			return i
		}
	}
	return -1
}

func OrderedSet[T comparable](slice []T) []T {
	seen := make(map[T]bool)
	var res []T
	for _, v := range slice {
		if !seen[v] {
			seen[v] = true
			res = append(res, v)
		}
	}
	return res
}

func Flatten(v any) []any {
	var res []any
	switch val := v.(type) {
	case []any:
		for _, item := range val {
			res = append(res, Flatten(item)...)
		}
	case string:
		if val != "" {
			res = append(res, val)
		}
	case nil:
		// skip
	default:
		res = append(res, val)
	}
	return res
}

func Conform[T any](v any) []T {
	var res []T
	switch val := v.(type) {
	case []T:
		res = append(res, val...)
	case T:
		res = append(res, val)
	}
	return res
}

func SafeUnpack[T comparable](vals ...T) T {
	var zero T
	for _, v := range vals {
		if v != zero {
			return v
		}
	}
	return zero
}

func ListDictFilterBool(data []map[string]any) []map[string]any {
	var res []map[string]any
	for _, d := range data {
		if len(d) == 0 {
			continue
		}
		keep := false
		for _, v := range d {
			if v != nil && v != "" && v != 0 && v != false {
				keep = true
				break
			}
		}
		if keep {
			res = append(res, d)
		}
	}
	return res
}

func Chunks[T any](slice []T, chunkSize int) [][]T {
	if chunkSize <= 0 {
		return [][]T{slice}
	}
	var res [][]T
	for i := 0; i < len(slice); i += chunkSize {
		end := min(i+chunkSize, len(slice))
		res = append(res, slice[i:end])
	}
	return res
}

func Similarity[T comparable](a, b []T) float64 {
	if len(a) == 0 || len(b) == 0 {
		return 0.0
	}
	setA := make(map[T]bool)
	for _, v := range a {
		setA[v] = true
	}
	setB := make(map[T]bool)
	for _, v := range b {
		setB[v] = true
	}

	intersection := 0
	for v := range setA {
		if setB[v] {
			intersection++
		}
	}

	union := len(setA) + len(setB) - intersection
	if union == 0 {
		return 0.0
	}
	return float64(intersection) / float64(union)
}

func ValueCounts[T comparable](slice []T) []int {
	counts := make(map[T]int)
	for _, v := range slice {
		counts[v]++
	}
	res := make([]int, len(slice))
	for i, v := range slice {
		res[i] = counts[v]
	}
	return res
}

func Divisors(n int) []int {
	if n < 4 {
		return nil
	}
	var res []int
	sqrt := int(math.Sqrt(float64(n)))
	for i := 2; i <= sqrt; i++ {
		if n%i == 0 {
			res = append(res, i)
			if i*i != n {
				res = append(res, n/i)
			}
		}
	}
	return res
}

func DivideSequence[T Number](slice []T) float64 {
	if len(slice) == 0 {
		return 0
	}
	res := float64(slice[0])
	for _, v := range slice[1:] {
		val := float64(v)
		if val == 0 {
			if res > 0 {
				return math.Inf(1)
			} else if res < 0 {
				return math.Inf(-1)
			} else {
				return math.NaN()
			}
		}
		res /= val
	}
	return res
}
