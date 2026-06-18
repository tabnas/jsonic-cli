# Reference: the `jsonic` command

Complete, dry specification of the `jsonic` command-line interface as
implemented in [`ts/src/jsonic-cli.ts`](../src/jsonic-cli.ts). This package
is a CLI — there is no public library API beyond the `run` entry point used
by the binary (documented at the end).

## Synopsis

```
jsonic <args> [<source-text>]*
```

`<source-text>` is relaxed-JSON text parsed into JSON. If no source text is
given, source is read from STDIN. Multiple sources are merged (see
[Source merging](#source-merging)).

## Arguments

| Flag | Alias | Takes value | Effect |
|---|---|---|---|
| `-` | | no | Force STDIN to be read as a source (even when source-text arguments are also given). |
| `--` | | no | Stop flag parsing. Every token after `--` is treated as source text. |
| `--file <file>` | `-f <file>` | yes | Load and parse `<file>` as a source. May be repeated. |
| `--option <name=value>` | `-o <name=value>` | yes | Set option `<name>` (a dotted path) to `<value>`. May be repeated. |
| `--nice` | `-n` | no | Pretty-print output. Exact alias of `-o JSON.space=2`. |
| `--meta <name=value>` | `-m <name=value>` | yes | Set parse meta-data `<name>` (a dotted path) to `<value>`. May be repeated. |
| `--plugin <require>` | `-p <require>` | yes | Load a plugin by module reference (name or path). May be repeated. |
| `--debug` | `-d` | no | Load the `@tabnas/debug` plugin, print a grammar description, and enable a parse trace. Also adds meta `log=-1`. |
| `--help` | `-h` | no | Print the usage message and exit (no parsing). |

Notes:

- **Unknown flags are not errors.** Any `-`-prefixed token that is not one of
  the above (and appears before `--`) is pushed onto the source list and
  parsed as source text. For example `jsonic --not-an-arg a:1` ignores the
  unknown flag (it parses to a harmless value the real source overrides) and
  prints `{"a":1}`.
- A value-taking flag with no following argument simply consumes nothing.

## Source merging

Sources are collected from three places and deep-merged into a single
result. Precedence, highest (wins on conflict) first:

1. **Source-text arguments** — each positional `<source-text>`, in
   left-to-right order (later arguments win over earlier ones).
2. **STDIN** — read when there are no source-text arguments, **or** when `-`
   is present.
3. **`--file` sources** — each `-f`/`--file`, in order (later files win over
   earlier ones).

Internally the result is seeded as `{ val: null }` and each parsed source is
folded in with `util.deep(data, { val: parsed })` (a deep merge). The
implementation applies files first, then STDIN, then arguments, so the
last-applied (arguments) sit highest in precedence.

Empty / whitespace-only source parses to `undefined`, and merging
`undefined` is a no-op, so empty sources never clobber accumulated data.

## Option handling (`-o` / `--option`)

Each `-o name=value`:

- splits once on `=` into `name` and `value`;
- skips the entry if `name` or `value` is empty (`-o ''`, `-o =`, `-o bad=`
  are all no-ops);
- parses `value` with **vanilla `Jsonic(...)`** so the value is typed
  (`-o JSON.space=2` sets the number `2`, `-o JSON.replacer=[b]` sets the
  array `["b"]`);
- assigns it onto a nested options object at the dotted path `name`.

The assembled options object is passed to `Jsonic.make(options)`, so any
`@tabnas/jsonic` engine option is settable (e.g. `-o number.lex=false`).
Two top-level keys are interpreted by the CLI itself rather than the engine:
`JSON` (output serialization) and `plugin` (plugin options).

`-m` / `--meta` works identically, but the resulting object is passed as the
parse *meta-data* argument (`jsonic(source, meta)`) rather than as
constructor options.

## Output serialization

Output is produced by the built-in `JSON.stringify(value, replacer, space)`.
The two stringify arguments are taken from the `JSON` option key:

| Option | Effect |
|---|---|
| `-o JSON.space=<n or string>` | `JSON.stringify` `space` argument. Numbers indent by that many spaces; strings are used verbatim. `--nice`/`-n` sets `2`. |
| `-o JSON.replacer=<key or [keys]>` | `JSON.stringify` `replacer` whitelist. An array keeps only those keys; a single scalar is wrapped into a one-element array. Absent means no filtering. |

Both values are parsed by jsonic first (so `[b]` becomes `["b"]`), then
normalized: a non-array, non-null replacer becomes `[value]`; a null/absent
replacer means "no filter".

## Plugins (`-p` / `--plugin`)

`-p <require>` loads a plugin:

- `<require>` is passed to `require(...)`.
- If a **bare** name (no leading `@`) fails to resolve, it retries
  `require('@tabnas/' + name)` — so `-p csv` finds `@tabnas/csv`.
- Four export shapes are accepted and normalized to the plugin function:
  1. a bare function (`module.exports = fn`),
  2. a `.default` export (`module.exports = { default: fn }`),
  3. a named export matching the CamelCased file basename
     (e.g. file `pa-qa.js` exporting `PaQa`),
  4. a named export matching the lowercase basename
     (e.g. `module.exports = { p2: fn }`).
- A reference whose export is none of these throws
  `Plugin is not a function: <name>`.

Plugin options come from `-o plugin.<name>.<option>=<value>` and are passed
as the plugin's options argument.

## Debug (`-d` / `--debug`)

`-d`:

- installs the `@tabnas/debug` plugin,
- adds meta `log=-1`,
- before parsing, prints `jsonic.debug.describe()` followed by a
  `=== PARSE ===` marker,
- wires `options.debug.get_console` to the active console so the trace is
  captured.

`@tabnas/debug` is a runtime peer dependency; it must resolve for `-d` to
work (npm 7+ installs it automatically).

## STDIN / STDOUT / STDERR contract

- **STDIN** is read (UTF-8) when there are no source-text arguments or when
  `-` is given. A TTY STDIN reads as empty.
- **STDOUT** receives the single serialized JSON line via `console.log`
  (debug output, when enabled, is printed first).
- **STDERR / exit** — the binary catches a rejected `run(...)` and prints
  `e.message` to `console.error`. Parse errors (e.g. malformed source) reject
  with a jsonic error message.

## Exit codes

The TypeScript binary
([`ts/bin/jsonic`](../bin/jsonic)) does not set an explicit exit code: a
successful run exits `0`; an uncaught error in `run(...)` is caught and its
message printed (the process still exits `0` unless Node itself faults).
Compare with the Go port, which returns explicit non-zero codes on errors
(see [go/doc/reference.md](../../go/doc/reference.md)).

## Help text (verbatim summary)

`jsonic -h` / `jsonic --help` prints a usage message beginning:

```
A JSON parser that isn't strict.

Usage: jsonic <args> [<source-text>]*
```

followed by the argument list and worked examples.

## The `run` entry point (internal)

The package's only export is `run`, the testable in-process entry point the
binary calls:

```ts
import { run } from '@tabnas/jsonic-cli'
await run(argv: string[], console: Console): Promise<void>
```

- `argv` mirrors `process.argv`: parsing starts at index `2` (so callers
  pass two placeholder elements before the real arguments, matching
  `[node, script, ...args]`).
- `console` receives output via `console.log`. Two test hooks: a string
  `console.test$` is used as the STDIN body (instead of reading
  `process.stdin`), and `console.log` calls are how callers capture output.

This is an implementation detail used by the test suite, not a stable public
API. The supported interface is the `jsonic` command.

## Out of scope

ABNF / grammar conversion is **not** part of this CLI. It lives in
[`@tabnas/abnf`](https://github.com/tabnas/abnf) as the `tabnas-abnf`
command.
