# @tabnas/jsonic-cli

The `jsonic` command — a command-line interface for
[`@tabnas/jsonic`](https://github.com/tabnas/jsonic), a JSON parser that
isn't strict.

It reads relaxed-JSON source (from arguments, `--file`, or STDIN), parses it
with jsonic, merges the results, and prints standard JSON via
`JSON.stringify`. This package has no grammar of its own — all parsing is
delegated to the engine.

## Install

```bash
npm install -g @tabnas/jsonic-cli
```

`@tabnas/jsonic` and the optional `@tabnas/debug` are peer dependencies; npm
7+ installs them automatically.

## Usage

```bash
jsonic a:1
# => {"a":1}
```

```bash
echo a:1 | jsonic
# => {"a":1}
```

## Documentation

Four-quadrant [Diátaxis](https://diataxis.fr) docs:

- [tutorial.md](doc/tutorial.md) — zero to working result, step by step.
- [guide.md](doc/guide.md) — task recipes (merging, filtering, plugins).
- [reference.md](doc/reference.md) — every flag, argument, exit behaviour,
  and the STDIN/STDOUT contract.
- [concepts.md](doc/concepts.md) — how the CLI relates to the engine, and
  the design trade-offs.

The Go port lives in [`../go`](../go) with its own
[docs](../go/doc/concepts.md).

> No railroad diagram — this CLI has no grammar. ABNF / grammar conversion
> lives in [`@tabnas/abnf`](https://github.com/tabnas/abnf) (the
> `tabnas-abnf` command).

## Build & test

```bash
npm install      # resolves the @tabnas siblings
npm run build    # tsc --build src
npm test         # node --test (CLI tests + doc-example harness)
```

## License

MIT. Copyright (c) Richard Rodger.
