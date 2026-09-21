# Agents Guide — rs/ (the Rust port)

Read [`../AGENTS.md`](../AGENTS.md) first: it is the guide to the
repository, and everything it says about the CLI contract holds here.
This file is what is specific to the Rust crate.

## What this crate is

`tabnas-jsonic-cli` is the `jsonic` command as a **library plus a thin
launcher**, the shape `railroad/rs` uses:

| Path | What it is |
|---|---|
| `src/lib.rs` | `VERSION`, the module wiring, and the README doctest hook. |
| `src/cli.rs` | `run`, `run_with_plugins`, `capture`, `Captured`. The port of `ts/src/jsonic-cli.ts` `run()` and of `go/cli/run.go`. |
| `src/args.rs` | The argument loop and the dotted-path option and metadata bags. The port of `go/cli/args.go`. |
| `src/merge.rs` | A port of the engine's `util.deep`, which is how sources are folded together. NOT `tabnas_jsonic::deep_merge`; the module comment says why at length. |
| `src/stringify.rs` | A port of `JSON.stringify(value, replacer, space)` over `tabnas::Value`. The port of `go/cli/stringify.go`. |
| `src/registry.rs` | The compiled-in plugin registry behind `-p`. The port of `go/cli/registry.go`. |
| `src/help.rs` | The `--help` text. |
| `src/bin/jsonic.rs` | The launcher: `std::env::args`, a terminal check on standard input, `std::process::exit`. |

The tests drive `run` and `capture` **in process**. Nothing in the suite
spawns the binary, which is what makes a failure a stack trace rather
than an exit code.

## The one seam that is not obvious

`capture` exists because the fixtures compare **the first printed
entry**, and an entry is one `console.log` in the canonical command. The
help text, the grammar description and indented JSON are each ONE entry
spanning several physical lines, so splitting standard output on
newlines answers the wrong question and `-h` appears to print only a
blank line. `run` streams to a `Write`; `capture` keeps the entries.
Both go through the same `Sink` trait and the same `run_inner`.

## Options, and why there is a throwaway parser

`-o` builds a `serde_json` object from dotted paths, with each leaf value
parsed by vanilla jsonic, exactly as `handle_props` does. That bag then
has to reach the engine, and the two languages offer different doors:

- TypeScript hands an options OBJECT to `Jsonic.make(options)`, which
  applies it and then builds the grammar against the result. An option
  that decides which alternates exist (`map.child`, `list.child`,
  `list.pair`) therefore takes effect.
- Rust's engine applies an options DOCUMENT to a parser
  (`Tabnas::grammar_json`), while `tabnas_jsonic::make_with` wants the
  finished typed `Options`.

So `build_parser` resolves the caller's options on a throwaway instance,
reads the finished `Options` off it, and hands those to `make_with`,
which installs the grammar against them. That is the canonical order,
and it is why `-o list.pair=true '[a:1,2]'` prints `[{"a":1},2]` here
rather than `[2]`. Keep it: applying the document to a finished parser
instead is one line shorter and silently drops every alternate-gating
option.

`JSON` is stripped from the document, because it configures the output
and not the engine. **`plugin` is NOT stripped**, and the whole plugin
option path depends on that: the engine merges
`options.plugin[<plugin name>]` into the bag a plugin is installed with,
keyed by the PLUGIN'S own name rather than by the `-p` reference. That is how `-p ../test/p0 -o plugin.p0.x=0` reaches
the plugin in TypeScript, and it works the same way here, which is why
`use_plugin` is called with `None` for the call-site options.

A numeric option value is narrowed to an integer on the way into the
document. `serde_json` answers `as_u64` with `None` for a float-backed
number however round it is, so `-o debug.maxlen=11` would otherwise be
rejected as "not a non-negative integer".

## The debug flag

`-d` installs `tabnas-debug` with tracing on and prints
`describe(&parser) + "\n=== PARSE ==="` before the parse, as the
canonical command does. The trace reaches the command through the
engine's debug sink (`parser.options.debug.output`), which is an
`Arc<dyn Fn(&str)>` and so cannot hold the caller's `&mut` stream: the
lines are collected and flushed right after each parse, which puts them
back in the order the canonical command prints them.

