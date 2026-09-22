// Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
// contributors, MIT License

package cli

import (
	"math"
	"sort"
	"strconv"
	"strings"
	"unicode/utf8"

	jsonic "github.com/tabnas/jsonic/go"
)

// parseReplacer resolves the JSON.replacer option into a key whitelist,
// mirroring the TS CLI:
//
//	replacer = Jsonic(options.JSON.replacer)   // parse the raw value
//	replacer = Array.isArray(replacer) ? replacer
//	         : null == replacer ? null
//	         : [replacer]                       // a scalar becomes [scalar]
//
// The bag already holds the *parsed* value (handleProps parses option leaf
// values with jsonic), so a raw "[b]" arrives as []any{"b"} and "b" as the
// string "b". A nil/absent replacer means "no filtering".
func parseReplacer(jsonBag map[string]any) []string {
	if jsonBag == nil {
		return nil
	}
	raw, ok := jsonBag["replacer"]
	if !ok || raw == nil {
		return nil
	}
	switch v := raw.(type) {
	case []any:
		keys := make([]string, 0, len(v))
		for _, e := range v {
			keys = append(keys, toKeyString(e))
		}
		return keys
	default:
		return []string{toKeyString(v)}
	}
}

// parseSpace resolves the JSON.space option into an indent string,
// mirroring JSON.stringify's space argument: a number N (clamped to 0..10)
// becomes N spaces; a string is cut at its first 10 UTF-16 code units. An
// absent or invalid space means no indentation.
func parseSpace(jsonBag map[string]any) string {
	if jsonBag == nil {
		return ""
	}
	raw, ok := jsonBag["space"]
	if !ok || raw == nil {
		return ""
	}
	switch v := raw.(type) {
	case string:
		return truncateUTF16(v, 10)
	case float64:
		return spaceFromNumber(v)
	case int:
		return spaceFromNumber(float64(v))
	case int64:
		return spaceFromNumber(float64(v))
	}
	return ""
}

// truncateUTF16 returns the leading `units` UTF-16 code units of s.
//
// JSON.stringify cuts a string `space` with substring(0, 10), and
// substring counts UTF-16 code units. A Go string counts BYTES, so
// s[:10] cuts a multi-byte character in half: the standard output then
// carries a lone lead byte and is not valid UTF-8 at all, once per
// indent level.
//
// A character STRADDLING the boundary is the sharp case. JavaScript
// keeps the high surrogate on its own, and Node writes an unpaired
// surrogate to a UTF-8 stream as U+FFFD, so the canonical command's
// indent has a replacement character where the gap is. A Go string
// cannot hold half a surrogate pair, and dropping the character is not
// the same answer, so the half becomes U+FFFD here. Measured under Node
// with -o 'JSON.space="abcdefghi<astral>"', whose indent is the nine
// ASCII characters plus the three bytes ef bf bd. rs/src/stringify.rs
// carries the same function.
func truncateUTF16(s string, units int) string {
	var b strings.Builder
	used := 0
	for _, r := range s {
		width := 1
		if 0xFFFF < r {
			width = 2
		}
		if units < used+width {
			if used < units {
				b.WriteRune(utf8.RuneError)
			}
			break
		}
		b.WriteRune(r)
		used += width
	}
	return b.String()
}

func spaceFromNumber(n float64) string {
	count := int(math.Floor(n))
	if count < 0 {
		count = 0
	}
	if count > 10 {
		count = 10
	}
	return strings.Repeat(" ", count)
}

// toKeyString renders a parsed replacer entry as a string key (JSON.stringify
// coerces replacer-array entries that are strings or numbers to strings).
func toKeyString(v any) string {
	switch t := v.(type) {
	case string:
		return t
	case float64:
		return formatNumber(t)
	case int:
		return strconv.Itoa(t)
	case int64:
		return strconv.FormatInt(t, 10)
	default:
		return ""
	}
}

// stringify serializes a parsed value to a string exactly like the browser
// JSON.stringify(value, replacer, space): object keys are filtered by the
// replacer whitelist (when non-nil, recursively at every object level) and
// indentation follows the space string. The engine's parse result is now an
// insertion-ordered *jsonic.OrderedMap (TS-engine parity), so object keys are
// emitted in SOURCE order — the order they appeared in the input — matching
// JSON.stringify over a JS object. A plain map[string]any (which carries no
// order) still falls back to sorted-key emission.
func stringify(value any, replacer []string, space string) string {
	// JSON.stringify(undefined) returns the JS value undefined (not a
	// string); console.log(undefined) then prints the literal "undefined".
	// This is the all-empty-source case (the seed data.val stays
	// Undefined). Mirror that output.
	if jsonic.IsUndefined(value) {
		return "undefined"
	}
	var b strings.Builder
	writeValue(&b, value, replacer, space, "")
	return b.String()
}

