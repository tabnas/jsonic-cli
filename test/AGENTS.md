# Agents Guide — shared spec fixtures

`spec/*.tsv` holds the cross-runtime conformance fixtures. Both runtimes
auto-discover and run **every** file in this directory, so a change here
affects TypeScript and Go together — edit with that in mind.

## Format

Tab-separated, one case per line, with a header row naming the columns.
Blank lines are skipped, and so are comment lines — a line starting with
`#` that contains no tab. (A data row always has at least one tab.)

| Column | Meaning |
|---|---|
| `argv` | The CLI arguments, as a JSON array of strings. The two leading placeholders the TS entry point expects (`node`, script) are supplied by the runner, not the fixture. |
| `stdout` | The first printed line, as a JSON string — or `CONTAINS:<substring>` when only a fragment is pinned (the help text). |
| `stdin` | Optional stdin text. Escapes `\n` `\r` `\t` `\\` are decoded; empty means no stdin. |

`argv` and `stdout` are **not** escape-decoded — they are raw JSON, so
JSON's own escape rules apply.

## Who runs what

- TypeScript: `ts/test/parity.test.js` — reads `../../test/spec`.
- Go: `go/cli/parity_test.go` — `TestSpec` globs `../../test/spec/*.tsv`.

Both discover files by directory listing: adding a `.tsv` here runs it in
both runtimes without touching either runner.

Cases that turn on how a runtime loads code or reads the filesystem stay
out of here, in `ts/test/cli.test.js` and `go/cli/run_test.go`: the `-p`
plugin fixtures (JS modules resolved by `require` vs a compiled-in Go
registry) and the `-f` file fixtures (`./test/foo.jsonic` vs
`testdata/foo.jsonic`). Those files document each adaptation.

## Rules

- Prefer adding a fixture here over a one-off in-language assertion when a
  case is expressible as argv (+ stdin) → stdout. That is what keeps the two
  runtimes honest against each other.
- TypeScript is canonical. If the two runtimes disagree, the TS behaviour is
  the expected value — unless Go has exposed a genuine TS defect, in which
  case fix TS first and pin the corrected behaviour here.
- A new fixture must pass in BOTH runtimes: run `go test ./...` (from `go/`)
  and `npm test` (from `ts/`) before considering it done.