Tracing under `-d` is unconditional, because `log=-1` is unconditional in
TypeScript and no `-o` turns it off there. `-p debug` without `-d` gets
the registry's entry, which is quiet: it describes the grammar and traces
nothing, which is what the canonical command does without that metadata.
The describe header is keyed by the `-p` REFERENCE rather than by the
plugin's name, so `-p debug` prints it and `-p @tabnas/debug` does not,
matching `null != plugins.debug` in TypeScript.

## Tests

| File | What it holds |
|---|---|
| `tests/parity_test.rs` | Every shared `../test/spec/*.tsv` fixture, through `tabnas_support`, plus a tripwire for a fixture file that stopped being discovered. |
| `tests/cli_test.rs` | The port of `ts/test/cli.test.js` and `go/cli/run_test.go`, including both spellings of every long flag and the `--k=v` form the canonical command does not accept. |
| `tests/stringify_test.rs` | The output contract. Every expectation was MEASURED by running `JSON.stringify` under Node, because a formatter checked against itself proves nothing. |
| `tests/untrusted_test.rs` | Deep nesting, long input, unterminated constructs, empty input, control characters, odd Unicode and bytes that are not UTF-8. |
| `tests/divergence_test.rs` | The `../test/divergent.tsv` register, plus the divergences a fixture cell cannot express. |
| `tests/version_test.rs` | `Cargo.toml`, `Cargo.lock`, `VERSION` and `ts/package.json` agree. |
| `tests/common/mod.rs` | `run`, `run_with`, `run_streams`, and the four fixture plugins. |
| `tests/testdata/` | `foo.jsonic` and `bar.jsonic`, byte for byte the `ts/test/` and `go/cli/testdata/` files. |

The four `ts/test/p*.js` fixtures exist to cover the four module export
shapes `handle_plugins` accepts. Rust has no module loading and so no
export shapes; what survives the adaptation is the option plumbing, which
is what `value_def_plugin` carries. The plugin's NAME is what the option
sub-bag is keyed by, so a fixture plugin registered under a reference
must be constructed with the matching name.

## Adding a fixture

Prefer a row in `../test/spec/` over an assertion here: that directory is
run by all three runtimes, and a row added to it runs everywhere without
touching a runner. A case that turns on how a runtime loads code or reads
the filesystem stays in `tests/cli_test.rs`, where the adaptation is
written down.

A row that this port cannot pass goes in `../test/divergent.tsv` with a
`rust` column and an entry in `../DIVERGENCE.md`, never in
`../test/spec/`: everything there is run by TypeScript and Go too, and a
row one of them cannot pass breaks their suites.

## The gate

From `rs/`:

```bash
cargo fmt --check
cargo test --all-targets
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
```

From the repository root, `make test-rs` is the fast inner loop and
`bash ci/rust/run.sh` is the whole gate, including formatting and the
lockfile comparison. `ci/workflows/rust.yml` runs that same script and is
STAGED under ADR-8: a maintainer promotes it, never a session.

`rs/README.md` is in the gated documentation set
(`ts/scripts/gated-docs.cjs`), so `make prose` and
`node --test ts/test/docs.test.js` both read it. Every `rust` fence in it
is a complete program, compiled by `cargo test --doc` through the
`readme_examples` hook in `src/lib.rs`.

## Versions

`Cargo.toml`, `Cargo.lock`, `VERSION` in `src/lib.rs` and
`ts/package.json` must agree, and `tests/version_test.rs` fails when they
do not. `make version-rs V=x.y.z` moves the two Rust sites and refreshes
the lock; the TypeScript and Go sites are the release orchestrator's.

## House rules

- **Never edit a sibling repository.** The engine, the grammars, the
  debug plugin and the fixture runner are path dependencies on checkouts
  beside this one. A defect found in one of them is reported there, and
  worked around here only where the workaround is small and explained,
  as `src/merge.rs` is.
- `export CARGO_BUILD_JOBS=2 CARGO_TERM_COLOR=never`, and run one cargo
  command at a time.
- `git status --short` at the end shows `rs/**`, `Makefile`, `ci/**`,
  the documentation pages and the register, and nothing else.
