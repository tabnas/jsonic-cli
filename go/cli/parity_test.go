// Copyright (c) 2013-2026 Richard Rodger and other contributors, MIT License

package cli

// parity_test.go — cross-runtime conformance, driven by the shared
// `test/spec/*.tsv` fixtures at the repo root (see ../../test/AGENTS.md).
//
// The fixture loader, the escape codec and the comparison come from
// github.com/tabnas/support/go, whose TypeScript half
// ts/test/parity.test.js uses to run the SAME files — so the two
// implementations cannot drift without one of them going red, and neither
// can the two loaders.
//
// (The TS side keeps its own row loop: running the CLI is asynchronous
// there, and the runner's loop is synchronous in both languages. It uses
// the shared loader and comparison, which is where the drift was.)
//
// Cases that turn on how a runtime loads code or reads the filesystem — the
// `-p` plugin fixtures and the `-f` file fixtures — stay in run_test.go,
// where each runtime's adaptation is spelled out. Everything expressible as
// argv (+ stdin) → stdout lives here.

import (
	"encoding/json"
	"strings"
	"testing"

	support "github.com/tabnas/support/go"
)

// Help text is long and version-ish; those rows pin a SUBSTRING of the
// output rather than the whole of it.
const containsPrefix = "CONTAINS:"

// TestSpec runs every fixture in the spec directory. FindSpecDir walks up
// from the package directory, and Dir discovers the files by listing, so
// adding a .tsv runs it in both runtimes without touching either runner.
func TestSpec(t *testing.T) {
	dir, err := support.FindSpecDir("")
	if err != nil {
		t.Fatal(err)
	}

	support.Runner{
		ParseRow: func(_ string, row *support.Row) (any, error) {
			var argv []string
			if err := json.Unmarshal([]byte(row.Named("argv")), &argv); err != nil {
				return nil, err
			}

			lines := capture(argv, row.UnescNamed("stdin"), nil)
			got := ""
			if 0 < len(lines) {
				got = lines[0]
			}

			// A CONTAINS row's expected value IS the substring (see
			// ParseExpected), so answer it when the output holds it. When it
			// does not, answering the whole output makes the failure print
			// what actually came out against what was wanted.
			if want, ok := containsWant(row); ok && strings.Contains(got, want) {
				return want, nil
			}
			return got, nil
		},

		ParseExpected: func(expected string, _ *support.Row) (any, error) {
			if strings.HasPrefix(expected, containsPrefix) {
				return strings.TrimPrefix(expected, containsPrefix), nil
			}
			return support.ParseExpect(expected)
		},

		InputName:    "argv",
		ExpectedName: "stdout",
	}.Dir(t, dir)
}

// containsWant returns the substring a CONTAINS row pins, and whether the
// row is one.
func containsWant(row *support.Row) (string, bool) {
	cell := row.Named("stdout")
	if !strings.HasPrefix(cell, containsPrefix) {
		return "", false
	}
	return strings.TrimPrefix(cell, containsPrefix), true
}
