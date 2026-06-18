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
`github.com/tabnas/jsonic-cli/go`) tracks it. There is no grammar here, so
there is no `.tsv` alignment fixture and no railroad diagram; the TS↔Go
contract is the **CLI behavior** — same flags, same stdout for the same
inputs (the Go `cmd/jsonic/main_test.go` ports `ts/test/cli.test.js`). The
repo was created on 2026-06-16; the Go port was added on 2026-06-18.

> The BNF / grammar-conversion CLI is **not here.** It lives in the
> [`abnf`](https://github.com/tabnas/abnf) repo as the `tabnas-bnf` command
> (the `@tabnas/bnf` package). This package's `package.json` `description`
> still mentions a `jsonic-bnf` command (and `bnf` is in its `keywords`),
> but that is stale text — this repo ships only the `jsonic` bin.

## Repository map

| Path | What it is |
|---|---|
| [`ts/`](ts/) | The only implementation — the `@tabnas/jsonic-cli` package. |
| [`ts/src/jsonic-cli.ts`](ts/src/jsonic-cli.ts) | The whole CLI: arg parsing, `run()`, `help()`, plugin/option/meta handling. Builds to `dist/jsonic-cli.js`. |
| [`ts/bin/jsonic`](ts/bin/jsonic) | The `jsonic` bin (the only one in `package.json`). `require`s `../dist/jsonic-cli` and calls `run(process.argv, console)`, printing `e.message` on rejection. |
| [`ts/test/cli.test.js`](ts/test/cli.test.js) | The test suite — plain committed JS (not compiled), run by `node --test`. Calls `run()` in-process with a fake `console`. |
| `ts/test/p0.js`, `p1.js`, `p2.js`, `pa-qa.js` | Plugin fixtures exercising the four export shapes `handle_plugins` accepts (bare fn, `.default`, named `[name]`, CamelCased `PaQa`). |
| `ts/test/foo.jsonic`, `bar.jsonic` | `--file` source fixtures (`bar:1` / `qaz: 2`). |
| [`go/`](go/) | The Go port (module `github.com/tabnas/jsonic-cli/go`). |
| [`go/cmd/jsonic/main.go`](go/cmd/jsonic/main.go) | The CLI entry + `run`/`runLog` (arg loop, plugin/option/meta wiring, source merge, serialization). Holds `const Version`. |
| `go/cmd/jsonic/args.go` | Arg parsing, dotted-path prop bags, plugin registry lookup. |
| `go/cmd/jsonic/stringify.go` | A `JSON.stringify(value, replacer, space)` port (replacer whitelist, space indent). |
| `go/cmd/jsonic/help.go` | The `--help` text (mirrors the TS `help()`). |
| `go/cmd/jsonic/main_test.go` | Port of `ts/test/cli.test.js`. |
| `go/cmd/jsonic/testdata/foo.jsonic`, `bar.jsonic` | Go `--file` fixtures (same contents as the TS ones). |

There is **no `test/spec/` and no `ts/doc/grammar.*`** — there is no grammar.

> **Go plugin loading differs from TS.** The TS CLI loads `-p`/`--plugin`
> modules by `require(<reference>)`; Go cannot load a module by name at
> runtime, so the Go CLI resolves plugins from a compiled-in registry
> (empty in the production binary; the tests inject the four fixture
> plugins as native functions). The `-d`/`--debug` flag uses the
> first-party `@tabnas/debug` Go plugin (`debug.Debug` + `debug.Describe`).

## The tabnas engine dependency

This package depends on the unpublished `@tabnas` siblings via a **sibling
checkout** (the standard tabnas dev model until the packages publish tagged
releases). It is **not** the usual all-dev-only arrangement, because the
CLI uses one tabnas package at runtime beyond jsonic:

- `@tabnas/jsonic` is the engine wrapper the CLI parses with — a **runtime
  peer dependency** (`">=2"`).
- `@tabnas/debug` is **also a real runtime peer dependency** (`">=2"`),
  not a dev-only test dep as in the grammar repos. `src/jsonic-cli.ts`
  `import`s `Debug` at the top level and installs it when the user passes
  `--debug` / `-d`. So debug must resolve at runtime for that flag to work;
  npm 7+ installs it automatically alongside the CLI.
- `@tabnas/parser` is **devDependency-only** here (`file:../../parser/ts`),
  pulled in solely so the `@tabnas/jsonic` type imports
  (`import type { Plugin } from '@tabnas/jsonic'`) resolve at build time.
  The CLI never imports `@tabnas/parser` directly.

`ts/package.json` mirrors the two peers as `file:` devDependencies for
local builds:

```json
"peerDependencies": { "@tabnas/jsonic": ">=2", "@tabnas/debug": ">=2" }
"devDependencies":  {
  "@tabnas/jsonic": "file:../../jsonic/ts",
  "@tabnas/debug":  "file:../../debug/ts",
  "@tabnas/parser": "file:../../parser/ts"
}
```

`engines.node` is `">=24"`; npm >=7 / Node >=24 auto-installs peers.
`node_modules/@tabnas/{jsonic,debug,parser}` resolve as symlinks into the
sibling checkouts. Clone `https://github.com/tabnas/{jsonic,debug,parser}`
(plus their own transitive closure) as siblings and build their TS first,
then work here. CI does this for you (see below).

There is **no `@tabnas/railroad` dependency** — there is no grammar to
diagram.

## CLI behaviour (the non-obvious bits)

`run(argv, console)` is written to be **testable in-process**: it takes
`argv` and a `console` as parameters (the bin passes `process.argv` and the
real `console`; tests pass a fake whose `log` calls are captured). Two
console hooks make the suite work without real I/O — keep them:

- **`console.test$`** — if `read_stdin` sees a string `console.test$`, it
  returns it as the STDIN body instead of reading `process.stdin`. The
  truthy-but-non-string default in the fake just marks "this is a test".
- The fake `console.log` pushes each call's args; assertions read
  `cn.d.log[0][0]`.

Other behaviour an agent should know before touching `jsonic-cli.ts`:

- **Sources merge, last-wins by precedence.** `--file` results, then
  STDIN, then positional `<source-text>` args are each `util.deep`-merged
  into `data.val`. Unknown `-`-prefixed args (e.g. `--not-an-arg`) fall
  through to `args.sources` and are parsed as source text, not errors.
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

## Build & test

From `ts/`:

```bash
npm install            # auto-installs the @tabnas/jsonic + @tabnas/debug peers; resolves file: siblings
npm run build          # tsc --build src   (NOT "src test")
npm test               # node --enable-source-maps --test test/**/*.test.js
```

The key difference from the grammar repos: **`build` compiles `src` only.**
The tests are **committed plain `.js`** in `test/` and run directly by
`node --test` — they are not TypeScript and are not compiled, so there is
**no `dist-test/`** here. `npm test` runs them against the already-built
`dist/jsonic-cli.js` (build first). `npm run reset` does the full
`clean && npm i && build && test` cycle.

The repo-root [`Makefile`](Makefile) is the **TS-only variant** (no Go
targets): `make build` / `make test` delegate to the `ts/` `build` / `test`
npm scripts, while `make clean` is a direct `rm -rf ts/dist ts/dist-test`
(it does *not* run the npm `clean` script, which would also wipe
`node_modules` and the lockfile). `make publish-ts` runs the tests then
`npm publish --access public` at the `package.json` version (currently
`0.1.0`); `make reset` delegates to the `ts/` `reset` script. There is
**no `make publish-go` / `tags-go`** and no `const Version` to inject —
those exist only in repos with a `go/`.

## CI

[`.github/workflows/build.yml`](.github/workflows/build.yml) has a **single
`build` job** (no `build-go`, since there is no Go port):

- Matrix: Ubuntu / Windows / macOS, Node 24.
- Sets `git config --global core.autocrlf false` (LF line endings; the
  Windows runner would otherwise risk CRLF corruption of fixtures).
- Clones the tabnas closure as siblings — `parser debug json abnf railroad
  jsonic` — then `npm i && npm run build --if-present` in that topo order
  (so jsonic and debug are built before this repo), and finally
  `npm test` in `jsonic-cli/ts`.

Note CI builds the full sibling set even though this repo only *imports*
jsonic + debug + (type-only) parser, because those are jsonic's own
transitive build dependencies. There is no separate publish step in CI;
`@tabnas` packages are not published to npm from CI.
