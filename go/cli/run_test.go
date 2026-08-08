// Copyright (c) 2013-2026 Richard Rodger and other contributors, MIT License

// This is the Go port of ts/test/cli.test.js. Each subtest mirrors one
// assertion (or group) in the TypeScript suite: same inputs, same expected
// stdout. The TypeScript tests call run() in-process with a fake console
// and inspect console.log calls (cn.d.log[0][0]); the Go tests call runLog
// in-process with a capturing logger and inspect logger.lines[0].
//
// Two TypeScript behaviors have no Go equivalent and are adapted rather
// than dropped:
//
//   - Plugin loading by `require(<reference>)`: Go cannot load a module by
//     name at runtime, so the four export-shape fixtures (p0/p1/p2/pa-qa)
//     are provided as compiled-in plugins in a test registry, keyed by the
//     same reference base names the TypeScript tests use ("p0", "p1",
//     "p2", "paqa"). The four plugins all set a value def from their
//     options, exactly like the JS fixtures.
//
//   - Reading source from a real ./test/foo.jsonic file: the Go tests read
//     from testdata/foo.jsonic (committed with the same contents).
package cli

import (
	"strings"
	"testing"

	jsonic "github.com/tabnas/jsonic/go"
	tabnas "github.com/tabnas/parser/go"
)

// capture runs the CLI in-process with the given args, STDIN text, and
// plugin registry, returning the captured output lines.
func capture(argv []string, stdin string, plugins map[string]tabnas.Plugin) []string {
	c := &logger{}
	runLog(argv, stdin, c, plugins)
	return c.lines
}

// valueDefPlugin builds a plugin that registers a single value def whose
// key defaults to defKey (overridable via the plugin's "s" option) and
// whose value is the plugin's <valOpt> option. This is the Go form of the
// p0/p1/p2/pa-qa fixtures (which differ only in default key and option
// name).
func valueDefPlugin(defKey, valOpt string) tabnas.Plugin {
	return func(j *jsonic.Jsonic, opts map[string]any) error {
		key := defKey
		if s, ok := opts["s"].(string); ok && s != "" {
			key = s
		}
		j.SetOptions(tabnas.MapToOptions(map[string]any{
			"value": map[string]any{
				"def": map[string]any{
					key: map[string]any{"val": opts[valOpt]},
				},
			},
		}))
		return nil
	}
}

// testPlugins is the registry the plugin subtests inject, mirroring the
// four JS fixtures by their reference base names.
func testPlugins() map[string]tabnas.Plugin {
	return map[string]tabnas.Plugin{
		"p0":   valueDefPlugin("X", "x"),
		"p1":   valueDefPlugin("Y", "y"),
		"p2":   valueDefPlugin("Z", "z"),
		"paqa": valueDefPlugin("Q", "q"),
	}
}

func TestBasic(t *testing.T) {
	out := capture([]string{"a:1"}, "", nil)
	if out[0] != `{"a":1}` {
		t.Fatalf("got %q", out[0])
	}

	out = capture([]string{"-o", "number.lex=false", "a:1"}, "", nil)
	if out[0] != `{"a":"1"}` {
		t.Fatalf("got %q", out[0])
	}
}

func TestArgs(t *testing.T) {
	// -h / --help print usage.
	out := capture([]string{"-h"}, "", nil)
	if !strings.Contains(out[0], "Usage:") {
		t.Fatalf("-h missing Usage: %q", out[0])
	}
	out = capture([]string{"--help"}, "", nil)
	if !strings.Contains(out[0], "Usage:") {
		t.Fatalf("--help missing Usage: %q", out[0])
	}

	// Multiple positional sources, each parsed and merged.
	out = capture([]string{"a:1", "b:[2]", "c:{x:1}"}, "", nil)
	if out[0] != `{"a":1,"b":[2],"c":{"x":1}}` {
		t.Fatalf("got %q", out[0])
	}

	// --file then a positional source, merged.
	out = capture([]string{"-f", "testdata/foo.jsonic", "zed:2"}, "", nil)
	if out[0] != `{"bar":1,"zed":2}` {
		t.Fatalf("got %q", out[0])
	}

	// Two --file sources, merged (TS has this case commented out due to a
	// jest quirk; the Go port can run it).
	out = capture([]string{"-f", "testdata/foo.jsonic", "--file", "testdata/bar.jsonic"}, "", nil)
	if out[0] != `{"bar":1,"qaz":2}` {
		t.Fatalf("got %q", out[0])
	}

	// An unknown flag is treated as a source (and parses to a harmless
	// string the real source overrides).
	out = capture([]string{"--not-an-arg-so-ignored", "a:1"}, "", nil)
	if out[0] != `{"a":1}` {
		t.Fatalf("got %q", out[0])
	}

	// STDIN only (no positional sources).
	out = capture([]string{}, "{a:1}", nil)
	if out[0] != `{"a":1}` {
		t.Fatalf("got %q", out[0])
	}

	// `-` alias for STDIN.
	out = capture([]string{"-"}, "{a:1}", nil)
	if out[0] != `{"a":1}` {
		t.Fatalf("got %q", out[0])
	}

	// `-` plus a positional source, merged.
	out = capture([]string{"-", "b:2"}, "{a:1}", nil)
	if out[0] != `{"a":1,"b":2}` {
		t.Fatalf("got %q", out[0])
	}
}

