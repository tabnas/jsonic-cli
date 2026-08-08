/* Copyright (c) 2013-2022 Richard Rodger and other contributors, MIT License */
'use strict'

const { describe, it } = require('node:test')
const assert = require('node:assert')

const JsonicCli = require('../dist/jsonic-cli')
const jr = async (...rest) => await JsonicCli.run(...rest)

describe('cli', function () {
  it('basic', async () => {
    let cn = make_cn()
    await jr([0, 0, 'a:1'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"a":1}')

    cn = make_cn()
    await jr([0, 0, '-o', 'number.lex=false', 'a:1'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"a":"1"}')
  })

  it('args', async () => {
    let cn = make_cn()
    await jr([0, 0, '-h'], cn)
    assert.deepEqual(cn.d.log[0][0].includes('Usage:'), true)

    cn = make_cn()
    await jr([0, 0, '--help'], cn)
    assert.deepEqual(cn.d.log[0][0].includes('Usage:'), true)

    cn = make_cn()
    await jr([0, 0, 'a:1', 'b:[2]', 'c:{x:1}'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"a":1,"b":[2],"c":{"x":1}}')

    // // TODO: `{zed:2}` should work too!
    cn = make_cn()
    await jr([0, 0, '-f', './test/foo.jsonic', 'zed:2'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"bar":1,"zed":2}')

    // Two --file sources, merged last-wins (the Go port has the same case
    // over go/cli/testdata). The suite runs on `node --test`, not jest, so
    // the old "jest borks this" exclusion no longer applies.
    cn = make_cn()
    await jr([0, 0, '-f', './test/foo.jsonic', '--file', './test/bar.jsonic'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"bar":1,"qaz":2}')

    cn = make_cn()
    await jr([0, 0, '--not-an-arg-so-ignored', 'a:1'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"a":1}')

    cn = make_cn()
    cn.test$ = '{a:1}'
    await jr([0, 0], cn)
    // console.log(cn.d.log)
    assert.deepEqual(cn.d.log[0][0], '{"a":1}')

    cn = make_cn()
    cn.test$ = '{a:1}'
    await jr([0, 0, '-'], cn)
    // console.log(cn.d.log)
    assert.deepEqual(cn.d.log[0][0], '{"a":1}')

    cn = make_cn()
    cn.test$ = '{a:1}'
    await jr([0, 0, '-', 'b:2'], cn)
    // console.log(cn.d.log)
    assert.deepEqual(cn.d.log[0][0], '{"a":1,"b":2}')
  })

  it('bad-args', async () => {
    let cn = make_cn()
    await jr([0, 0, '-f', { bad: 1 }, 'a:1'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"a":1}')

    cn = make_cn()
    await jr([0, 0, '-f', '', 'a:1'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"a":1}')

    cn = make_cn()
    await jr([0, 0, '-o', '', 'a:1'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"a":1}')

    cn = make_cn()
    await jr([0, 0, '-o', '=', 'a:1'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"a":1}')

    cn = make_cn()
    await jr([0, 0, '-o', 'bad=', 'a:1'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"a":1}')

    // An unresolvable -p reference rejects with require's original error
    // (handle_plugins rethrows it after the @tabnas/<name> retry also
    // fails). The Go port exits 1 on the same input — see
    // go/cli/run_test.go TestBuiltinPlugins.
    cn = make_cn()
    await assert.rejects(
      () => jr([0, 0, '-p', 'no-such-plugin', 'a:1'], cn),
      (e) => 'MODULE_NOT_FOUND' === e.code &&
        e.message.includes("Cannot find module 'no-such-plugin'"),
    )
  })

  // --debug / -d installs @tabnas/debug, prints the grammar description
  // ahead of the parse, and still emits the JSON result last. The trace
  // volume is engine-dependent, so pin the two ends, not a line count.
  it('debug', async () => {
    let cn = make_cn()
    await jr([0, 0, '-d', 'a:1'], cn)
    assert.deepEqual(cn.d.log[0][0].includes('=== PARSE ==='), true)
    assert.deepEqual(1 < cn.d.log.length, true)
    assert.deepEqual(cn.d.log[cn.d.log.length - 1][0], '{"a":1}')

    // The long form, plus options that reach both the debug plugin and the
    // engine, and `--` ending flag parsing.
    cn = make_cn()
    await jr(
      [0, 0, '--debug', '-o', 'debug.maxlen=11',
        '--option', 'value.lex=false', '--', 'a:true'],
      cn,
    )
    assert.deepEqual(cn.d.log[0][0].includes('=== PARSE ==='), true)
    assert.deepEqual(cn.d.log[cn.d.log.length - 1][0], '{"a":"true"}')
  })

  it('plugin', async () => {
    let cn = make_cn()
    await jr([0, 0, '-p', '../test/p0', '-o', 'plugin.p0.x=0', 'a:X'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"a":0}')

    cn = make_cn()
    await jr(
      [
        0,
        0,
        '-p',
        '../test/p0',
        '-o',
        'plugin.p0.x=0',
        '-o',
        'plugin.p0.s=W',
        'a:W',
      ],
      cn,
    )
    assert.deepEqual(cn.d.log[0][0], '{"a":0}')

    cn = make_cn()
    await jr([0, 0, '-o', 'plugin.p1.y=1', '-p', '../test/p1', 'a:Y'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"a":1}')

    cn = make_cn()
    await jr(
      [
        0,
        0,
        '-o',
        'plugin.p0.x=0',
        '-p',
        '../test/p0',
        '-o',
        'plugin.p1.y=1',
        '-p',
        '../test/p1',
        'a:X,b:Y',
      ],
      cn,
    )
    assert.deepEqual(cn.d.log[0][0], '{"a":0,"b":1}')

    cn = make_cn()
    await jr([0, 0, '-p', '../test/p2', '-o', 'plugin.p2.z=2', 'a:Z'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"a":2}')

    cn = make_cn()
    await jr(
      [0, 0, '-p', '../test/pa-qa.js', '-o', 'plugin.paqa.q=3', 'a:Q'],
      cn,
    )
    assert.deepEqual(cn.d.log[0][0], '{"a":3}')

  })

  it('stringify', async () => {
    let cn = make_cn()
    await jr([0, 0, '-o', 'JSON.space=2', 'a:1'], cn)
    assert.deepEqual(cn.d.log[0][0], '{\n  "a": 1\n}')

    cn = make_cn()
    await jr([0, 0, '-n', 'a:1'], cn)
    assert.deepEqual(cn.d.log[0][0], '{\n  "a": 1\n}')

    cn = make_cn()
    await jr([0, 0, '-o', 'JSON.replacer=[b]', 'a:1,b:2'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"b":2}')

    cn = make_cn()
    await jr([0, 0, '-o', 'JSON.replacer=b', 'a:1,b:2'], cn)
    assert.deepEqual(cn.d.log[0][0], '{"b":2}')
  })
})

function make_cn() {
  let d = {
    log: [],
    dir: [],
  }
  return {
    // `test$` is the STDIN body read_stdin() returns instead of touching
    // process.stdin. The default is the EMPTY STRING, i.e. "nothing piped
    // in" — what the real CLI sees at a terminal (isTTY -> ''), and what
    // the Go tests pass as their stdin argument. It must stay a string: a
    // truthy non-string falls through to the real process.stdin, which
    // under `node --test` never ends, so any case with no positional
    // source (e.g. --file only) would hang instead of run.
    test$: '',
    d,
    log: (...rest) => d.log.push(rest),
    dir: (...rest) => d.dir.push(rest),
  }
}
