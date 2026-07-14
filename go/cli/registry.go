// Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
// contributors, MIT License

package cli

import (
	"sync"

	debug "github.com/tabnas/debug/go"
	tjson "github.com/tabnas/json/go"
	jsonic "github.com/tabnas/jsonic/go"
	tabnas "github.com/tabnas/parser/go"
)

// The compiled-in plugin registry backing -p/--plugin. The TypeScript CLI
// require()s plugin modules at runtime; Go cannot load code dynamically,
// so plugin reference names resolve against this registry instead. Names
// resolve with the same fallbacks as the TS CLI's require chain: the bare
// name, the base name of a path-like reference, and the tail of an
// `@tabnas/<name>` reference (see lookupPlugin in args.go).

var (
	registryMu sync.Mutex
	registry   = map[string]tabnas.Plugin{}
)

// RegisterPlugin makes a plugin available to -p/--plugin under the given
// reference name. Register with the plain name (e.g. "csv"); the lookup
// also accepts the "@tabnas/csv" form. Registering an existing name
// replaces it. Safe for concurrent use; call it before Run (typically
// from a custom binary's main, or an init function):
//
//	cli.RegisterPlugin("csv", csv.Csv)
//	os.Exit(cli.Run(os.Args[1:], cli.ReadStdin(), os.Stdout, nil))
func RegisterPlugin(name string, plugin tabnas.Plugin) {
	registryMu.Lock()
	defer registryMu.Unlock()
	registry[name] = plugin
}

// Plugins returns a snapshot of the registered plugins (built-in and
// caller-registered), keyed by reference name.
func Plugins() map[string]tabnas.Plugin {
	registryMu.Lock()
	defer registryMu.Unlock()
	out := make(map[string]tabnas.Plugin, len(registry))
	for name, p := range registry {
		out[name] = p
	}
	return out
}

// The plugins already present in this module's dependency graph are
// pre-registered, so the standard binary supports them out of the box:
//
//	debug  — @tabnas/debug: parse tracing and grammar description
//	         (options: trace, describe, ...)
//	jsonic — the relaxed-JSON grammar plugin itself (a no-op on the
//	         CLI's default instance, which already has it; useful with
//	         plugins that layer on the jsonic rules)
//	json   — @tabnas/json: restrict the parser to strict JSON
//
// Other @tabnas grammar plugins (csv, toml, directive, multisource, ...)
// live in modules that depend on jsonic; requiring them here would bloat
// the CLI's dependency graph, so they are left to custom binaries via
// RegisterPlugin.
func init() {
	RegisterPlugin("debug", debug.Debug)
	RegisterPlugin("jsonic", jsonic.Grammar)
	RegisterPlugin("json", tjson.Json)
}
