# Concepts: how `jsonic` works, and why

This explains the design of the `jsonic` command — what it is, how it
relates to the parser engine, and the trade-offs behind its behaviour. For
the *what* (flags, contract) see [reference.md](reference.md); for
step-by-step learning see [tutorial.md](tutorial.md).

## What the CLI actually is

`@tabnas/jsonic-cli` is a **thin wrapper around
[`@tabnas/jsonic`](https://github.com/tabnas/jsonic)**. It contains no
grammar, no lexer, no parsing rules of its own. Every byte of relaxed-JSON
parsing happens inside the `@tabnas/jsonic` engine. The CLI's entire job is
the plumbing *around* that engine:

1. parse the command line into arguments, options, meta, files, and plugins;
2. build a configured jsonic instance;
3. read source text from arguments, STDIN, and files;
4. merge the parsed results into one value;
5. serialize that value to standard JSON and print it.

Think of it as relaxed-JSON's counterpart to a `cat`-plus-`JSON.parse`
pipeline, with merging and serialization controls bolted on.

## The engine relationship

The flow inside `run()` is direct:

```
options ─▶ Jsonic.make(options) ─▶ jsonic instance
                                        │
source text ────────────────────────▶ jsonic(text, meta) ─▶ value
                                        │
                                        ▼
                              JSON.stringify(value, ...) ─▶ stdout
```

- **`-o` options** become the constructor options for `Jsonic.make(...)`.
  Because they are parsed by jsonic itself, you get real typed values
  (`-o number.lex=false` sets the boolean `false`, not the string
  `"false"`). This is deliberate leverage: the CLI does not reimplement a
  type system, it borrows the engine's.
- **`-m` meta** becomes the per-parse meta argument `jsonic(text, meta)`.
- **The parsed result** is whatever the engine returns — an object, array,
  number, string, etc. The CLI does not interpret it.

The cost of this leverage is a coupling the source code calls out: arg-value
parsing depends on core jsonic working. If the engine breaks, so does
`-o`/`-m`. That is an accepted trade-off for a tool whose whole purpose is to
expose that engine.

## Relaxed in, strict out

The asymmetry is the point. Input is parsed by jsonic's relaxed grammar
(optional quotes, optional top-level braces, trailing commas, comments,
and more). Output is produced by the *standard* `JSON.stringify`. So
`jsonic` is a normalizer: hand-written, forgiving text in; canonical,
machine-readable JSON out.

Serialization is intentionally the *built-in* `JSON.stringify`, not a custom
serializer. That keeps output 100% standards-compliant and lets the CLI
expose `JSON.stringify`'s own `replacer` and `space` arguments directly
(via `-o JSON.replacer=...` and `-o JSON.space=...`) instead of inventing
formatting flags. `--nice`/`-n` is mere sugar for `-o JSON.space=2`.

## Why sources merge (and the precedence order)

A common real task is layering: a base config file, environment-piped
overrides, and last-minute command-line tweaks. Rather than make you choose
one input source, `jsonic` accepts all three and **deep-merges** them into a
single result.

The result is seeded as `{ val: null }` and each parsed source is folded in
with a deep merge. The merge is applied in this order — **files, then STDIN,
then argument fragments** — which makes argument fragments the highest
precedence (last applied wins on conflict), STDIN next, files lowest. The
intuition: the most "immediate" input (what you just typed) should win over
the most "stored" input (a file).

Two details fall out of this design:

- **Empty sources are no-ops.** An empty or whitespace-only source parses to
  `undefined`, and deep-merging `undefined` changes nothing. So a TTY STDIN
  (which reads empty) or a blank file never erases accumulated data.
- **`-` exists to force STDIN.** Normally STDIN is read only when there are
  no argument sources. If you want to merge a pipe *and* arguments, `-`
  explicitly opts STDIN back in.

## Plugins: extending the grammar from the command line

Because all parsing is the engine's, extending what `jsonic` understands
means adding an engine plugin — not changing the CLI. `-p <require>` loads a
plugin module and `jsonic.use(...)`s it before parsing, so a plugin like
`@tabnas/csv` makes `jsonic` parse CSV, `@tabnas/toml` makes it parse TOML,
and so on.

Two pragmatic design choices in the loader:

- **Scope fallback.** A bare `-p csv` retries `@tabnas/csv`, so the common
  first-party plugins have short names.
- **Export-shape tolerance.** Plugins in the wild export their function four
  different ways (bare function, `.default`, a named export matching the
  file basename, and its CamelCased form). The loader normalizes all four,
  so you do not have to know how a given plugin packaged its export. The
  four `test/p*.js` fixtures exist precisely to pin this behaviour.

Plugin options ride on the same `-o` mechanism, namespaced under
`plugin.<name>.<option>`, keeping a single uniform way to configure
everything.

## Debugging is also "just a plugin"

`-d`/`--debug` is not special-cased parsing machinery; it installs the
`@tabnas/debug` plugin, prints the engine's own grammar description
(`jsonic.debug.describe()`), and turns on a parse trace. This keeps the CLI
honest: the diagnostics you see are the engine's, not a separate
reimplementation that could drift. (This is why `@tabnas/debug` is a real
runtime peer dependency here, not a dev-only test dependency.)

## Testability as a design constraint

`run(argv, console)` takes its argument vector and its `console` as
parameters rather than reaching for the global `process.argv` and global
`console`. That lets the test suite call `run` in-process with a fake
console, capture `console.log` output, and even inject STDIN via a
`console.test$` string — no child processes, no real I/O. The binary
([`ts/bin/jsonic`](../bin/jsonic)) is a two-line shim that supplies the real
`process.argv` and `console`. Designing the core to be I/O-injectable is why
the whole CLI can be unit-tested deterministically.

## What is deliberately *not* here

- **No grammar.** This repo has no `.jsonic` grammar, no railroad diagram,
  no engine code. If you are looking for the parsing rules, they are in
  [`@tabnas/jsonic`](https://github.com/tabnas/jsonic).
- **No ABNF / BNF conversion.** That CLI is a separate package,
  [`@tabnas/abnf`](https://github.com/tabnas/abnf) (the `tabnas-abnf`
  command). Despite some stale keyword text in this package's metadata, the
  only binary this package ships is `jsonic`.

## See also

- [tutorial.md](tutorial.md) — zero to working result.
- [guide.md](guide.md) — task recipes.
- [reference.md](reference.md) — exact flags and contract.
- The Go port: [../../go/doc/concepts.md](../../go/doc/concepts.md), whose
  "Differences from the TS version" section covers where the two diverge.
