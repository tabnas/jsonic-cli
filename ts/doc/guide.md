# How-to guide: `jsonic` recipes

Task-focused recipes for common jobs. Each is self-contained — skim for the
one you need. For the exhaustive flag list see [reference.md](reference.md);
for *why* it works this way see [concepts.md](concepts.md).

All examples use `bash`. Quote any fragment containing shell-special
characters (`[`, `{`, `,`, spaces, `$`).

## Convert relaxed JSON to strict JSON

The basic job. Pass the source as an argument:

```bash
jsonic a:1
```

```
{"a":1}
```

Or read it from STDIN by piping (give no source arguments):

```bash
echo a:1 | jsonic
```

```
{"a":1}
```

## Pretty-print the output

Use `-n` (alias `--nice`), which is sugar for `-o JSON.space=2`:

```bash
jsonic -n a:1
```

```
{
  "a": 1
}
```

For a different indent — say four spaces — set `JSON.space` directly:

```bash
jsonic -o JSON.space=4 a:1
```

```
{
    "a": 1
}
```

## Merge several config fragments into one object

Pass multiple source arguments. They are deep-merged left to right, with
later fragments taking precedence on conflicts:

```bash
jsonic a:b:1 a:c:2
```

```
{"a":{"b":1,"c":2}}
```

This is handy for layering defaults and overrides on the command line:

```bash
jsonic 'port:8080,host:localhost' 'host:0.0.0.0'
```

```
{"port":8080,"host":"0.0.0.0"}
```

The later `host:0.0.0.0` wins over the earlier `host:localhost`.

## Merge a base file with command-line overrides

Files are parsed first and have *lower* precedence than argument fragments,
so arguments override the file:

```bash
printf 'bar:1' > foo.jsonic
jsonic -f foo.jsonic bar:99
```

```
{"bar":99}
```

Add new keys the same way:

```bash
jsonic -f foo.jsonic zed:2
```

```
{"bar":1,"zed":2}
```

## Merge two files

Repeat `-f`. Files are merged in order, later files winning:

```bash
printf 'bar:1' > foo.jsonic
printf 'qaz:2' > bar.jsonic
jsonic -f foo.jsonic -f bar.jsonic
```

```
{"bar":1,"qaz":2}
```

## Layer a file, a pipe, and arguments together

You can combine all three source kinds. The precedence (highest wins) is:
argument fragments, then STDIN, then files. Use `-` to force STDIN to be
read even when you also pass arguments:

```bash
printf 'bar:1' > foo.jsonic
echo 'mid:2' | jsonic -f foo.jsonic - top:3
```

```
{"bar":1,"mid":2,"top":3}
```

## Keep only certain keys (filter the output)

`JSON.stringify` accepts a *replacer* whitelist. Set `JSON.replacer` to a
list of key names to keep:

```bash
jsonic -o 'JSON.replacer=[b]' 'a:1,b:2'
```

```
{"b":2}
```

A single scalar is wrapped into a one-element whitelist automatically:

```bash
jsonic -o JSON.replacer=b 'a:1,b:2'
```

```
{"b":2}
```

## Turn off a lexer feature (keep numbers as strings)

Engine options pass straight through with `-o`. For example, disabling the
number lexer leaves numeric-looking values as strings:

```bash
jsonic -o number.lex=false a:1
```

```
{"a":"1"}
```

Any `@tabnas/jsonic` option works this way (the option name is a dotted
path; the value is itself parsed by jsonic, so types come through).

## Load a plugin

Use `-p` (alias `--plugin`) with a module reference. Bare names also try the
`@tabnas/` scope, so `-p csv` resolves `@tabnas/csv` if installed. Plugin
options go under `plugin.<name>.<option>`:

```bash
# requires: npm install @tabnas/csv
jsonic -p csv -o plugin.csv.record.separators=^ "a,b^1,2"
```

```
[{"a":"1","b":"2"}]
```

## Trace a parse for debugging

`-d` (alias `--debug`) loads the `@tabnas/debug` plugin, prints a grammar
description, and enables a token-by-token trace:

```bash
jsonic -d -o plugin.debug.trace=true a:1
```

The trace is printed before the final JSON line. `@tabnas/debug` is a peer
dependency, so it is installed alongside the CLI.

## Stop flag parsing (treat everything after as source)

A bare `--` turns off flag handling; every token after it is treated as
source text, even if it starts with `-`:

```bash
jsonic -- -o
```

This parses the literal string `-o` instead of treating it as the option
flag. (Note: an *unrecognised* `-`-prefixed token before `--` is also
treated as source, not an error.)

## Verified examples (run by the test harness)

The fragments below are executed by the doc-example test harness. They drive
the CLI's own `run()` entry point — the same function the `jsonic` binary
calls — with a captured console, so the asserted output is exactly what the
command prints. (The CLI has no library API beyond `run`; these blocks exist
to verify the recipes above against real behaviour.)

```js
const { run } = require('@tabnas/jsonic-cli')

// Drive the CLI in-process and return its last printed line.
async function jsonic(args, stdin) {
  const lines = []
  const cn = { test$: stdin == null ? true : stdin, log: (...a) => lines.push(a.join(' ')), error: () => {} }
  await run([null, null, ...args], cn)
  return lines[lines.length - 1]
}

await jsonic(['a:1'])                                  // => '{"a":1}'
await jsonic(['a:b:1', 'a:c:2'])                       // => '{"a":{"b":1,"c":2}}'
await jsonic(['-n', 'a:1'])                            // => '{\n  "a": 1\n}'
await jsonic(['-o', 'JSON.replacer=[b]', 'a:1,b:2'])   // => '{"b":2}'
await jsonic(['-o', 'number.lex=false', 'a:1'])        // => '{"a":"1"}'
await jsonic([], 'x:1')                                // => '{"x":1}'
```
