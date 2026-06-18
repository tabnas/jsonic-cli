# @tabnas/jsonic-cli

Command-line interface for [`@tabnas/jsonic`](https://github.com/tabnas/jsonic) —
a JSON parser that isn't strict. Installs the `jsonic` command, which parses
relaxed-JSON source (from arguments, files, or STDIN) and prints standard
JSON.

This repo has **no grammar of its own**: it is a thin wrapper that delegates
all parsing to `@tabnas/jsonic` and adds argument parsing, source merging,
and serialization.

```bash
jsonic a:1
# => {"a":1}
```

## Implementations

| Path | Description |
|---|---|
| [`ts/`](ts/) | TypeScript / JavaScript implementation (`@tabnas/jsonic-cli`), the `jsonic` command. **Canonical.** |
| [`go/`](go/) | Go port (module `github.com/tabnas/jsonic-cli/go`, command `cmd/jsonic`). Tracks the TS behaviour. |

## Documentation

Four-quadrant [Diátaxis](https://diataxis.fr) docs, per implementation:

**TypeScript** — [tutorial](ts/doc/tutorial.md) ·
[how-to guide](ts/doc/guide.md) · [reference](ts/doc/reference.md) ·
[concepts](ts/doc/concepts.md)

**Go** — [tutorial](go/doc/tutorial.md) · [how-to guide](go/doc/guide.md) ·
[reference](go/doc/reference.md) · [concepts](go/doc/concepts.md)

Start with [`ts/README.md`](ts/README.md) for install and usage.

> There is no railroad diagram here — this CLI has no grammar.
> The ABNF / grammar-conversion CLI lives in
> [`@tabnas/abnf`](https://github.com/tabnas/abnf) as the `tabnas-abnf`
> command.

## Quick start

```bash
npm install -g @tabnas/jsonic-cli
jsonic a:1
# => {"a":1}
```

## License

MIT. Copyright (c) Richard Rodger.
