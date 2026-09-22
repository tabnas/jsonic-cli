/* Copyright (c) 2013-2026 Richard Rodger and other contributors, MIT License */
'use strict'

// The recorded divergences from this, the canonical command — the
// TypeScript side of the ledger.
//
// `test/divergent.tsv` at the repository root is the executable register:
// one row per input where a port produces a different RESULT, one column
// per runtime. This suite asserts the `ts` column, `go/cli/divergence_test.go`
// asserts `go` and `rs/tests/divergence_test.rs` asserts `rust`, so every
// column of the file is run by the runtime it describes.
//
// The property that matters is not that a regression fails but that a FIX
// fails too: when a port is repaired to agree, the register still claims a
// difference, the suite goes red and names the row to delete. That is what
// a prose list cannot do, and ../../DIVERGENCE.md points here rather than
// carrying the claims itself.
//
// The canonical column is the one it costs most to leave unrun. Every
// other column is measured AGAINST it, so a stale `ts` cell makes each of
// the other suites assert the wrong difference while staying green.
//
// The loop is local rather than `DivergenceRegister` from @tabnas/support,
// for the reason parity.test.js keeps its own: running this CLI is
// ASYNCHRONOUS and the shared runner's row loop is synchronous in both
// languages. The loading, the escape decoding, the expectation vocabulary
// and the value comparison are all the shared ones, which is where drift
// would otherwise live.

const { describe, it } = require('node:test')
const assert = require('node:assert')
const path = require('node:path')

const {
  findSpecDir, loadSpec, equalValue, formatValue, parseExpect,
  isErrorExpect, errorCode,
} = require('@tabnas/support')

const JsonicCli = require('../dist/jsonic-cli')

// Which columns of the register are runtimes, and which one is this
// suite's. Named rather than read off the header, so that an `issue` or
// `stdin` column is never mistaken for a runtime whose answer happens to
// be a sentence.
const RUNTIMES = ['ts', 'go', 'rust']
const MINE = 'ts'

// The register sits BESIDE spec/, deliberately: everything in spec/ is
// discovered and run by all three suites, and a row one of them cannot
// pass would break that suite.
const REGISTER = path.join(path.dirname(findSpecDir(__dirname)), 'divergent.tsv')

// One register row: the first printed entry, or `ERROR:<code>` when the
// run rejects, which is the vocabulary an expectation cell is written in.
async function outcome(argv, stdin) {
  const d = { log: [] }
  const cn = {
    // A string, always: a truthy non-string makes read_stdin() fall
    // through to the real process.stdin, which never ends under
    // `node --test`. parity.test.js says the same at more length.
    test$: undefined === stdin ? '' : stdin,
    d,
    log: (...rest) => d.log.push(rest.map(String).join(' ')),
    dir: () => {},
  }
  try {
    await JsonicCli.run([0, 0, ...argv], cn)
  }
  catch (e) {
    return 'ERROR:' + (e.code || e.message)
  }
  return d.log.length ? d.log[0] : ''
}

// Do two expectation CELLS mean the same thing? Compared by meaning
// rather than by bytes, as the shared register does: `1` and `1.0` are
// one expectation, and a row whose cells differ only that way records no
// divergence at all.
function sameExpectation(a, b) {
  if (a === b) {
    return true
  }
  if (isErrorExpect(a) || isErrorExpect(b)) {
    return isErrorExpect(a) && isErrorExpect(b) && errorCode(a) === errorCode(b)
  }
  try {
    return equalValue(parseExpect(a), parseExpect(b))
  }
  catch {
    return false
  }
}

// Does an outcome match a cell? An ERROR cell is compared by code; every
// other cell goes through the shared expectation reader.
function matches(got, cell) {
  if (isErrorExpect(cell)) {
    return isErrorExpect(got) && errorCode(got) === errorCode(cell)
  }
  if (isErrorExpect(got)) {
    return false
  }
  return equalValue(got, parseExpect(cell))
}

const spec = loadSpec(REGISTER)

describe('divergence register: ' + spec.file, () => {
  it('has rows', () => {
    assert.ok(0 < spec.rows.length,
      `${REGISTER} has no rows. An empty register means nothing diverges: ` +
      'delete the file and its three runners rather than leaving it here.')
  })

  for (const row of spec.rows) {
    const argv = row.named('argv')
    const stdin = row.unescNamed('stdin')

    it(`row ${row.line}: ${argv}`, async () => {
      const mine = row.named(MINE)
      const others = RUNTIMES
        .filter((name) => MINE !== name)
        .map((name) => ({ name, cell: row.named(name) }))

      // 1. Does this row record a divergence at all? A row whose every
      // runtime column means the same thing asserts nothing and would
      // pass forever, which is the shape of the prose claims the register
      // replaces.
      assert.ok(others.some((other) => !sameExpectation(other.cell, mine)),
        `${row.where()}: every runtime column means ${JSON.stringify(mine)}, ` +
        'so this row records no divergence and can never fail meaningfully. ' +
        'Delete it, or correct the cells to what the runtimes actually do.')

      const got = await outcome(JSON.parse(argv), '' === stdin ? undefined : stdin)

      // 2. Does the canonical command still do what the register says?
      if (matches(got, mine)) {
        return
      }

      // 3. When it does not, and it now produces what ANOTHER runtime's
      // cell says, the divergence is CLOSED rather than regressed, and
      // the row has to go.
      const closed = others.find((other) => matches(got, other.cell))
      assert.fail(closed
        ? `${row.where()}: this divergence is CLOSED. The canonical command ` +
          `now produces the ${closed.name} column's ${formatValue(got)}. ` +
          'Delete this row, and the paragraph in DIVERGENCE.md that ' +
          'describes it.'
        : `${row.where()}: the ts column is stale.` +
          `\n  got:      ${formatValue(got)}` +
          `\n  expected: ${formatValue(parseExpect(mine))}`)
    })
  }
})
