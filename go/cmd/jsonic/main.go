// Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
// contributors, MIT License

// Command jsonic is a JSON parser that isn't strict. It reads relaxed-JSON
// source (from command arguments, --file, or STDIN), parses it with
// github.com/tabnas/jsonic/go, merges the results in precedence, and prints
// standard JSON.
//
// It is the Go port of ts/src/jsonic-cli.ts. The dynamic plugin loader
// (`require`) that the TypeScript CLI uses has no Go equivalent: Go cannot
// load a plugin module by name at runtime. Instead, plugins are looked up
// in a registry of compiled-in plugins keyed by reference name (empty by
// default; tests inject their own). Everything else — argument parsing,
// option/meta wiring, source merging, and JSON serialization — mirrors the
// TypeScript CLI exactly.
package main

import (
	"fmt"
	"io"
	"os"

	debug "github.com/tabnas/debug/go"
	jsonic "github.com/tabnas/jsonic/go"
	tabnas "github.com/tabnas/parser/go"
)

// Version is the current version of the jsonic-cli Go module. Kept in sync
// with ts/package.json by `make publish-go`.
const Version = "0.2.0"

func main() {
	// os.Args[1:] mirrors the TypeScript run()'s argv slice from index 2
	// (it skips node and the script path; we skip the binary path).
	code := run(os.Args[1:], readStdin(), os.Stdout, nil)
	os.Exit(code)
}

// readStdin returns piped STDIN as a string, or "" when STDIN is a TTY
// (mirrors process.stdin.isTTY in the TypeScript read_stdin).
func readStdin() string {
	if fi, err := os.Stdin.Stat(); err == nil && (fi.Mode()&os.ModeCharDevice) != 0 {
		return ""
	}
	b, _ := io.ReadAll(os.Stdin)
	return string(b)
}

// run executes the CLI. argv is the argument list *after* the program name
// (matching the TypeScript run()'s argv.slice(2)). stdin is the STDIN text
// (the test$ override in the TypeScript tests). out receives each output
// "line" — every logf call corresponds to one console.log in the
// TypeScript CLI. plugins is the registry of compiled-in plugins available
// to -p/--plugin (nil for the production binary; tests inject fixtures).
// It returns the process exit code.
func run(argv []string, stdin string, out io.Writer, plugins map[string]tabnas.Plugin) int {
	c := &logger{w: out}
	return runLog(argv, stdin, c, plugins)
}

// logger writes newline-terminated lines and records them for tests.
type logger struct {
	w     io.Writer
	lines []string
}

func (c *logger) logf(s string) {
	c.lines = append(c.lines, s)
	if c.w != nil {
		fmt.Fprintln(c.w, s)
	}
}

// runLog is the core of run, working against a logger so tests can inspect
// the captured lines directly.
func runLog(argv []string, stdin string, c *logger, plugins map[string]tabnas.Plugin) int {
	args := parseArgs(argv)

	// --debug / -d registers the debug plugin and enables a parse trace
	// (the TypeScript equivalent sets meta log=-1; the Go debug plugin is
	// driven by its `trace` option instead).
	usePlugins := map[string]tabnas.Plugin{}
	pluginOpts := map[string]map[string]any{}
	if args.debug {
		usePlugins["debug"] = debug.Debug
		pluginOpts["debug"] = map[string]any{"trace": true}
	}

	if args.help {
		help(c)
		return 0
	}

	// Options and meta are dotted-path property bags whose leaf values are
	// parsed by vanilla jsonic, exactly like handle_props in the TS CLI.
	optionsBag := handleProps(args.options)
	metaBag := handleProps(args.meta)

	// Resolve named plugins from the registry. Unknown names are skipped
	// (the TS CLI throws on a failed require; Go has no dynamic require, so
	// a missing compiled-in plugin is simply unavailable). Plugin options
	// come from options.plugin.<name>.
	for _, name := range args.plugins {
		p, ok := lookupPlugin(plugins, name)
		if !ok {
			fmt.Fprintf(os.Stderr, "Plugin not found: %s\n", name)
			return 1
		}
		usePlugins[name] = p
		if po := pluginOptionMap(optionsBag, name); po != nil {
			pluginOpts[name] = po
		}
	}

	// Build the parser instance from the engine-relevant options, then
	// apply each plugin with its options. EmptyResult is forced to the
	// Undefined sentinel so an empty source parses to Undefined (matching
	// the TypeScript jsonic('') === undefined): the deep-merge below then
	// skips it instead of clobbering accumulated data with null — exactly
	// the TS util.deep(data, {val: undefined}) no-op.
	engOpts := tabnas.MapToOptions(engineOptions(optionsBag))
	engOpts.Lex = mergeEmptyResult(engOpts.Lex)
	j := jsonic.Make(engOpts)
	for name, p := range usePlugins {
		opts := pluginOpts[name]
		if opts == nil {
			opts = map[string]any{}
		}
		if err := j.Use(p, opts); err != nil {
			fmt.Fprintln(os.Stderr, err.Error())
			return 1
		}
	}

	if _, ok := usePlugins["debug"]; ok {
		desc, derr := debug.Describe(j)
		if derr == nil {
			c.logf(desc + "\n=== PARSE ===")
		}
	}

	meta := bagToMeta(metaBag)

	// Merge sources in precedence (highest from the right): files, then
	// STDIN (when no positional sources or `-` was given), then positional
	// sources. The TS CLI seeds data = {val: null} and folds each parsed
	// source in with util.deep(data, {val: parsed}); we mirror that exactly
	// (Deep skips Undefined overlays, so empty sources are no-ops).
	data := map[string]any{"val": nil}
	mergeVal := func(v any) {
		data = tabnas.Deep(data, map[string]any{"val": v}).(map[string]any)
	}

	for _, fp := range args.files {
		if fp == "" {
			continue
		}
		b, err := os.ReadFile(fp)
		if err != nil {
			fmt.Fprintln(os.Stderr, err.Error())
			return 1
		}
		v, perr := j.ParseMeta(string(b), meta)
		if perr != nil {
			fmt.Fprintln(os.Stderr, perr.Error())
			return 1
		}
		mergeVal(v)
	}

	if len(args.sources) == 0 || args.stdin {
		v, perr := j.ParseMeta(stdin, meta)
		if perr != nil {
			fmt.Fprintln(os.Stderr, perr.Error())
			return 1
		}
		mergeVal(v)
	}

	for _, src := range args.sources {
		v, perr := j.ParseMeta(src, meta)
		if perr != nil {
			fmt.Fprintln(os.Stderr, perr.Error())
			return 1
		}
		mergeVal(v)
	}

	// Serialize with JSON.replacer / JSON.space, exactly as the TS CLI does
	// via JSON.stringify(data.val, replacer, space).
	jsonBag, _ := optionsBag["JSON"].(map[string]any)
	replacer := parseReplacer(jsonBag)
	space := parseSpace(jsonBag)

	c.logf(stringify(data["val"], replacer, space))
	return 0
}

// mergeEmptyResult returns lex options with EmptyResult forced to the
// Undefined sentinel, preserving any other lex settings the caller set via
// -o lex.*.
func mergeEmptyResult(lex *tabnas.LexOptions) *tabnas.LexOptions {
	if lex == nil {
		return &tabnas.LexOptions{EmptyResult: tabnas.Undefined}
	}
	lex.EmptyResult = tabnas.Undefined
	return lex
}

// help prints the usage message (the TS help()).
func help(c *logger) {
	c.logf(helpText)
}
