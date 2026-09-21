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

- TypeScript: `ts/test/parity.test.js` — the shared loader, a local loop.
- Go: `go/cli/parity_test.go` — `support.Runner{...}.Dir(t, dir)`.
- Rust: `rs/tests/parity_test.rs` — `Runner::new_with_row(..).dir(&dir)`.

All three read the fixtures with
[`@tabnas/support`](https://github.com/tabnas/support) and its Go and
Rust halves — the same loader, escape codec and value comparison, so they
cannot drift from each other.

The Go side also uses the shared ROW LOOP; the TypeScript side cannot,
because running the CLI there is asynchronous and the loop is synchronous
in both languages (Go has no async to be). That is the one asymmetry, and
it is confined to the loop: everything the loop reads and compares with is
shared.

All three discover files by directory listing: adding a `.tsv` here runs
it in every runtime without touching any runner. An empty fixture, and a
spec directory with no fixtures in it, both **fail**.

The comparison is per PRINTED ENTRY, not per line. One entry is one
`console.log` in the canonical command, and the help text, the grammar
description and indented JSON each span several physical lines while
staying one entry. The Rust runner reads them through
`tabnas_jsonic_cli::capture` for that reason.

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
- A new fixture must pass in EVERY runtime: run `go test ./...` (from
  `go/`), `npm test` (from `ts/`) and `cargo test --all-targets` (from
  `rs/`) before considering it done.

## `divergent.tsv`

[`divergent.tsv`](divergent.tsv) sits HERE rather than in `spec/`,
deliberately. Everything in `spec/` is discovered and run by all three
suites, so a row one runtime cannot pass breaks that runtime's build. The
register is the opposite: it records inputs where the runtimes DISAGREE,
one column per runtime, and the port that owns a column asserts it.
`rs/tests/divergence_test.rs` runs it for `rust`.

A row that gets FIXED fails as loudly as one that regresses, and must
then be deleted. See [`../DIVERGENCE.md`](../DIVERGENCE.md) for the shape
of each recorded disagreement and the measurement behind it.
