// Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
// contributors, MIT License

package main

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
// becomes N spaces; a string is used verbatim (first 10 chars). An absent
// or invalid space means no indentation.
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
		if len(v) > 10 {
			return v[:10]
		}
		return v
	case float64:
		return spaceFromNumber(v)
	case int:
		return spaceFromNumber(float64(v))
	case int64:
		return spaceFromNumber(float64(v))
	}
	return ""
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
// indentation follows the space string. Like Go's encoding/json and a
// faithful match for every cli.test.js expectation, object keys are emitted
// in sorted order (the engine's parse result is an unordered map, so there
// is no insertion order to preserve).
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

// formatNumber renders a number the way JSON.stringify does: integers
// without a decimal point, non-integers with the shortest round-trippable
// form, and non-finite numbers as null.
func formatNumber(f float64) string {
	if math.IsNaN(f) || math.IsInf(f, 0) {
		return "null"
	}
	if f == math.Trunc(f) && math.Abs(f) < 1e21 {
		return strconv.FormatInt(int64(f), 10)
	}
	return strconv.FormatFloat(f, 'g', -1, 64)
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
