/* Copyright (c) 2013-2026 Richard Rodger and other contributors, MIT License */
'use strict'

// Cross-runtime conformance, driven by the shared `test/spec/*.tsv` fixtures
// at the repo root — the same convention @tabnas/parser and @tabnas/abnf use
// (see ../../test/AGENTS.md).
//
// `go/cli/parity_test.go` discovers and runs the SAME files, so the two
// implementations cannot drift without one of them going red.

const { describe, it } = require('node:test')
const assert = require('node:assert')
const Fs = require('node:fs')
const Path = require('node:path')

const JsonicCli = require('../dist/jsonic-cli')

const specDir = Path.join(__dirname, '..', '..', 'test', 'spec')

// Decode the escape set used in non-JSON columns. Kept byte-identical to the
// Go loader so both runtimes feed the CLI the exact same stdin text.
function unescape(s) {
  if (!s.includes('\\')) return s
  let out = ''
  for (let i = 0; i < s.length; i++) {
    const c = s[i]
    if ('\\' === c && i + 1 < s.length) {
      const n = s[i + 1]
      if ('n' === n) { out += '\n'; i++; continue }
      if ('r' === n) { out += '\r'; i++; continue }
      if ('t' === n) { out += '\t'; i++; continue }
      if ('\\' === n) { out += '\\'; i++; continue }
    }
    out += c
  }
  return out
}

function loadSpec(file) {
  const body = Fs.readFileSync(Path.join(specDir, file), 'utf8')
  const lines = body.split(/\r?\n/)
  const rows = []
  // Line 1 is the header naming the columns.
  for (let i = 1; i < lines.length; i++) {
    const raw = lines[i]
    // A comment line starts with '#' and has no tab; a data row always has
    // at least one (argv + stdout), so '#'-leading data still works.
    if ('' === raw || (raw.startsWith('#') && !raw.includes('\t'))) continue
    const cols = raw.split('\t')
    if (cols.length < 2) {
      throw new Error(`${file}:${i + 1}: expected at least 2 tab-separated columns`)
    }
    rows.push({
      line: i + 1,
      argv: JSON.parse(cols[0]),
      expected: cols[1],
      stdin: undefined === cols[2] || '' === cols[2]
        ? undefined : unescape(cols[2]),
    })
  }
  return rows
}

// The CLI takes argv with two leading placeholders (node, script) and a
// console-like object; `test$` carries the stdin text (or `true` for none).
async function runCli(argv, stdin) {
  const d = { log: [], dir: [] }
  const cn = {
    test$: undefined === stdin ? true : stdin,
    d,
    log: (...rest) => d.log.push(rest),
    dir: (...rest) => d.dir.push(rest),
  }
  await JsonicCli.run([0, 0, ...argv], cn)
  return d.log.length ? d.log[0][0] : ''
}

function runSpec(file) {
  const rows = loadSpec(file)
  describe('spec: ' + file, () => {
    assert.ok(0 < rows.length, file + ': no cases')
    for (const row of rows) {
      it(`row ${row.line}: ${JSON.stringify(row.argv)}`, async () => {
        const got = await runCli(row.argv, row.stdin)

        // Help text is long and version-ish; those rows pin a substring.
        if (row.expected.startsWith('CONTAINS:')) {
          const want = row.expected.slice('CONTAINS:'.length)
          assert.ok(got.includes(want),
            `${file}:${row.line}: output lacks ${JSON.stringify(want)}`)
          return
        }

        assert.strictEqual(got, JSON.parse(row.expected), `${file}:${row.line}`)
      })
    }
  })
}

// Auto-discover every fixture: adding a .tsv runs it in both runtimes
// without touching either runner.
for (const file of Fs.readdirSync(specDir).sort()) {
  if (file.endsWith('.tsv')) runSpec(file)
}