func TestBadArgs(t *testing.T) {
	// Empty --file is skipped.
	out := capture([]string{"-f", "", "a:1"}, "", nil)
	if out[0] != `{"a":1}` {
		t.Fatalf("empty -f: got %q", out[0])
	}

	// Empty / malformed -o values are no-ops.
	for _, opt := range []string{"", "=", "bad="} {
		out = capture([]string{"-o", opt, "a:1"}, "", nil)
		if out[0] != `{"a":1}` {
			t.Fatalf("-o %q: got %q", opt, out[0])
		}
	}
}

// captureCode is capture plus the exit code.
func captureCode(argv []string, stdin string, plugins map[string]tabnas.Plugin) ([]string, int) {
	c := &logger{}
	code := runLog(argv, stdin, c, plugins)
	return c.lines, code
}

// TestBuiltinPlugins exercises -p/--plugin against the real compiled-in
// registry (registry.go), not injected fixtures: the shipped binary's
// built-in plugins are debug, jsonic, and json, resolvable by bare name
// and by the @tabnas/<name> form (the TS require fallback).
func TestBuiltinPlugins(t *testing.T) {
	reg := Plugins()
	for _, name := range []string{"debug", "jsonic", "json"} {
		if _, ok := reg[name]; !ok {
			t.Fatalf("built-in plugin %q not registered", name)
		}
	}

	// -p debug loads @tabnas/debug and, like the TS CLI, prints the
	// grammar description before the parse output.
	out, code := captureCode([]string{"-p", "debug", "a:1"}, "", Plugins())
	if code != 0 {
		t.Fatalf("-p debug: exit %d", code)
	}
	if len(out) != 2 || !strings.Contains(out[0], "=== PARSE ===") {
		t.Fatalf("-p debug: missing describe header: %q", out)
	}
	if out[1] != `{"a":1}` {
		t.Fatalf("-p debug: got %q", out[1])
	}

	// The @tabnas/<name> reference form resolves to the same plugin.
	out, code = captureCode([]string{"-p", "@tabnas/debug", "a:1"}, "", Plugins())
	if code != 0 || out[len(out)-1] != `{"a":1}` {
		t.Fatalf("-p @tabnas/debug: exit %d, got %q", code, out)
	}

	// -p jsonic (the grammar plugin itself) is a no-op on the CLI's
	// default instance.
	out, code = captureCode([]string{"-p", "jsonic", "a:1"}, "", Plugins())
	if code != 0 || out[0] != `{"a":1}` {
		t.Fatalf("-p jsonic: exit %d, got %q", code, out)
	}

	// -p json restricts the parser to strict JSON.
	out, code = captureCode([]string{"-p", "json", `{"a":1}`}, "", Plugins())
	if code != 0 || out[0] != `{"a":1}` {
		t.Fatalf("-p json strict: exit %d, got %q", code, out)
	}
	_, code = captureCode([]string{"-p", "json", "a:1"}, "", Plugins())
	if code != 1 {
		t.Fatalf("-p json with relaxed source: exit %d, want 1", code)
	}

	// An unknown plugin reference is an error (TS: require throws).
	_, code = captureCode([]string{"-p", "no-such-plugin", "a:1"}, "", Plugins())
	if code != 1 {
		t.Fatalf("-p no-such-plugin: exit %d, want 1", code)
	}

	// RegisterPlugin extends the registry for custom binaries; Run merges
	// per-run extras over it.
	RegisterPlugin("cli-test-fixture", valueDefPlugin("F", "f"))
	out, code = captureCode(
		[]string{"-p", "cli-test-fixture", "-o", "plugin.cli-test-fixture.f=9", "a:F"},
		"", Plugins())
	if code != 0 || out[0] != `{"a":9}` {
		t.Fatalf("registered fixture: exit %d, got %q", code, out)
	}
}

