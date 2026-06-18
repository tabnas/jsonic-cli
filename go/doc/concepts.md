# Concepts: how the Go `jsonic` command works, and why

This explains the design of the **Go port** of the `jsonic` command — what
it is, how it relates to the parser engine, and where it deliberately
diverges from the canonical TypeScript CLI. For the *what* see
[reference.md](reference.md); for step-by-step learning see
[tutorial.md](tutorial.md).

## What the CLI actually is

The Go `jsonic` command is a **thin wrapper around
`github.com/tabnas/jsonic/go`** — the Go port of the relaxed-JSON parser
engine. It contains no grammar, no lexer, no parsing rules of its own. All
parsing is the engine's. The command's job is the plumbing around it:

1. parse the command line into arguments, options, meta, files, plugins;
2. build a configured engine instance;
3. read source from arguments, STDIN, and files;
4. deep-merge the parsed results into one value;
5. serialize that value to standard JSON and print it.

It is a `package main` program — a tool, not a library. There is no exported
Go API; you drive it as a command.

## The engine relationship

The flow inside `runLog()` mirrors the TS `run()`:

```
options ─▶ MapToOptions ─▶ jsonic.Make(opts) ─▶ engine instance
                                                     │
source text ─────────────────────────────────────▶ j.ParseMeta(text, meta) ─▶ value
                                                     │
                                                     ▼
                                  stringify(value, replacer, space) ─▶ stdout
```

- **`-o` options** become the constructor options for `jsonic.Make(...)`,
  with each leaf value parsed by `jsonic.Parse(...)` so types come through
  (`-o number.lex=false` sets the boolean `false`).
- **`-m` meta** becomes the per-parse meta map for `ParseMeta`.
- **The parsed result** is whatever the engine returns; the CLI does not
  interpret it, only serializes it.

As in TS, this couples arg-value parsing to the engine working — an accepted
trade-off for a tool whose purpose is to expose that engine.

## Relaxed in, strict out

Input is parsed by jsonic's relaxed grammar; output is canonical JSON
produced by a faithful port of the browser `JSON.stringify`. So `jsonic`
normalizes hand-written, forgiving text into machine-readable JSON. The
serializer exposes `JSON.stringify`'s own `replacer` and `space` controls
(via `-o JSON.replacer=...` / `-o JSON.space=...`) rather than inventing
formatting flags; `-n` is sugar for `-o JSON.space=2`.

## Why sources merge (and the precedence order)

Layering — a base file, piped overrides, last-minute argument tweaks — is the
motivating use case. All three source kinds are accepted and deep-merged.
The result is seeded `{"val": nil}` and each parsed source folded in with
`tabnas.Deep`. Files are applied first, then STDIN, then arguments, making
arguments highest precedence (most-immediate-input-wins). Empty sources
parse to the Undefined sentinel, which `Deep` skips, so they never erase
data; `-` exists to force STDIN in alongside arguments.

## Debugging is "just a plugin"

`-d`/`--debug` installs the first-party `github.com/tabnas/debug/go` plugin,
prints the engine's own grammar description (`debug.Describe(j)`), and turns
on a trace. The diagnostics are the engine's, not a reimplementation that
could drift.

## Testability as a design constraint

`run(argv, stdin, out, plugins)` and its core `runLog(argv, stdin, logger,
plugins)` take their inputs and output sink as parameters rather than
reaching for globals. The tests call `runLog` in-process with a capturing
`logger` and inspect `logger.lines[0]` — exactly mirroring the TS suite,
which calls `run` with a fake console and reads `cn.d.log[0][0]`. The
`main()` entry is a thin shim that supplies `os.Args[1:]`, real STDIN, and
`os.Stdout`.

## Differences from the TS version

The Go port tracks the TypeScript CLI's *behaviour* — same flags, same
stdout for the same inputs (`main_test.go` ports `cli.test.js`
one-for-one). The differences are structural, forced by the language:

- **Plugin loading.** The biggest divergence. The TS CLI loads
  `-p`/`--plugin` modules dynamically with `require(<reference>)` (retrying
  the `@tabnas/` scope and normalizing four export shapes). **Go cannot load
  a module by name at runtime.** So the Go CLI resolves plugins from a
  **compiled-in registry** passed into `run`. The production binary passes a
  `nil` registry, so naming any plugin prints `Plugin not found: <name>` and
  exits `1`; the tests inject the four fixture plugins as native Go
  functions keyed by reference name. To ship a plugin you must compile it
  in, not name it on the command line.

- **Debug driver.** Both install a debug plugin and print a description, but
  the Go debug plugin is driven by its `trace` option (which `-d` sets to
  `true`), whereas the TS CLI uses meta `log=-1`. For parity the Go arg loop
  still appends `log=-1` to meta, but it has no Go engine meaning and is
  harmless.

- **Exit codes.** The TS binary does not set an explicit exit code (a caught
  `run` rejection just prints its message). The Go `main` calls
  `os.Exit(run(...))`, returning `1` on a missing plugin, an unreadable
  `--file`, a `Use` failure, or a parse error — and `0` otherwise.

- **Serialization is a hand-written port.** TS calls the built-in
  `JSON.stringify`. Go reimplements it in `stringify.go` (replacer
  whitelist, space indent, number/string escaping). One observable
  consequence: the engine's parse result is an unordered `map[string]any`,
  so the Go serializer emits **object keys in sorted order**. The TS side
  preserves the engine's key order. For the simple objects in the test
  suite the two agree, but key ordering is the place to watch if outputs
  ever differ.

- **Empty-result wiring.** The Go CLI explicitly forces the engine's
  `Lex.EmptyResult` to the Undefined sentinel so empty source parses to
  Undefined (the TS engine returns `undefined` for `jsonic('')` natively).
  This is what makes empty-source merges no-ops on both sides.

- **No dynamic types / `console` injection.** TS injects a fake `console`
  (including a `test$` STDIN hook); Go passes an `io.Writer` and a `stdin`
  string explicitly. Same effect (in-process, deterministic tests), different
  mechanism.

## What is deliberately *not* here

- **No grammar.** No `.jsonic` grammar, no railroad diagram, no engine code
  — the parsing rules live in `github.com/tabnas/jsonic/go`.
- **No ABNF / BNF conversion.** That is a separate package,
  [`@tabnas/abnf`](https://github.com/tabnas/abnf) (the `tabnas-abnf`
  command). The only binary here is `jsonic`.

## See also

- [tutorial.md](tutorial.md) — zero to working result.
- [guide.md](guide.md) — task recipes.
- [reference.md](reference.md) — exact flags, exit codes, and contract.
- The canonical TypeScript docs: [../../ts/doc/](../../ts/doc/).