func writeValue(b *strings.Builder, v any, replacer []string, space, indent string) {
	switch t := v.(type) {
	case nil:
		b.WriteString("null")
	case bool:
		if t {
			b.WriteString("true")
		} else {
			b.WriteString("false")
		}
	case string:
		writeString(b, t)
	case float64:
		b.WriteString(formatNumber(t))
	case int:
		b.WriteString(strconv.Itoa(t))
	case int64:
		b.WriteString(strconv.FormatInt(t, 10))
	case int32:
		b.WriteString(strconv.FormatInt(int64(t), 10))
	case *jsonic.OrderedMap:
		writeOrderedObject(b, t, replacer, space, indent)
	case map[string]any:
		writeObject(b, t, replacer, space, indent)
	case []any:
		writeArray(b, t, replacer, space, indent)
	default:
		// Anything else falls back to jsonic-string coercion is overkill;
		// JSON.stringify of an unknown becomes its String form only for
		// functions/undefined (omitted). Treat as null to stay valid JSON.
		b.WriteString("null")
	}
}

func writeObject(b *strings.Builder, m map[string]any, replacer []string, space, indent string) {
	keys := make([]string, 0, len(m))
	for k := range m {
		if replacer != nil && !inSet(replacer, k) {
			continue
		}
		// JSON.stringify omits keys whose value is undefined/function; the
		// engine never produces those, so all remaining keys are kept.
		keys = append(keys, k)
	}
	if len(keys) == 0 {
		b.WriteString("{}")
		return
	}
	sort.Strings(keys)

	newIndent := indent + space
	b.WriteByte('{')
	for i, k := range keys {
		if i > 0 {
			b.WriteByte(',')
		}
		if space != "" {
			b.WriteByte('\n')
			b.WriteString(newIndent)
		}
		writeString(b, k)
		b.WriteByte(':')
		if space != "" {
			b.WriteByte(' ')
		}
		writeValue(b, m[k], replacer, space, newIndent)
	}
	if space != "" {
		b.WriteByte('\n')
		b.WriteString(indent)
	}
	b.WriteByte('}')
}

// writeOrderedObject serializes an OrderedMap parse node in SOURCE key order
// (om.Keys), applying the replacer whitelist. Unlike writeObject it does not
// sort: the engine's parse result preserves insertion order (TS-engine
// parity), so a faithful stringify emits keys in the order they appeared in
// the source, exactly like JSON.stringify over a JS object.
func writeOrderedObject(b *strings.Builder, om *jsonic.OrderedMap, replacer []string, space, indent string) {
	keys := make([]string, 0, len(om.Keys))
	for _, k := range om.Keys {
		if replacer != nil && !inSet(replacer, k) {
			continue
		}
		// JSON.stringify omits keys whose value is undefined/function; the
		// engine never produces those, so all remaining keys are kept.
		keys = append(keys, k)
	}
	if len(keys) == 0 {
		b.WriteString("{}")
		return
	}

	newIndent := indent + space
	b.WriteByte('{')
	for i, k := range keys {
		if i > 0 {
			b.WriteByte(',')
		}
		if space != "" {
			b.WriteByte('\n')
			b.WriteString(newIndent)
		}
		writeString(b, k)
		b.WriteByte(':')
		if space != "" {
			b.WriteByte(' ')
		}
		writeValue(b, om.Vals[k], replacer, space, newIndent)
	}
	if space != "" {
		b.WriteByte('\n')
		b.WriteString(indent)
	}
	b.WriteByte('}')
}

func writeArray(b *strings.Builder, a []any, replacer []string, space, indent string) {
	if len(a) == 0 {
		b.WriteString("[]")
		return
	}
	newIndent := indent + space
	b.WriteByte('[')
	for i, el := range a {
		if i > 0 {
			b.WriteByte(',')
		}
		if space != "" {
			b.WriteByte('\n')
			b.WriteString(newIndent)
		}
		// Array elements are not filtered by the replacer whitelist (only
		// object keys are), but nested objects within them are.
		writeValue(b, el, replacer, space, newIndent)
	}
	if space != "" {
		b.WriteByte('\n')
		b.WriteString(indent)
	}
	b.WriteByte(']')
}

// formatNumber renders a number the way JSON.stringify does: a
// non-finite number is null, and everything else is JavaScript's
// Number::toString.
func formatNumber(f float64) string {
	if math.IsNaN(f) || math.IsInf(f, 0) {
		return "null"
	}
	return jsNumberToString(f)
}

