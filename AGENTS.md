# Agents Guide — jsonic-cli

## What this project is

`@tabnas/jsonic-cli` is the **command-line interface for
[`@tabnas/jsonic`](https://github.com/tabnas/jsonic)** — a JSON parser that
isn't strict. It installs the `jsonic` command, which reads relaxed-JSON
source (from arguments, `--file`, or STDIN), parses it with jsonic, merges
the results, and prints standard JSON via `JSON.stringify`.

It is a **thin wrapper, not a grammar**: this repo has **no grammar of its
own**, no engine code, and no parsing rules — all parsing is delegated to
`@tabnas/jsonic`. The CLI's job is argument parsing, option/meta/plugin
wiring, source merging, and serialization. It is the relaxed-JSON
counterpart to the strict-JSON `tabnas-json` CLI that ships inside the
`@tabnas/json` package.

ts/ is canonical; a **Go port** in `go/` (the `jsonic` command, module
`github.com/tabnas/jsonic-cli/go`) tracks it. There is no grammar here and
no railroad diagram; the TS↔Go contract is the **CLI behavior** — same
flags, same stdout for the same inputs. It is pinned by the shared
`test/spec/*.tsv` fixtures, one row per argv (see
[`test/AGENTS.md`](test/AGENTS.md)), with the plugin-loading and
filesystem cases left in the in-language suites. The
repo was created on 2026-06-16; the Go port was added on 2026-06-18.

> The ABNF / grammar-conversion CLI is **not here.** It lives in the
> [`abnf`](https://github.com/tabnas/abnf) repo as the `tabnas-abnf` command
> (the `@tabnas/abnf` package). This repo ships only the `jsonic` bin, and
> `ts/package.json` says so — the old `jsonic-bnf` mention in its
> `description`/`keywords` is gone.

## Repository map

| Path | What it is |
|---|---|
| [`ts/`](ts/) | The canonical implementation — the `@tabnas/jsonic-cli` package. |
| [`ts/src/jsonic-cli.ts`](ts/src/jsonic-cli.ts) | The whole CLI: arg parsing, `run()`, `help()`, plugin/option/meta handling. Builds to `dist/jsonic-cli.js`. |
| [`ts/bin/jsonic`](ts/bin/jsonic) | The `jsonic` bin (the only one in `package.json`). `require`s `../dist/jsonic-cli` and calls `run(process.argv, console)`, printing `e.message` on rejection. |
| [`ts/test/cli.test.js`](ts/test/cli.test.js) | The in-language suite — plain committed JS (not compiled), run by `node --test`. Calls `run()` in-process with a fake `console`. |
| [`ts/test/parity.test.js`](ts/test/parity.test.js) | Runs every shared `test/spec/*.tsv` fixture (auto-discovered) through `run()`. |
| `ts/test/doc-examples.test.ts` | Extracts ```` ```js ```` blocks with `// =>` assertions from the repo's Markdown and runs them (shared harness, identical in every tabnas repo). |
| `ts/test/p0.js`, `p1.js`, `p2.js`, `pa-qa.js` | Plugin fixtures exercising the four export shapes `handle_plugins` accepts (bare fn, `.default`, named `[name]`, CamelCased `PaQa`). |
| `ts/test/foo.jsonic`, `bar.jsonic` | `--file` source fixtures (`bar:1` / `qaz: 2`). |
| [`ts/doc/`](ts/doc/) | Diátaxis docs (tutorial / guide / reference / concepts). No `grammar.*` — there is no grammar. |
| [`test/spec/`](test/spec/) | The shared TSV fixtures both runtimes run — see [`test/AGENTS.md`](test/AGENTS.md). |
| [`go/`](go/) | The Go port (module `github.com/tabnas/jsonic-cli/go`). |
| [`go/cmd/jsonic/main.go`](go/cmd/jsonic/main.go) | Thin entry: `cli.Run(os.Args[1:], cli.ReadStdin(), os.Stdout, nil)`. Holds `const Version` (injected by `make publish-go`). |
| [`go/cli/run.go`](go/cli/run.go) | The library package: `Run`/`runLog` (plugin/option/meta wiring, source merge, serialization) + `ReadStdin`. |
| `go/cli/args.go` | Arg parsing, dotted-path prop bags, plugin registry lookup. |
| `go/cli/registry.go` | The compiled-in plugin registry + `RegisterPlugin`/`Plugins`. |
| `go/cli/stringify.go` | A `JSON.stringify(value, replacer, space)` port (replacer whitelist, space indent, insertion-ordered keys). |
| `go/cli/help.go` | The `--help` text (mirrors the TS `help()`). |
| `go/cli/run_test.go` | Port of `ts/test/cli.test.js`. |
| `go/cli/parity_test.go` | `TestSpec` — globs and runs the shared `test/spec/*.tsv`. |
| `go/cli/testdata/foo.jsonic`, `bar.jsonic` | Go `--file` fixtures (same contents as the TS ones). |
| [`go/doc/`](go/doc/) | Diátaxis docs for the port. |

There is **no `ts/doc/grammar.*` and no railroad diagram** — there is no
grammar.

> **Go plugin loading differs from TS.** The TS CLI loads `-p`/`--plugin`
> modules by `require(<reference>)`; Go cannot load a module by name at
> runtime, so the Go CLI resolves plugins from a compiled-in registry
> (`go/cli/registry.go`). The standard binary pre-registers the three
> plugins already in its dependency graph — `debug`, `jsonic`, `json` —
> and custom binaries add more with `cli.RegisterPlugin` before `cli.Run`;
> the plugin tests inject the four fixture plugins as native functions.
> The `-d`/`--debug` flag uses the first-party `@tabnas/debug` Go plugin
> (`debug.Debug` + `debug.Describe`).

## The tabnas engine dependency

In a dev checkout this package resolves its `@tabnas` dependencies from the
**sibling repos** (the standard tabnas dev model). It is **not** the usual
all-dev-only arrangement, because the CLI uses tabnas packages at runtime
beyond jsonic:

- `@tabnas/jsonic` is the engine wrapper the CLI parses with — a **runtime
  peer dependency** (`">=0"`).
- `@tabnas/debug` is **also a real runtime peer dependency** (`">=0"`),
  not a dev-only test dep as in the grammar repos. `src/jsonic-cli.ts`
  `import`s `Debug` at the top level and installs it when the user passes
  `--debug` / `-d`. So debug must resolve at runtime for that flag to work;
  npm 7+ installs it automatically alongside the CLI.
- `@tabnas/parser` is pulled in solely so the `@tabnas/jsonic` type imports
  (`import type { Plugin } from '@tabnas/jsonic'`) resolve at build time.
  The CLI never imports `@tabnas/parser` directly.

All three are declared both as peers and as devDependencies:

```json
"peerDependencies": {
  "@tabnas/jsonic": ">=0", "@tabnas/debug": ">=0", "@tabnas/parser": ">=0"
}
"devDependencies": {
  "@tabnas/jsonic": "*", "@tabnas/debug": "*", "@tabnas/parser": "*"
}
```

`engines.node` is `">=24"`; npm >=7 / Node >=24 auto-installs peers.
`node_modules/@tabnas/{jsonic,debug,parser,json}` resolve as symlinks into
the sibling checkouts (wired by `admin/scripts/link.sh` — do not `npm ci`
or delete `node_modules`, that breaks the wiring). Clone
`https://github.com/tabnas/{jsonic,debug,parser}` (plus their own
transitive closure) as siblings and build their TS first, then work here.
CI does this for you (see below).

The Go module's dependencies (`jsonic/go`, `parser/go`, `debug/go`,
`json/go`) are required at published versions in [`go/go.mod`](go/go.mod)
and resolved locally through the repo-set `go.work`.

There is **no `@tabnas/railroad` dependency** — there is no grammar to
diagram.

## CLI behaviour (the non-obvious bits)

`run(argv, console)` is written to be **testable in-process**: it takes
`argv` and a `console` as parameters (the bin passes `process.argv` and the
real `console`; tests pass a fake whose `log` calls are captured). Two
console hooks make the suite work without real I/O — keep them:

- **`console.test$`** — if `read_stdin` sees a **string** `console.test$`,
  it returns it as the STDIN body instead of reading `process.stdin`. Both
  fakes (`cli.test.js`'s `make_cn`, `parity.test.js`'s `runCli`) default it
  to the **empty string** — "nothing piped in", which is what the real CLI
  sees at a terminal and what the Go tests pass as their `stdin` argument.
  Keep it a string: a truthy non-string falls through to the real
  `process.stdin`, which under `node --test` never ends, so any case with
  no positional source (`--file` only, or no source at all) hangs the run
  instead of failing.
- The fake `console.log` pushes each call's args; assertions read
  `cn.d.log[0][0]`.

Other behaviour an agent should know before touching `jsonic-cli.ts`:

- **Sources merge, last-wins by precedence.** `--file` results, then
  STDIN, then positional `<source-text>` args are each `util.deep`-merged
  into `data.val`. Unknown `-`-prefixed args (e.g. `--not-an-arg`) fall
  through to `args.sources` and are parsed as source text, not errors.
- **STDIN is read whenever there are no positional sources** — including
  when `--file` supplied all the input. At a terminal that is harmless
  (`isTTY` reads as `''`), but in a non-TTY context with nothing piped in
  it blocks. Go behaves the same (`cmd/jsonic` calls `cli.ReadStdin()`
  before `cli.Run`).
- **A value-taking flag in the last position consumes nothing.**
  `jsonic a:1 -o` is a no-op, not a crash: the arg loop only takes a value
  when one follows (`aI + 1 < argv.length`), mirroring Go's
  `if i+1 < len(argv)`. Pinned by `test/spec/bad-args.tsv`.
- **`-o` / `-m` values are parsed by vanilla `Jsonic(...)`**, so
  `-o JSON.space=2` and `-o JSON.replacer=[b]` set real typed values via
  `util.prop` on dotted paths. (The code comment flags that this couples
  arg parsing to core jsonic working.)
- **`--nice` / `-n`** is sugar for `-o JSON.space=2`.
- **`--debug` / `-d`** installs `Debug` (cast `as unknown as Plugin`
  because debug is typed against the bare engine, not the jsonic wrapper),
  adds `--meta log=-1`, and prints `jsonic.debug.describe()` before the
  parse. `options.debug.get_console` is wired to the injected `console` so
  debug output is captured in tests.
- **`--plugin` / `-p`** loads a plugin by `require`. `handle_plugins`
  retries `@tabnas/<name>` for bare names, then normalizes four export
  shapes (bare function, `module.exports.default`, a named export matching
  the file basename, and the CamelCased form). The four `test/p*.js` /
  `pa-qa.js` fixtures each cover one shape — keep them in sync with that
  logic. Plugin options come from `-o plugin.<name>.<opt>=<val>`.

## What the suites pin (the conformance bar)

There is no external language spec to conform to — the bar is the CLI
contract above, and it is covered like this:

| Behaviour | Pinned by |
|---|---|
| Basic parse, multi-source merge, dotted-key merge, unknown flag as source, empty source, number formatting, source key order | `test/spec/basic.tsv` |
| `--`, `--option`/`--nice`/`--meta` long forms, `-m`, `-d` describe header | `test/spec/args.tsv` |
| `JSON.space` (number, clamped, string), `JSON.replacer` (array, scalar, numeric, nested) | `test/spec/stringify.tsv` |
| STDIN alone, `-` alias, `-` plus arguments, no source at all | `test/spec/stdin.tsv` |
| Empty/malformed `-o`, empty `-f`, value-less trailing flag | `test/spec/bad-args.tsv` |
| `-h` / `--help` | `test/spec/help.tsv` |
| `--file` (real filesystem), the four plugin export shapes, unresolvable `-p`, `-d` trace tail | `ts/test/cli.test.js` and `go/cli/run_test.go` (each in its own idiom) |
| Registry-resolved built-in plugins, `RegisterPlugin`, exit codes | `go/cli/run_test.go` |
| The `ts/doc/guide.md` "Verified examples" block | `ts/test/doc-examples.test.ts` |

Anything expressible as argv (+ stdin) → first printed line belongs in
`test/spec/`, so both runtimes check it. Keep it that way.

## Build & test

From the repo root, `make build` (= `build-ts` + `build-go`) and
`make test` (= `test-ts` + `test-go`) do everything. Per runtime:

```bash
cd ts
npm install            # peers auto-install; @tabnas siblings are symlinks
npm run build          # tsc --build src   (NOT "src test")
npm test               # node --enable-source-maps --test 'test/**/*.test.js' 'test/**/*.test.ts'

cd ../go
go build ./... && go test ./...
```

The key difference from the grammar repos: **`build` compiles `src` only.**
The tests are **committed plain `.js`** in `test/` (plus the one shared
`doc-examples.test.ts`, which Node 24 runs directly by type-stripping) and
are executed by `node --test` — they are not compiled, so there is **no
`dist-test/`** here. `npm test` runs them against the already-built
`dist/jsonic-cli.js`, so build first. `npm run reset` does the full
`clean && npm i && build && test` cycle.

The repo-root [`Makefile`](Makefile) also carries the Go targets
(`build-go`, `test-go`, `clean-go`, `publish-go`, `tags-go`). `make clean`
is a direct `rm -rf ts/dist ts/dist-test` plus `go clean` (it does *not*
run the npm `clean` script, which would also wipe `node_modules` and the
lockfile). `make publish-ts` runs the tests then `npm publish --access
public` at the `package.json` version;
`make publish-go V=x.y.z` injects `V` into the `const Version` in
`go/cmd/jsonic/main.go`, commits, and tags `go/vX.Y.Z`. `make reset`
delegates to the `ts/` `reset` script and rebuilds/retests Go.

## CI

Two workflows, both org-standard (the old per-repo `build.yml` is gone;
`.github/workflows/*` is written by the `tabnas/admin` rollout, not from a
session):

- [`ci.yml`](.github/workflows/ci.yml) — a thin caller of the shared
  `tabnas/.github` `polyglot-ci.yml`, passing
  `deps: "parser debug json abnf railroad jsonic"`. That reusable workflow
  clones the tabnas closure as siblings, builds them in topo order (so
  jsonic and debug are built before this repo), then builds and tests both
  the TS and Go sides here.
- [`release.yml`](.github/workflows/release.yml) — publishes the npm
  package on a `ts/v*` tag via GitHub OIDC trusted publishing. It builds
  against already-published dependency versions and does **not** re-run the
  suite (some sibling-by-path tests can't resolve in that standalone env);
  the gate is a green `ci` on main, enforced by `admin/publish.sh`. The Go
  module needs no publish step — the proxy serves it from the `go/v*` tag.

Note CI builds the full sibling set even though this repo only *imports*
jsonic + debug + (type-only) parser, because those are jsonic's own
transitive build dependencies.
