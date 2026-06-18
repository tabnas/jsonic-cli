# How-to guide: Go `jsonic` recipes

Task-focused recipes for the **Go port** of the `jsonic` CLI. Each is
self-contained. For the full flag list see [reference.md](reference.md); for
*why* it works this way see [concepts.md](concepts.md).

Examples assume you are in the repo's `go/` directory and run the command
with `go run ./cmd/jsonic`. If you installed a binary
(`go install github.com/tabnas/jsonic-cli/go/cmd/jsonic@latest`), substitute
`jsonic`. Quote any fragment containing shell-special characters.

## Convert relaxed JSON to strict JSON

```bash
go run ./cmd/jsonic a:1
```

```
{"a":1}
```

Or from STDIN (give no source arguments):

```bash
echo a:1 | go run ./cmd/jsonic
```

```
{"a":1}
```

## Pretty-print the output

Use `-n` (alias `--nice`), sugar for `-o JSON.space=2`:

```bash
go run ./cmd/jsonic -n a:1
```

```
{
  "a": 1
}
```

For four-space indent set `JSON.space` directly:

```bash
go run ./cmd/jsonic -o JSON.space=4 a:1
```

```
{
    "a": 1
}
```

## Merge several config fragments into one object

Pass multiple source arguments. They are deep-merged left to right, later
fragments winning on conflict:

```bash
go run ./cmd/jsonic a:b:1 a:c:2
```

```
{"a":{"b":1,"c":2}}
```

## Merge a base file with command-line overrides

Files have lower precedence than argument fragments, so arguments override:

```bash
printf 'bar:1' > foo.jsonic
go run ./cmd/jsonic -f foo.jsonic bar:99
```

```
{"bar":99}
```

Add new keys the same way:

```bash
go run ./cmd/jsonic -f foo.jsonic zed:2
```

```
{"bar":1,"zed":2}
```

## Merge two files

Repeat `-f`. Files merge in order, later files winning:

```bash
printf 'bar:1' > foo.jsonic
printf 'qaz:2' > bar.jsonic
go run ./cmd/jsonic -f foo.jsonic -f bar.jsonic
```

```
{"bar":1,"qaz":2}
```

## Layer a file, a pipe, and arguments together

Precedence, highest wins: arguments, then STDIN, then files. Use `-` to
force STDIN to be read alongside arguments:

```bash
printf 'bar:1' > foo.jsonic
echo 'mid:2' | go run ./cmd/jsonic -f foo.jsonic - top:3
```

```
{"bar":1,"mid":2,"top":3}
```

## Keep only certain keys (filter the output)

Set `JSON.replacer` to a whitelist of key names:

```bash
go run ./cmd/jsonic -o 'JSON.replacer=[b]' 'a:1,b:2'
```

```
{"b":2}
```

A single scalar is wrapped into a one-element whitelist:

```bash
go run ./cmd/jsonic -o JSON.replacer=b 'a:1,b:2'
```

```
{"b":2}
```

## Turn off a lexer feature (keep numbers as strings)

Engine options pass through with `-o`:

```bash
go run ./cmd/jsonic -o number.lex=false a:1
```

```
{"a":"1"}
```

## Trace a parse for debugging

`-d` (alias `--debug`) installs the first-party `github.com/tabnas/debug/go`
plugin, prints a grammar description, and enables a parse trace:

```bash
go run ./cmd/jsonic -d a:1
```

The description and `=== PARSE ===` marker print before the final JSON line.
(The Go debug plugin is driven by its own `trace` option, which `-d` sets;
the TS CLI uses meta `log=-1` for the same purpose.)

## Plugins: an important difference from TS

`-p`/`--plugin` exists for parity, **but the production Go binary cannot load
a plugin by name** — Go has no runtime `require`. The binary's plugin
registry is empty, so naming a plugin fails:

```bash
go run ./cmd/jsonic -p csv 'a:1'
```

```
Plugin not found: csv
```

and the command exits non-zero. Plugins must be **compiled in** via a
registry; the test suite injects its fixtures this way. See
[concepts.md](concepts.md#differences-from-the-ts-version) for the rationale.

## Stop flag parsing

A bare `--` treats every following token as source text:

```bash
go run ./cmd/jsonic -- -o
```

This parses the literal string `-o`. An *unrecognised* `-`-prefixed token
before `--` is also treated as source (not an error), so
`go run ./cmd/jsonic --not-an-arg a:1` prints `{"a":1}`.

## Verifying

`go` blocks in these docs are illustrative and not executed. To verify Go
behaviour, run the test suite, which ports the TypeScript CLI tests
one-for-one:

```bash
cd go
go test ./...
```