// jsNumberToString is ECMAScript Number::toString with radix 10
// (ECMA-262 6.1.6.1.20), which is what String(n) gives and therefore what
// JSON.stringify writes for a finite number.
//
// Neither of Go's shortest forms is a substitute. FormatFloat(f, 'f', -1,
// 64) never switches to an exponent, so 1e21 comes out as twenty-two
// digits where JavaScript writes 1e+21. FormatFloat(f, 'g', -1, 64)
// switches much earlier than the specification does, so 1658206780088562.2
// came out as 1.6582067800885622e+15 and 0.000001 as 1e-06.
//
// The digits come from a FIXED-precision render rather than a shortest
// one. Both round-trip, but they break an exact decimal midpoint
// differently: the shortest form rounds away from zero, while the
// specification takes the even digit, which is what a fixed-precision
// render does. 1658206780088562.2 is such a midpoint. This is a port of
// jsNumberToString in tabnas/csv's go/csv.go, and rs/src/stringify.rs
// carries the same algorithm. Fuzzed against Node here over 221,898
// doubles (random bit patterns, decimal-scaled values, midpoint-prone
// halves, powers of ten and their neighbours, subnormals and the
// extremes) with no mismatch. ts/src/jsonic-cli.ts needs none of it,
// because the language does it.
//
// Deliberately NOT the exact integer value via FormatInt(int64(f)): past
// 2^53 the two differ (JavaScript prints 2^63 as 9223372036854776000, not
// 9223372036854775808), and outside int64 range a Go float to int
// conversion is undefined and wraps. 0xFFFFFFFFFFFFFFFF, which the engine
// yields as a float64, came out as -9223372036854775808.
func jsNumberToString(f float64) string {
	// Covers -0, which JavaScript prints as "0".
	if f == 0 {
		return "0"
	}

	magnitude := math.Abs(f)

	// The specification's `s` (the digits) and `n` (where the decimal
	// point sits). Take the digit count from the shortest form, then take
	// the digits themselves at that fixed precision.
	shortest := strconv.FormatFloat(magnitude, 'e', -1, 64)
	mantissa, _, _ := strings.Cut(shortest, "e")
	k := len(strings.Replace(mantissa, ".", "", 1))

	fixed := strconv.FormatFloat(magnitude, 'e', k-1, 64)
	mantissa, exponentText, _ := strings.Cut(fixed, "e")
	digits := strings.Replace(mantissa, ".", "", 1)
	exponent, err := strconv.Atoi(exponentText)
	if err != nil {
		// FormatFloat with 'e' always emits a signed integer exponent.
		return strconv.FormatFloat(f, 'g', -1, 64)
	}
	n := exponent + 1

	var body string
	switch {
	case k <= n && n <= 21:
		// 12 -> "12", 1e19 -> "10000000000000000000"
		body = digits + strings.Repeat("0", n-k)
	case 0 < n && n <= 21:
		// 1.5 -> "1.5"
		body = digits[:n] + "." + digits[n:]
	case -6 < n && n <= 0:
		// 1e-6 -> "0.000001"
		body = "0." + strings.Repeat("0", -n) + digits
	default:
		// 1e21 -> "1e+21", 1e-7 -> "1e-7"
		e := n - 1
		head := digits
		if 1 < k {
			head = digits[:1] + "." + digits[1:]
		}
		sign := "+"
		if e < 0 {
			sign = "-"
			e = -e
		}
		body = head + "e" + sign + strconv.Itoa(e)
	}

	if f < 0 {
		return "-" + body
	}
	return body
}

// writeString writes a JSON-escaped, double-quoted string matching the
// escaping JSON.stringify performs.
func writeString(b *strings.Builder, s string) {
	b.WriteByte('"')
	for _, r := range s {
		switch r {
		case '"':
			b.WriteString(`\"`)
		case '\\':
			b.WriteString(`\\`)
		case '\n':
			b.WriteString(`\n`)
		case '\r':
			b.WriteString(`\r`)
		case '\t':
			b.WriteString(`\t`)
		case '\b':
			b.WriteString(`\b`)
		case '\f':
			b.WriteString(`\f`)
		default:
			if r < 0x20 {
				b.WriteString(`\u`)
				const hex = "0123456789abcdef"
				b.WriteByte(hex[(r>>12)&0xf])
				b.WriteByte(hex[(r>>8)&0xf])
				b.WriteByte(hex[(r>>4)&0xf])
				b.WriteByte(hex[r&0xf])
			} else if r == utf8.RuneError {
				b.WriteString(`�`)
			} else {
				b.WriteRune(r)
			}
		}
	}
	b.WriteByte('"')
}

func inSet(set []string, k string) bool {
	for _, s := range set {
		if s == k {
			return true
		}
	}
	return false
}