// TestDebugFlag mirrors the TypeScript `debug` subtest: -d/--debug installs
// @tabnas/debug, prints the grammar description ahead of the parse, and
// still emits the JSON result last. The trace volume is engine-dependent,
// so pin the two ends, not a line count.
func TestDebugFlag(t *testing.T) {
	for _, flag := range []string{"-d", "--debug"} {
		out, code := captureCode([]string{flag, "a:1"}, "", nil)
		if code != 0 {
			t.Fatalf("%s: exit %d", flag, code)
		}
		if len(out) < 2 || !strings.Contains(out[0], "=== PARSE ===") {
			t.Fatalf("%s: missing describe header: %q", flag, out)
		}
		if out[len(out)-1] != `{"a":1}` {
			t.Fatalf("%s: got %q", flag, out[len(out)-1])
		}
	}

	// --debug with engine options and `--` ending flag parsing (the TS
	// `debug` subtest's second case): value.lex=false keeps `true` a string.
	out, code := captureCode([]string{
		"--debug", "-o", "debug.maxlen=11",
		"--option", "value.lex=false", "--", "a:true",
	}, "", nil)
	if code != 0 {
		t.Fatalf("--debug with options: exit %d", code)
	}
	if !strings.Contains(out[0], "=== PARSE ===") {
		t.Fatalf("--debug with options: missing describe header: %q", out[0])
	}
	if out[len(out)-1] != `{"a":"true"}` {
		t.Fatalf("--debug with options: got %q", out[len(out)-1])
	}
}

func TestPlugin(t *testing.T) {
	pl := testPlugins()

	// p0: value def X -> 0.
	out := capture([]string{"-p", "p0", "-o", "plugin.p0.x=0", "a:X"}, "", pl)
	if out[0] != `{"a":0}` {
		t.Fatalf("p0: got %q", out[0])
	}

	// p0 with an overridden key W.
	out = capture([]string{"-p", "p0", "-o", "plugin.p0.x=0", "-o", "plugin.p0.s=W", "a:W"}, "", pl)
	if out[0] != `{"a":0}` {
		t.Fatalf("p0/s=W: got %q", out[0])
	}

	// p1 (the .default-export shape in JS): value def Y -> 1.
	out = capture([]string{"-o", "plugin.p1.y=1", "-p", "p1", "a:Y"}, "", pl)
	if out[0] != `{"a":1}` {
		t.Fatalf("p1: got %q", out[0])
	}

	// p0 and p1 together.
	out = capture([]string{
		"-o", "plugin.p0.x=0", "-p", "p0",
		"-o", "plugin.p1.y=1", "-p", "p1", "a:X,b:Y",
	}, "", pl)
	if out[0] != `{"a":0,"b":1}` {
		t.Fatalf("p0+p1: got %q", out[0])
	}

	// p2 (the named-export shape): value def Z -> 2.
	out = capture([]string{"-p", "p2", "-o", "plugin.p2.z=2", "a:Z"}, "", pl)
	if out[0] != `{"a":2}` {
		t.Fatalf("p2: got %q", out[0])
	}

	// pa-qa (the CamelCased PaQa shape): value def Q -> 3.
	out = capture([]string{"-p", "paqa", "-o", "plugin.paqa.q=3", "a:Q"}, "", pl)
	if out[0] != `{"a":3}` {
		t.Fatalf("paqa: got %q", out[0])
	}
}

func TestStringify(t *testing.T) {
	// JSON.space=2 indents.
	out := capture([]string{"-o", "JSON.space=2", "a:1"}, "", nil)
	if out[0] != "{\n  \"a\": 1\n}" {
		t.Fatalf("space=2: got %q", out[0])
	}

	// -n is an alias for JSON.space=2.
	out = capture([]string{"-n", "a:1"}, "", nil)
	if out[0] != "{\n  \"a\": 1\n}" {
		t.Fatalf("-n: got %q", out[0])
	}

	// JSON.replacer as an array of keys.
	out = capture([]string{"-o", "JSON.replacer=[b]", "a:1,b:2"}, "", nil)
	if out[0] != `{"b":2}` {
		t.Fatalf("replacer=[b]: got %q", out[0])
	}

	// JSON.replacer as a scalar key (wrapped to a single-element whitelist).
	out = capture([]string{"-o", "JSON.replacer=b", "a:1,b:2"}, "", nil)
	if out[0] != `{"b":2}` {
		t.Fatalf("replacer=b: got %q", out[0])
	}
}
