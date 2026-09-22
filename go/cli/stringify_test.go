// Copyright (c) 2013-2026 Richard Rodger and other contributors, MIT License

// stringify_test.go — the output contract of go/cli/stringify.go, for the
// cases a shared `test/spec/*.tsv` row cannot carry.
//
// Everything expressible as argv (+ stdin) -> stdout belongs in
// test/spec/, which all three runtimes run. Two families cannot live
// there:
//
//   - A cut through an astral character. The canonical command holds a
//     lone high surrogate and Node writes it to a UTF-8 stream as U+FFFD,
//     so the expected cell would have to hold an unpaired surrogate: JSON
//     keeps it in TypeScript, Go's json.Unmarshal folds it to U+FFFD and
//     serde_json refuses it outright. The three runtimes would be reading
//     three different rows.
//
//   - The number table below, which asserts formatNumber directly over
//     doubles that no source text names exactly.
//
// Every expectation here was MEASURED under Node, with String(n) and with
// JSON.stringify over the same value, and the comment on each block says
// which. A formatter checked against itself proves nothing.
package cli

import (
	"math"
	"strconv"
	"strings"
	"testing"
	"unicode/utf8"
)

// A string JSON.space is cut at ten UTF-16 code units, and a cut that
// lands INSIDE an astral character keeps the half as U+FFFD rather than
// dropping the character or splitting its bytes.
//
// Measured under Node against ts/src/jsonic-cli.ts:
//
//	run(['node','jsonic','-o','JSON.space=abcdefghi\u{1F600}','a:{b:1}'], cn)
//
// prints "{\nabcdefghi\ud83d\"a\": {\n…", whose \ud83d is a lone high
// surrogate; written to a UTF-8 stream that is the three bytes ef bf bd.
// The byte cut this replaced wrote the single byte f0, which no UTF-8
// reader accepts, once per indent level (tabnas/jsonic-cli#29).
func TestSpaceCutThroughAnAstralCharacter(t *testing.T) {
	out := capture([]string{"-o", "JSON.space=abcdefghi\U0001F600", "a:{b:1}"}, "", nil)
	if len(out) == 0 {
		t.Fatal("no output")
	}
	got := out[0]

	if !utf8.ValidString(got) {
		t.Fatalf("standard output is not valid UTF-8: %q", got)
	}
	want := "{\nabcdefghi�\"a\": {\nabcdefghi�abcdefghi�\"b\": 1\nabcdefghi�}\n}"
	if got != want {
		t.Fatalf("astral cut:\n got %q\nwant %q", got, want)
	}
	if strings.ContainsRune(got, '\U0001F600') {
		t.Fatal("the astral character survived a cut that falls inside it")
	}
}

// A string JSON.space shorter than the cut is used as it stands, and one
// made only of astral characters loses half of them to the ten-unit
// count: six astral characters are twelve UTF-16 units, so five survive.
// The second case is also pinned by test/spec/stringify.tsv; this asserts
// the helper that decides it.
func TestTruncateUTF16(t *testing.T) {
	for _, c := range []struct{ in, want string }{
		{"", ""},
		{"--", "--"},
		{"abcdefghij", "abcdefghij"},
		{"abcdefghijk", "abcdefghij"},
		{"\U0001F600\U0001F600\U0001F600\U0001F600\U0001F600\U0001F600",
			"\U0001F600\U0001F600\U0001F600\U0001F600\U0001F600"},
		{"abcdefghi\U0001F600", "abcdefghi�"},
		// A character that is one UTF-16 unit but three bytes: ten of
		// them are ten units and thirty bytes, so all ten survive where a
		// byte cut kept three.
		{strings.Repeat("é", 12), strings.Repeat("é", 10)},
		{strings.Repeat("中", 12), strings.Repeat("中", 10)},
	} {
		if got := truncateUTF16(c.in, 10); got != c.want {
			t.Errorf("truncateUTF16(%q): got %q, want %q", c.in, got, c.want)
		}
	}
}

// formatNumber is ECMAScript Number::toString, with a non-finite number
// answered as null the way JSON.stringify answers it.
//
// Every row was measured under Node as String(n), and the null rows as
// JSON.stringify(n). The boundaries are the specification's: fixed
// notation from 1e-6 up to just under 1e21, an exponent outside that, and
// no leading zero in the exponent (Go's %g writes 1e-06).
func TestFormatNumberIsJavaScriptNumberToString(t *testing.T) {
	for _, c := range []struct {
		in   float64
		want string
	}{
		{0, "0"},
		{math.Copysign(0, -1), "0"},
		{1, "1"},
		{-2.25, "-2.25"},
		{0.1, "0.1"},
		{1e3, "1000"},
		{1e20, "100000000000000000000"},
		{1e21, "1e+21"},
		{1.5e22, "1.5e+22"},
		{18446744073709551615.0, "18446744073709552000"},
		{9223372036854775808.0, "9223372036854776000"},
		{0.000001, "0.000001"},
		{1e-7, "1e-7"},
		{-1.5e-9, "-1.5e-9"},
		{1.2345e-8, "1.2345e-8"},
		{5e-324, "5e-324"},
		{1.7976931348623157e308, "1.7976931348623157e+308"},
		// A shortest-form TIE: two equally short digit strings sit the
		// same distance from the double, and the specification takes the
		// one ending in an even digit. Go's shortest form rounds away
		// from zero and printed 1658206780088562.3 through a
		// fixed-notation render, and 1.6582067800885622e+15 through %g.
		{1658206780088562.2, "1658206780088562.2"},
	} {
		if got := formatNumber(c.in); got != c.want {
			t.Errorf("formatNumber(%v): got %q, want %q", c.in, got, c.want)
		}
	}

	for _, c := range []struct {
		in   float64
		want string
	}{
		{math.NaN(), "null"},
		{math.Inf(1), "null"},
		{math.Inf(-1), "null"},
	} {
		if got := formatNumber(c.in); got != c.want {
			t.Errorf("formatNumber(%v): got %q, want %q", c.in, got, c.want)
		}
	}
}

// Every finite double printed by formatNumber reads back as itself.
// Round-tripping is what makes the digit count the specification's `k`,
// and a render that lost a digit would still look plausible in the table
// above.
func TestFormatNumberRoundTrips(t *testing.T) {
	for _, n := range []float64{
		1658206780088562.2, 0.1, 1e-7, 5e-324, 1.7976931348623157e308,
		18446744073709551615.0, 1.2345e-8, -2.25, 1e21, 123456.789,
	} {
		text := formatNumber(n)
		back, err := strconv.ParseFloat(text, 64)
		if err != nil {
			t.Errorf("formatNumber(%v) = %q, which does not parse: %v", n, text, err)
			continue
		}
		if back != n {
			t.Errorf("formatNumber(%v) = %q, which reads back as %v", n, text, back)
		}
	}
}
