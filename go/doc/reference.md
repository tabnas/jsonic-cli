# Reference: the Go `jsonic` command

Complete, dry specification of the **Go port** of the `jsonic` CLI. The
command entry point is [`go/cmd/jsonic`](../cmd/jsonic), a thin wrapper
around the [`go/cli`](../cli) library package, whose exported API
(`cli.Run`, `cli.ReadStdin`, `cli.RegisterPlugin`, `cli.Plugins`) lets
custom binaries ship additional compiled-in plugins. The import paths are
`github.com/tabnas/jsonic-cli/go/cli` (library) and
`github.com/tabnas/jsonic-cli/go/cmd/jsonic` (command).

## Synopsis

```
jsonic <args> [<source-text>]*
```

Built / run as:

```bash
go run ./cmd/jsonic <args> [<source-text>]*
# or, after `go install .../cmd/jsonic@latest`:
jsonic <args> [<source-text>]*
```

`<source-text>` is relaxed-JSON text parsed into JSON. With no source text,
source is read from STDIN. Multiple sources are merged (see
[Source merging](#source-merging)).

## Arguments

Identical surface to the TypeScript CLI (`go/cli/args.go` ports the
arg loop exactly):

| Flag | Alias | Takes value | Effect |
|---|---|---|---|
| `-` | | no | Force STDIN to be read as a source. |
| `--` | | no | Stop flag parsing; following tokens are source text. |
| `--file <file>` | `-f <file>` | yes | Load and parse `<file>`. May be repeated. |
| `--option <name=value>` | `-o <name=value>` | yes | Set option `<name>` (dotted path) to `<value>`. May be repeated. |
| `--nice` | `-n` | no | Pretty-print. Alias of `-o JSON.space=2`. |
| `--meta <name=value>` | `-m <name=value>` | yes | Set parse meta-data (dotted path). May be repeated. |
| `--plugin <require>` | `-p <require>` | yes | Resolve a plugin by reference from the compiled-in registry. May be repeated. **See [Plugins](#plugins).** |
| `--debug` | `-d` | no | Install the `github.com/tabnas/debug/go` plugin, print a grammar description, enable a parse trace. |
| `--help` | `-h` | no | Print usage and exit `0`. |

Notes:

- **Unknown flags are not errors.** A `-`-prefixed token that is not a known
  flag (before `--`) becomes source text (matching TS). A value-taking flag
  with no following argument consumes nothing.

## Source merging

Sources are deep-merged into one result. Precedence, highest (wins) first:

1. **Source-text arguments**, left to right (later wins).
2. **STDIN**, read when there are no source-text arguments **or** `-` is
   present.
3. **`--file` sources**, in order (later wins).

The result is seeded as `{"val": nil}` and each parsed source is folded in
with `tabnas.Deep(data, {"val": parsed})`. Files are applied first, then
STDIN, then arguments — so arguments sit highest.

Empty / whitespace-only source parses to the engine's Undefined sentinel,
which `Deep` skips, so empty sources never clobber accumulated data. When
*every* source is empty, the seed `val` stays `nil` and the output is `null`
(`stringify(nil)` → `null`).

## Option handling (`-o` / `--option`)

Each `-o name=value` (`handleProps` in `args.go`):

- splits on the first `=` into `name` and `value` (text after a second `=`
  is discarded, matching the TS `split(/=/)` reading of `pv[0]`/`pv[1]`);
- skips the entry if `name` or `value` is empty (`-o ''`, `-o =`, `-o bad=`
  are no-ops);
- parses `value` with vanilla `jsonic.Parse(...)` for a typed value;
- assigns it at the dotted path `name` in a nested map.

Engine-relevant options (everything except the CLI-only `JSON` and `plugin`
keys) are converted with `tabnas.MapToOptions(...)` and passed to
`jsonic.Make(...)`. The CLI additionally forces the engine's
`Lex.EmptyResult` to the Undefined sentinel so empty source parses to
Undefined (matching `jsonic('') === undefined` in TS).

`-m` / `--meta` is handled identically but passed as the per-parse meta map
to `ParseMeta(source, meta)`.

## Output serialization

Output is produced by a faithful port of
`JSON.stringify(value, replacer, space)` in
[`go/cli/stringify.go`](../cli/stringify.go):

| Option | Effect |
|---|---|
| `-o JSON.space=<n or string>` | Indent. A number N is clamped to 0..10 spaces; a string is used verbatim (first 10 chars). `-n` sets `2`. |
| `-o JSON.replacer=<key or [keys]>` | Key whitelist. An array keeps only those keys (recursively at every object level); a single scalar is wrapped to a one-element list. Absent means no filtering. Array *elements* are never filtered, only object keys. |

Object keys are emitted in **sorted order** (the engine's parse result is an
unordered map, so there is no insertion order to preserve). Numbers,
strings, escaping, and non-finite handling mirror `JSON.stringify`.

## Plugins

This is the principal divergence from the TypeScript CLI.

- `-p <require>` resolves the reference against a **compiled-in registry**
  (`cli/registry.go`). Go cannot load a module by name at runtime, so
  there is no dynamic `require`.
- The shipped binary has these plugins **built in**:

  | Name | Plugin | Effect |
  |---|---|---|
  | `debug` | `github.com/tabnas/debug/go`'s `Debug` | Grammar description + parse tracing (options: `-o plugin.debug.trace=...`). |
  | `jsonic` | `github.com/tabnas/jsonic/go`'s `Grammar` | The relaxed-JSON grammar plugin itself (a no-op on the CLI's default instance). |
  | `json` | `github.com/tabnas/json/go`'s `Json` | Restrict parsing to strict JSON. |

- A reference that is not in the registry fails:

  ```
  Plugin not found: <name>
  ```

  and the command exits **`1`**.
- Resolution tries the bare name, then the base name of a path-like
  reference (`../test/p0` → `p0`, stripping a `.js` suffix), then a
  `@tabnas/`-stripped tail (`@tabnas/json` → `json`) — mirroring the
  spirit of the TS scope fallback.
- Plugin options come from `-o plugin.<name>.<option>=<value>`.

To ship any other plugin, build a **custom binary** that registers it via
`cli.RegisterPlugin` before calling `cli.Run`:

```go
package main

import (
	"os"

	csv "github.com/tabnas/csv/go"
	cli "github.com/tabnas/jsonic-cli/go/cli"
)

func main() {
	cli.RegisterPlugin("csv", csv.Csv)
	os.Exit(cli.Run(os.Args[1:], cli.ReadStdin(), os.Stdout, nil))
}
```

`cli.Run` also accepts a per-run `extra map[string]tabnas.Plugin` argument
merged over the registry (same-name entries win); the test suite uses the
underlying registry parameter to inject fixtures (see `run_test.go`).

## Debug (`-d` / `--debug`)

`-d` installs `debug.Debug` (the `github.com/tabnas/debug/go` plugin) with
its `trace` option set to `true`, and prints `debug.Describe(j)` followed by
`=== PARSE ===` before the parse. (The TS CLI instead sets meta `log=-1`;
the arg loop still appends `log=-1` to meta for parity, but the Go debug
plugin is driven by the `trace` option.)

## STDIN / STDOUT / STDERR contract

- **STDIN** is read when there are no source-text arguments or `-` is given.
  A character-device (TTY) STDIN reads as empty (`cli.ReadStdin` checks
  `os.ModeCharDevice`).
- **STDOUT** receives each output line via `fmt.Fprintln` (the final
  serialized JSON; debug output, when enabled, first).
- **STDERR** receives error messages: a missing plugin
  (`Plugin not found: ...`), a file read error, a `Use` error, or a parse
  error.

## Exit codes

Unlike the TS binary (which does not set an explicit code), the Go `main`
calls `os.Exit(cli.Run(...))` with the code `cli.Run` returns:

| Code | When |
|---|---|
| `0` | Success, or `--help`. |
| `1` | A named plugin is not in the registry; a `--file` cannot be read; a plugin `Use` fails; a parse fails. |

## Help text

`jsonic -h` / `jsonic --help` prints usage beginning:

```
A JSON parser that isn't strict.

Usage: jsonic <args> [<source-text>]*
```

The help text mirrors the TS `help()` verbatim, including the dynamic-require
plugin examples (retained for parity even though only compiled-in plugins
resolve in Go).

## Module layout

| File | Responsibility |
|---|---|
| `cmd/jsonic/main.go` | Entry point: `cli.Run(os.Args[1:], cli.ReadStdin(), os.Stdout, nil)`, `const Version`. |
| `cli/run.go` | `Run`/`runLog` (arg→option/meta/plugin wiring, source merge, serialize), `ReadStdin`. |
| `cli/registry.go` | Compiled-in plugin registry: `RegisterPlugin`, `Plugins`, built-ins (`debug`, `jsonic`, `json`). |
| `cli/args.go` | `parseArgs`, `handleProps`, dotted-path prop bags, `lookupPlugin`. |
| `cli/stringify.go` | `JSON.stringify(value, replacer, space)` port. |
| `cli/help.go` | `helpText`. |
| `cli/run_test.go` | Port of `ts/test/cli.test.js`, plus built-in registry tests. |
| `cli/testdata/{foo,bar}.jsonic` | `--file` fixtures. |

## Out of scope

ABNF / grammar conversion is **not** part of this CLI; it lives in
[`@tabnas/abnf`](https://github.com/tabnas/abnf) (the `tabnas-abnf`
command).
