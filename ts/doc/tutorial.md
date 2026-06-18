# Tutorial: parse your first input with `jsonic`

This is a learning-by-doing walkthrough. By the end you will have installed
the `jsonic` command and used it to turn relaxed-JSON text into standard
JSON, from arguments, from a file, and from a pipe. One happy path, start to
finish.

`jsonic` is the command-line front end for
[`@tabnas/jsonic`](https://github.com/tabnas/jsonic) — a JSON parser that
isn't strict. You write JSON without the ceremony (no quotes around keys, no
required braces at the top level) and `jsonic` prints back clean,
standards-compliant JSON.

## Step 1 — Install

Install the package globally so the `jsonic` command is on your `PATH`:

```bash
npm install -g @tabnas/jsonic-cli
```

The package declares `@tabnas/jsonic` (and the optional `@tabnas/debug`) as
peer dependencies; npm 7+ installs them automatically.

Check it works:

```bash
jsonic --help
```

You should see a usage message that begins `A JSON parser that isn't
strict.`

## Step 2 — Parse your first input

Pass a relaxed-JSON fragment as an argument. Notice there are no quotes
around the key and no surrounding braces:

```bash
jsonic a:1
```

```
{"a":1}
```

`jsonic` parsed `a:1` and printed standard JSON. That is the whole core
idea: relaxed in, strict out.

## Step 3 — Make it readable

By default the output is compact (one line). Add `-n` (short for `--nice`)
to indent it:

```bash
jsonic -n a:1
```

```
{
  "a": 1
}
```

`-n` is just a shortcut for `-o JSON.space=2`, which sets the `space`
argument of `JSON.stringify`. You will meet `-o` properly in the reference.

## Step 4 — Build a bigger object

You can pass several fragments. Each is parsed and then *merged* into one
result. Dotted-style nesting works with `:` separators:

```bash
jsonic a:b:1 a:c:2
```

```
{"a":{"b":1,"c":2}}
```

The two fragments `a:b:1` and `a:c:2` were merged into a single nested
object. Merging is how `jsonic` combines multiple sources — you will use it
again with files and pipes.

A quick note on your shell: characters like spaces, `[`, `{` and `,` are
special to the shell, so quote any fragment that contains them:

```bash
jsonic 'a:1' 'b:[2]' 'c:{x:1}'
```

```
{"a":1,"b":[2],"c":{"x":1}}
```

## Step 5 — Parse a file

Create a small relaxed-JSON file:

```bash
printf 'bar:1' > foo.jsonic
```

Parse it with `-f` (short for `--file`):

```bash
jsonic -f foo.jsonic
```

```
{"bar":1}
```

You can mix a file with argument fragments. The argument is merged *over*
the file (arguments have higher precedence than files):

```bash
jsonic -f foo.jsonic zed:2
```

```
{"bar":1,"zed":2}
```

## Step 6 — Pipe from another command

If you give no source arguments, `jsonic` reads from STDIN. So you can pipe
input in:

```bash
echo a:1 | jsonic
```

```
{"a":1}
```

This makes `jsonic` a natural link in a shell pipeline — feed it relaxed
config or hand-written data and get JSON out the other end.

## What you have learned

You can now:

- install the `jsonic` command,
- parse relaxed-JSON from an argument (`jsonic a:1`),
- pretty-print it (`-n`),
- merge several fragments into one object,
- parse a file (`-f`),
- and pipe input via STDIN.

## Where to go next

- [guide.md](guide.md) — focused recipes for real tasks (config merging,
  filtering output keys, plugins).
- [reference.md](reference.md) — every flag, argument, exit behaviour, and
  the STDIN/STDOUT contract, stated precisely.
- [concepts.md](concepts.md) — how the CLI relates to the `@tabnas/jsonic`
  engine, and the design trade-offs behind merging and serialization.
