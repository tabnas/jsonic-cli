// Copyright (c) 2013-2026 Richard Rodger and other contributors, MIT License

// divergence_test.go — the recorded divergences from the canonical
// TypeScript command, for the Go port.
//
// ../../test/divergent.tsv is the executable register: one row per input
// where a port produces a different RESULT, one column per runtime. This
// suite runs it through tabnassupport.Register and asserts the `go`
// column; rs/tests/divergence_test.rs asserts the `rust` one. The
// property that matters is not that a regression fails but that a FIX
// fails too: when Go is repaired to agree, the register still claims a
// difference, the suite goes red and names the row to delete. That is
// what keeps the record from outliving the divergence, which a prose list
// cannot do — ../../DIVERGENCE.md describes each row and points here.
//
// One limitation, stated rather than discovered later: runLog reports a
// failure on os.Stderr and answers the caller with an exit code alone, so
// this runner can see THAT a run failed and not what it said. No `go`
// cell records a failure today. A row that needs one (an `ERROR:<code>`
// cell) needs run.go to report through the logger first.
package cli

import (
	"encoding/json"
	"fmt"
	"path/filepath"
	"testing"

	support "github.com/tabnas/support/go"
)

// TestDivergenceRegister runs every row of the register and asserts the
// `go` column.
func TestDivergenceRegister(t *testing.T) {
	dir, err := support.FindSpecDir("")
	if err != nil {
		t.Fatal(err)
	}
	// The register sits BESIDE spec/, not inside it: everything in spec/
	// is discovered and run by all three suites, and a row one of them
	// cannot pass would break that suite.
	path := filepath.Join(filepath.Dir(dir), "divergent.tsv")

	support.Register{
		Runner: support.Runner{
			ParseRow: func(input string, row *support.Row) (any, error) {
				var argv []string
				if err := json.Unmarshal([]byte(input), &argv); err != nil {
					return nil, fmt.Errorf("argv is not a JSON array: %w", err)
				}

				c := &logger{}
				if code := runLog(argv, row.UnescNamed("stdin"), c, nil); code != 0 {
					return nil, fmt.Errorf("the command exited %d", code)
				}
				if len(c.lines) == 0 {
					return "", nil
				}
				return c.lines[0], nil
			},
			ParseExpected: func(expected string, _ *support.Row) (any, error) {
				return support.ParseExpect(expected)
			},
			InputName: "argv",
		},
		Runtime:  "go",
		Runtimes: []string{"ts", "go", "rust"},
	}.File(t, path)
}
