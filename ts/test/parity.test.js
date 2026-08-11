/* Copyright (c) 2013-2026 Richard Rodger and other contributors, MIT License */
'use strict'

// Cross-runtime conformance, driven by the shared `test/spec/*.tsv` fixtures
// at the repo root (see ../../test/AGENTS.md).
//
// The fixture loader, the escape codec and the comparison come from
// @tabnas/support, whose Go half `go/cli/parity_test.go` uses to run the
// SAME files — so the two implementations cannot drift without one of them
// going red, and neither can the two loaders.
//
// The Go side drives them through `support.Runner`; this side cannot,
// because running the CLI is ASYNCHRONOUS and the runner's row loop is
// synchronous in both languages (Go has no async to be). So the loop is
// here — but the loading, the escape decoding and the value comparison are
// the shared ones, which is where the drift was.

const { describe, it } = require('node:test')
const assert = require('node:assert')

const {
  findSpecDir, loadSpecDir, equalValue, formatValue, parseExpect,
} = require('@tabnas/support')

const JsonicCli = require('../dist/jsonic-cli')

// Help text is long and version-ish; those rows pin a SUBSTRING of the
// output rather than the whole of it.
const CONTAINS = 'CONTAINS:'

// The CLI takes argv with two leading placeholders (node, script) and a
// console-like object; `test$` carries the stdin text (or `true` for none).
async function runCli(argv, stdin) {
  const d = { log: [], dir: [] }
  const cn = {
    // An absent stdin column means "nothing piped in": pass the empty
    // string, exactly what the Go runner passes, and what the real CLI
    // sees at a terminal. It must stay a string — a truthy non-string
    // makes read_stdin() fall through to the real process.stdin, which
    // never ends under `node --test`.
    test$: undefined === stdin ? '' : stdin,
    d,
    // Render each line the way console.log would — arguments stringified and
    // space-joined. The Go logger captures rendered text, so returning a raw
    // JS value here (e.g. `undefined` for an empty parse) would report a
    // false mismatch against a fixture holding the string "undefined".
    log: (...rest) => d.log.push(rest.map(String).join(' ')),
    dir: (...rest) => d.dir.push(rest),
  }
  await JsonicCli.run([0, 0, ...argv], cn)
  return d.log.length ? d.log[0] : ''
}

for (const spec of loadSpecDir(findSpecDir(__dirname))) {
  describe('spec: ' + spec.file, () => {
    for (const row of spec.rows) {
      const argv = row.named('argv')
      const expected = row.named('stdout')
      const stdin = row.unescNamed('stdin')

      it(`row ${row.line}: ${argv}`, async () => {
        const got = await runCli(JSON.parse(argv), '' === stdin ? undefined : stdin)

        if (expected.startsWith(CONTAINS)) {
          const want = expected.slice(CONTAINS.length)
          assert.ok(got.includes(want),
            `${row.where()}: output lacks ${JSON.stringify(want)}\n${got}`)
          return
        }

        const want = parseExpect(expected)
        assert.ok(equalValue(got, want),
          `${row.where()}\n  got:      ${formatValue(got)}` +
          `\n  expected: ${formatValue(want)}`)
      })
    }
  })
}
