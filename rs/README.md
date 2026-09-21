# tabnas-jsonic-cli (Rust)

The `jsonic` command, a JSON parser that is not strict, as a crate:
`tabnas-jsonic-cli` (library `tabnas_jsonic_cli`).

It reads relaxed-JSON source from command arguments, from `--file`, or
from standard input, parses each with
[`tabnas-jsonic`](https://github.com/tabnas/jsonic), merges the results
in precedence, and prints standard JSON.

This is the Rust port of the canonical TypeScript implementation in
[`../ts`](../ts); the TypeScript version is authoritative and this crate
tracks it. The Go port is in [`../go`](../go). There is no grammar here
and none of the parsing happens here: the command is argument parsing,
option, metadata and plugin wiring, source merging, and serialization.

## The command

```bash
jsonic a:1
# {"a":1}

jsonic -n a:b:1 a:c:2
# {
#   "a": {
#     "b": 1,
#     "c": 2
#   }
# }

echo a:1 | jsonic
# {"a":1}
```

`jsonic --help` prints the whole flag surface. In short: `-` reads
standard input, `--` ends flag parsing, `--file` / `-f` loads a file,
`--option` / `-o` and `--meta` / `-m` set a dotted-path option or
metadata value, `--nice` / `-n` indents, `--plugin` / `-p` installs a
plugin, `--debug` / `-d` traces the parse, and `--help` / `-h` prints the
usage.

Only those exact spellings are flags. `--option=JSON.space=2` is not
`--option`: like every other unrecognized token it becomes a source and
is parsed as relaxed JSON, which is what the canonical command does with
it.

Sources merge last wins by precedence. Every `--file` result is folded in
first, then standard input, then the positional sources, each deep-merged
over what came before. Standard input is read whenever there is no
positional source, so `jsonic -f config.jsonic` at a terminal reads the
terminal and finds nothing there.

## Use as a library

The crate is the command, and the binary in `src/bin/jsonic.rs` is a
launcher. `run` takes its streams as arguments, so a caller drives the
whole command in process:

```rust
fn main() {
    let argv: Vec<String> = ["jsonic", "a:1", "b:[2]"]
        .iter()
        .map(|arg| arg.to_string())
        .collect();
    let mut out: Vec<u8> = Vec::new();
    let code = tabnas_jsonic_cli::run(
        &argv,
        &mut std::io::empty(),
        &mut out,
        &mut std::io::sink(),
    );
    assert_eq!(code, 0);
    assert_eq!(String::from_utf8(out).unwrap(), "{\"a\":1,\"b\":[2]}\n");
}
```

`argv` starts with the program name, as `std::env::args` yields it. The
second argument is standard input, which is read only when the arguments
call for it. The return value is the process exit code: zero, or one when
something is reported on standard error.

To inspect what was printed rather than write it somewhere, `capture`
returns one entry per printed call, which is the unit a `console.log` is
in the canonical command:

```rust
fn main() {
    let argv: Vec<String> = ["jsonic", "-n", "a:1"]
        .iter()
        .map(|arg| arg.to_string())
        .collect();
    let out = tabnas_jsonic_cli::capture(&argv, &mut std::io::empty(), &Default::default());
    assert_eq!(out.code, 0);
    assert_eq!(out.lines, vec!["{\n  \"a\": 1\n}".to_string()]);
}
```

Standard input is read from anything that implements `Read`, so a piped
document needs no file and no process:

```rust
fn main() {
    let argv: Vec<String> = ["jsonic", "-", "b:2"]
        .iter()
        .map(|arg| arg.to_string())
        .collect();
    let out = tabnas_jsonic_cli::capture(&argv, &mut &b"{a:1}"[..], &Default::default());
    assert_eq!(out.first(), r#"{"a":1,"b":2}"#);
}
```

## Plugins

`-p` / `--plugin` resolves a reference against a compiled-in registry.
The stock command carries three: `debug`, `jsonic` and `json`. The
`@tabnas/<name>` form and a path-like reference ending in the name
resolve to the same entry.

```rust
fn main() {
    let argv: Vec<String> = ["jsonic", "-p", "json", r#"{"a":1}"#]
        .iter()
        .map(|arg| arg.to_string())
        .collect();
    let out = tabnas_jsonic_cli::capture(&argv, &mut std::io::empty(), &Default::default());
    assert_eq!(out.first(), r#"{"a":1}"#);
}
```

A custom command adds its own with `register_plugin` before calling
`run`, and the options arrive under the plugin's own name through
`-o plugin.<name>.<option>=<value>`:

```rust
use tabnas::{Plugin, PluginError, Tabnas, Value};

fn main() {
    fn noop(_parser: &mut Tabnas, _options: &Value) -> Result<(), PluginError> {
        Ok(())
    }
    tabnas_jsonic_cli::register_plugin("example", Plugin::new("example", noop));

    let argv: Vec<String> = ["jsonic", "-p", "example", "a:1"]
        .iter()
        .map(|arg| arg.to_string())
        .collect();
    let out = tabnas_jsonic_cli::capture(&argv, &mut std::io::empty(), &Default::default());
    assert_eq!(out.first(), r#"{"a":1}"#);
}
```

The registry is what stands in for the TypeScript `require`. Neither Rust
nor Go can load a plugin module by name at run time, so both ports
compile theirs in. The tabnas grammars that are not built in (`csv`,
`toml`, `directive`, `multisource` and the rest) live in crates that depend
on jsonic, and pulling the family into every build of the command would cost
every user of it; a custom binary is the way to reach them.

## Output

Whatever the canonical command prints for a value, this prints for the
same value. The serializer is a port of JavaScript's `JSON.stringify`,
down to the details that differ from Rust's own formatting:

- **Key order is source order.** The engine's object keeps every key
  where it arrived, which is how a JavaScript object enumerates, so the
  keys come out unsorted.
- **Numbers follow `Number::toString`.** Fixed notation up to 1e21 and
  down to 1e-6, exponential outside that range, and shortest
  round-trippable digits throughout. Rust's own formatting never chooses
  exponential, so `1e21` would print as twenty-two digits without this.
- **Non-finite numbers print as `null`**, and both zeroes print as `0`.
- `-o JSON.space=<n>` clamps to ten, `-o JSON.space="<text>"` indents
  with that text, and `-o JSON.replacer=[<keys>]` keeps those object keys
  at every depth while leaving array elements alone.

## Install

Neither the engine nor the grammars are published to a registry, so all
four are consumed as **sibling checkouts**, the standard tabnas
development model. Clone `https://github.com/tabnas/parser`,
`https://github.com/tabnas/jsonic`, `https://github.com/tabnas/json` and
`https://github.com/tabnas/debug` next to this repository and point at
them:

```toml
[dependencies]
tabnas-jsonic-cli = { path = "../jsonic-cli/rs" }
tabnas = { path = "../parser/rs" }
```

Both entries are needed for the plugin example above: a crate's
dependencies are not passed on to its dependents, so `tabnas-jsonic-cli`
alone does not put `tabnas` in the extern prelude. The test suite
additionally needs `https://github.com/tabnas/support` beside the
repository, for the shared fixture runner.

## Differences from the canonical TypeScript

Every flag, every merge, every exit path and every printed value is meant
to be the TypeScript one, and the shared fixtures in
[`../test/spec`](../test/spec) hold all three runtimes to that. What
differs is recorded in `../DIVERGENCE.md` and, where a fixture cell can
express it, in `../test/divergent.tsv`, which the suite runs. The shape
of it:

- **A failure exits nonzero.** The canonical `ts/bin/jsonic` catches the
  rejection, prints the message and sets no exit code, so it exits zero
  after reporting a parse failure. Both ports exit one.
- **Plugins are compiled in**, so an unresolvable `-p` reference reports
  a registry miss rather than a module-loader failure, and the help
  text's Plugins section says so.
- **The debug trace comes from the plugin.** TypeScript switches on the
  engine log with `--meta log=-1`; here the trace is
  [`tabnas-debug`](https://github.com/tabnas/debug)'s own. The
  description header and the JSON result agree exactly, and the lines
  between them do not.
- **Nesting past 127 containers is refused** with the error code
  `cancel`. That bound belongs to `tabnas-jsonic` and is recorded there.
  It is what keeps a deeply nested document from ending the process,
  since walking a value to merge, print or drop it uses the call stack.

## Build and test

The engine, the grammars, the debug plugin and the fixture runner are all
path dependencies on sibling checkouts, so there is nothing to fetch:

```bash
cargo test --all-targets
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
```

Or, from the repository root, `make test-rs`. For what CI would say,
including formatting and the lockfile check, run `ci/rust/run.sh`.

The suite runs every shared `../test/spec/*.tsv` fixture, the same files
the TypeScript and Go suites run, through the shared
[`tabnas-support`](https://github.com/tabnas/support) loader. Beside them
are the ports of `ts/test/cli.test.js` and `go/cli/run_test.go`, a
`JSON.stringify` parity table measured against Node, the untrusted-input
boundaries, the divergence register, and the version check that holds
`Cargo.toml`, `Cargo.lock`, `VERSION` and `ts/package.json` together.

## License

MIT.
