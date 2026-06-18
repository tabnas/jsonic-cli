# jsonic-cli (Go)

The Go port of the `jsonic` command — a command-line interface for the Go
relaxed-JSON parser engine (`github.com/tabnas/jsonic/go`). It reads
relaxed-JSON source (from arguments, `--file`, or STDIN), parses it, merges
the results, and prints standard JSON.

Module path: `github.com/tabnas/jsonic-cli/go`. Command: `cmd/jsonic`. This
is a `package main` program — a tool, not a library. The TypeScript package
in [`../ts`](../ts) is canonical; this port tracks its behaviour.

## Build & run

The module wraps unpublished `@tabnas` Go siblings via `replace` directives
(see [`go.mod`](go.mod)); clone them as siblings first. Then:

```bash
cd go
go build ./...
go run ./cmd/jsonic a:1
# => {"a":1}
```

Install a binary onto your `PATH`:

```bash
go install github.com/tabnas/jsonic-cli/go/cmd/jsonic@latest
```

## Documentation

Four-quadrant [Diátaxis](https://diataxis.fr) docs:

- [tutorial.md](doc/tutorial.md) — build it and parse your first input.
- [guide.md](doc/guide.md) — task recipes.
- [reference.md](doc/reference.md) — every flag, exit code, and contract.
- [concepts.md](doc/concepts.md) — how it relates to the engine, plus a
  **Differences from the TS version** section (notably: plugins must be
  compiled in — Go has no runtime `require`).

> No railroad diagram — this CLI has no grammar. ABNF / grammar conversion
> lives in [`@tabnas/abnf`](https://github.com/tabnas/abnf) (the
> `tabnas-abnf` command).

## Test

```bash
cd go
go test ./...
```

The Go tests port `../ts/test/cli.test.js` one-for-one.

## License

MIT. Copyright (c) Richard Rodger.
