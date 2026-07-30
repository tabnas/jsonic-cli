// Copyright (c) 2013-2026 Richard Rodger and other contributors, MIT License

package cli

// parity_test.go — cross-runtime conformance, driven by the shared
// `test/spec/*.tsv` fixtures at the repo root (see ../../test/AGENTS.md), the
// same convention @tabnas/parser and @tabnas/abnf use.
//
// ts/test/parity.test.js discovers and runs the SAME files, so the two
// implementations cannot drift without one of them going red.
//
// Cases that turn on how a runtime loads code or reads the filesystem — the
// `-p` plugin fixtures and the `-f` file fixtures — stay in run_test.go,
// where each runtime's adaptation is spelled out. Everything expressible as
// argv (+ stdin) → stdout lives here.

import (
	"bufio"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

type specRow struct {
	file     string
	lineNo   int
	argv     []string
	expected string
	stdin    string
}

func specDir() string { return filepath.Join("..", "..", "test", "spec") }

// specUnescape decodes the escape set used in non-JSON columns. Kept
// byte-identical to the TS loader so both runtimes feed the CLI the exact
// same stdin text.
func specUnescape(s string) string {
	if !strings.Contains(s, `\`) {
		return s
	}
	var b strings.Builder
	b.Grow(len(s))
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c == '\\' && i+1 < len(s) {
			switch s[i+1] {
			case 'n':
				b.WriteByte('\n')
				i++
				continue
			case 'r':
				b.WriteByte('\r')
				i++
				continue
			case 't':
				b.WriteByte('\t')
				i++
				continue
			case '\\':
				b.WriteByte('\\')
				i++
				continue
			}
		}
		b.WriteByte(c)
	}
	return b.String()
}

func loadSpec(t *testing.T, path string) []specRow {
	t.Helper()
	f, err := os.Open(path)
	if err != nil {
		t.Fatalf("open %s: %v", path, err)
	}
	defer f.Close()

	var rows []specRow
	scanner := bufio.NewScanner(f)
	scanner.Buffer(make([]byte, 0, 64*1024), 1<<20)
	lineNo := 0
	for scanner.Scan() {
		lineNo++
		if lineNo == 1 {
			continue // header naming the columns
		}
		// Strip the CR of a CRLF line: the TS loader splits on /\r?\n/ and
		// drops it, so keeping it here would feed the runtimes different bytes.
		line := strings.TrimSuffix(scanner.Text(), "\r")
		// A comment line starts with '#' and has no tab; a data row always
		// has at least one (argv + stdout), so '#'-leading data still works.
		if line == "" || (strings.HasPrefix(line, "#") && !strings.Contains(line, "\t")) {
			continue
		}
		cols := strings.Split(line, "\t")
		if len(cols) < 2 {
			t.Fatalf("%s:%d: expected at least 2 tab-separated columns", path, lineNo)
		}
		var argv []string
		if err := json.Unmarshal([]byte(cols[0]), &argv); err != nil {
			t.Fatalf("%s:%d: bad argv JSON %q: %v", path, lineNo, cols[0], err)
		}
		row := specRow{
			file:     filepath.Base(path),
			lineNo:   lineNo,
			argv:     argv,
			expected: cols[1],
		}
		if 3 <= len(cols) {
			row.stdin = specUnescape(cols[2])
		}
		rows = append(rows, row)
	}
	if err := scanner.Err(); err != nil {
		t.Fatalf("read %s: %v", path, err)
	}
	if len(rows) == 0 {
		t.Fatalf("%s: no cases", path)
	}
	return rows
}

func runSpecFile(t *testing.T, path string) {
	for _, row := range loadSpec(t, path) {
		t.Run(strings.Join(row.argv, " "), func(t *testing.T) {
			lines := capture(row.argv, row.stdin, nil)
			got := ""
			if 0 < len(lines) {
				got = lines[0]
			}

			// Help text is long and version-ish; those rows pin a substring.
			if strings.HasPrefix(row.expected, "CONTAINS:") {
				want := strings.TrimPrefix(row.expected, "CONTAINS:")
				if !strings.Contains(got, want) {
					t.Errorf("%s:%d: output lacks %q:\n%s", row.file, row.lineNo, want, got)
				}
				return
			}

			var want string
			if err := json.Unmarshal([]byte(row.expected), &want); err != nil {
				t.Fatalf("%s:%d: bad expected JSON %q: %v", row.file, row.lineNo, row.expected, err)
			}
			if got != want {
				t.Errorf("%s:%d:\n  got  %q\n  want %q", row.file, row.lineNo, got, want)
			}
		})
	}
}

// TestSpec auto-discovers every fixture: adding a .tsv runs it in both
// runtimes without touching either runner.
func TestSpec(t *testing.T) {
	files, err := filepath.Glob(filepath.Join(specDir(), "*.tsv"))
	if err != nil {
		t.Fatalf("glob spec dir: %v", err)
	}
	if len(files) == 0 {
		t.Fatalf("no spec files under %s", specDir())
	}
	for _, path := range files {
		t.Run(filepath.Base(path), func(t *testing.T) { runSpecFile(t, path) })
	}
}
