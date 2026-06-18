# Tutorial: parse your first input with the Go `jsonic` command

A learning-by-doing walkthrough for the **Go port** of the `jsonic` CLI. By
the end you will have built the command and used it to turn relaxed-JSON text
into standard JSON — from arguments, from a file, and from a pipe.

This is the Go port of the TypeScript `jsonic` command (which is canonical).
It is a `package main` program — a command-line tool, not a Go library. The
import path is `github.com/tabnas/jsonic-cli/go`; the command lives at
`cmd/jsonic`.

## Step 1 — Get the code and build

The Go module wraps unpublished `@tabnas` Go siblings (jsonic, parser, json,
debug) via `replace` directives, so clone them as siblings of this repo
(see `go/go.mod`). With those in place, from the repo's `go/` directory:

```bash
cd go
go build ./...
```

You can run the command without installing it:

```bash
go run ./cmd/jsonic --help
```

You should see a usage message that begins `A JSON parser that isn't
strict.` To install a `jsonic` binary onto your `PATH` instead:

```bash
go install github.com/tabnas/jsonic-cli/go/cmd/jsonic@latest
```

The rest of this tutorial uses `go run ./cmd/jsonic`; substitute `jsonic` if
you installed the binary.

## Step 2 — Parse your first input

Pass a relaxed-JSON fragment as an argument. No quotes around the key, no
surrounding braces:

```bash
go run ./cmd/jsonic a:1
```

```
{"a":1}
```

Relaxed in, strict out: that is the whole idea.

## Step 3 — Make it readable

By default the output is compact. Add `-n` (short for `--nice`) to indent it:

```bash
go run ./cmd/jsonic -n a:1
```

```
{
  "a": 1
}
```

`-n` is sugar for `-o JSON.space=2`.

## Step 4 — Build a bigger object

Pass several fragments; each is parsed and merged into one result. Nest with
`:` separators:

```bash
go run ./cmd/jsonic a:b:1 a:c:2
```

```
{"a":{"b":1,"c":2}}
```

Quote any fragment with shell-special characters (`[`, `{`, `,`, spaces):

```bash
go run ./cmd/jsonic 'a:1' 'b:[2]' 'c:{x:1}'
```

```
{"a":1,"b":[2],"c":{"x":1}}
```

## Step 5 — Parse a file

Create a small relaxed-JSON file and parse it with `-f` (short for
`--file`):

```bash
printf 'bar:1' > foo.jsonic
go run ./cmd/jsonic -f foo.jsonic
```

```
{"bar":1}
```

Argument fragments are merged over the file (arguments win):

```bash
go run ./cmd/jsonic -f foo.jsonic zed:2
```

```
{"bar":1,"zed":2}
```

## Step 6 — Pipe from another command

With no source arguments, the command reads from STDIN:

```bash
echo a:1 | go run ./cmd/jsonic
```

```
{"a":1}
```

## What you have learned

You can now build and run the Go `jsonic` command, parse relaxed-JSON from
arguments, pretty-print it, merge fragments, parse a file, and pipe via
STDIN.

## Where to go next

- [guide.md](guide.md) — task recipes.
- [reference.md](reference.md) — every flag, exit code, and the
  STDIN/STDOUT contract.
- [concepts.md](concepts.md) — how the CLI relates to the engine, plus a
  "Differences from the TS version" section.
