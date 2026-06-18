// Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
// contributors, MIT License

package main

import (
	"strings"

	jsonic "github.com/tabnas/jsonic/go"
	tabnas "github.com/tabnas/parser/go"
)

// cliArgs holds the parsed command line, matching the TypeScript `args`
// object.
type cliArgs struct {
	help    bool
	stdin   bool
	debug   bool
	sources []string
	files   []string
	options []string
	meta    []string
	plugins []string
}

// parseArgs replicates the TypeScript argument loop exactly: a leading "-"
// is STDIN, "--" stops flag parsing, the value-taking flags consume the
// next argument, and any other "-" prefixed token before "--" is ignored
// as an unknown flag (note: in the TS CLI it is pushed to sources, so we
// match that — an unknown flag becomes a source, which jsonic then parses;
// the --not-an-arg-so-ignored test relies on this being harmless when a
// real source follows). To match TS, an unrecognized "-" token is added to
// sources.
func parseArgs(argv []string) cliArgs {
	var args cliArgs
	acceptArgs := true

	for i := 0; i < len(argv); i++ {
		arg := argv[i]

		if acceptArgs && strings.HasPrefix(arg, "-") {
			switch {
			case arg == "-":
				args.stdin = true
			case arg == "--":
				acceptArgs = false
			case arg == "--file" || arg == "-f":
				if i+1 < len(argv) {
					i++
					args.files = append(args.files, argv[i])
				}
			case arg == "--option" || arg == "-o":
				if i+1 < len(argv) {
					i++
					args.options = append(args.options, argv[i])
				}
			case arg == "--meta" || arg == "-m":
				if i+1 < len(argv) {
					i++
					args.meta = append(args.meta, argv[i])
				}
			case arg == "--debug" || arg == "-d":
				args.debug = true
				args.meta = append(args.meta, "log=-1")
			case arg == "--help" || arg == "-h":
				args.help = true
			case arg == "--plugin" || arg == "-p":
				if i+1 < len(argv) {
					i++
					args.plugins = append(args.plugins, argv[i])
				}
			case arg == "--nice" || arg == "-n":
				args.options = append(args.options, "JSON.space=2")
			default:
				args.sources = append(args.sources, arg)
			}
		} else {
			args.sources = append(args.sources, arg)
		}
	}

	return args
}

// handleProps replicates the TypeScript handle_props: each "name=value"
// string sets a dotted-path property in a bag, with the value parsed by
// vanilla jsonic. Entries with an empty name or empty value are skipped
// (the bad-args tests: "", "=", "bad=" are all no-ops).
func handleProps(propvals []string) map[string]any {
	out := map[string]any{}
	for _, propval := range propvals {
		// split(/=/) in TS splits on every "=", but only pv[0] and pv[1]
		// are used; everything after the first "=" is discarded. SplitN
		// here keeps pv[1] as the remainder, but the TS code reads pv[1]
		// (the segment between the first and second "="). Match TS: take
		// the text up to the second "=".
		name, value, hadEq := cut(propval, "=")
		if !hadEq {
			continue
		}
		// pv[1] in TS is the segment up to the next "=".
		value, _, _ = cut(value, "=")
		if name == "" || value == "" {
			continue
		}
		v, err := jsonic.Parse(value)
		if err != nil {
			continue
		}
		setProp(out, name, v)
	}
	return out
}

// cut is strings.Cut (Go 1.18+), spelled out for clarity of intent.
func cut(s, sep string) (before, after string, found bool) {
	return strings.Cut(s, sep)
}

// setProp sets a dotted-path property in a nested map[string]any bag,
// creating intermediate maps as needed. Mirrors tabnas util.prop /
// the TS util.prop.
func setProp(bag map[string]any, path string, val any) {
	parts := strings.Split(path, ".")
	cur := bag
	for i := 0; i < len(parts)-1; i++ {
		k := parts[i]
		next, ok := cur[k].(map[string]any)
		if !ok {
			next = map[string]any{}
			cur[k] = next
		}
		cur = next
	}
	cur[parts[len(parts)-1]] = val
}

// engineOptions returns the subset of the option bag that configures the
// engine, dropping the CLI-only JSON and plugin keys (which the engine
// would not understand). MapToOptions ignores unknown keys, but dropping
// them keeps intent clear and avoids any future surprise.
func engineOptions(bag map[string]any) map[string]any {
	out := map[string]any{}
	for k, v := range bag {
		if k == "JSON" || k == "plugin" {
			continue
		}
		out[k] = v
	}
	return out
}

// pluginOptionMap returns the options.plugin.<name> sub-bag, or nil.
func pluginOptionMap(bag map[string]any, name string) map[string]any {
	pl, ok := bag["plugin"].(map[string]any)
	if !ok {
		return nil
	}
	po, ok := pl[name].(map[string]any)
	if !ok {
		return nil
	}
	return po
}

// lookupPlugin resolves a plugin reference name against the registry,
// trying the bare name first and then a "@tabnas/"-stripped tail, matching
// the spirit of the TS handle_plugins fallback (require(name) then
// require('@tabnas/'+name)). Registry keys are plain reference names
// (e.g. "csv"); a path-like reference is reduced to its base name.
func lookupPlugin(registry map[string]tabnas.Plugin, name string) (tabnas.Plugin, bool) {
	if registry == nil {
		return nil, false
	}
	if p, ok := registry[name]; ok {
		return p, true
	}
	// Try the base name of a path-like reference (e.g. "../test/p0" -> "p0").
	base := name
	if i := strings.LastIndexAny(base, "/\\"); i >= 0 {
		base = base[i+1:]
	}
	base = strings.TrimSuffix(base, ".js")
	if p, ok := registry[base]; ok {
		return p, true
	}
	// Try @tabnas/-prefixed reference stripped to its tail.
	if strings.HasPrefix(name, "@tabnas/") {
		tail := strings.TrimPrefix(name, "@tabnas/")
		if p, ok := registry[tail]; ok {
			return p, true
		}
	}
	return nil, false
}

// bagToMeta converts the meta bag into the engine's meta map. The engine's
// ParseMeta takes a map[string]any directly. The CLI-injected "log" key
// (from --debug) has no Go engine meaning and is harmless to pass through.
func bagToMeta(bag map[string]any) map[string]any {
	if len(bag) == 0 {
		return nil
	}
	return bag
}
