# Divergences

TypeScript is the canonical implementation; the Go and Rust ports track
it. This file records where a port produces a **different result for the
same input**.

## The live register is executable

[`test/divergent.tsv`](test/divergent.tsv) is the authority for anything
a fixture cell can express, not this page. Unlike a prose list it is
**run**, and every column is run by the runtime it describes:
`ts/test/divergent.test.js` asserts `ts`, `go/cli/divergence_test.go`
asserts `go` and `rs/tests/divergence_test.rs` asserts `rust`. So a
divergence that gets **fixed** fails the suite as loudly as one that
regresses, and the row must then be deleted. The canonical column is the
one it costs most to leave unrun: every other column is measured against
it, so a stale `ts` cell would make the other two suites assert the wrong
difference while staying green.

It sits beside `test/spec/` rather than inside it, deliberately. Every
`.tsv` in `test/spec/` is discovered and run by all three suites, and a
register belongs to the port that records it.

**Read the `.tsv` for what diverges today.** What follows is the shape of
each disagreement and the evidence behind it.

## Where each row was measured

Everything below was run in the session that added the Rust port, except
where a line says otherwise:

- **Rust** was run from `rs/target/debug/jsonic`, and the whole set is
  covered by `rs/tests/`.
- **Go** was run from a `go build ./cmd/jsonic` of this repository.
- **TypeScript could not be executed here.** `ts/dist` was not built and
  `ts/node_modules/@tabnas/*` was not linked, and `ts/package.json`
  declares `engines.node` as `">=24"` against a Node 22 runtime. Its
  column is read from `ts/src/jsonic-cli.ts` and `ts/bin/jsonic`, and
  each entry below names the code it is read from. Where the mechanism is
  plain JavaScript rather than tabnas code, it was measured directly
  under Node, and the entry says so.

A later session, the one that repaired the seven review findings, DID
run the canonical command: Node 22 strips the types off
`ts/src/jsonic-cli.ts` directly, and a published `@tabnas/jsonic` sits in
a sibling checkout's `node_modules`, so `run(argv, console)` could be
driven in process over the shared fixtures (44 of 44 rows green) and over
the inputs below. Every TypeScript column added by that session was
measured that way, and says so.

## Exit code on failure

| input | TypeScript | Go | Rust |
|---|---|---|---|
| `jsonic '}'` | message on standard error, exit **0** | message, exit 1 | message, exit 1 |
| `jsonic -f no-such-file` | message on standard error, exit **0** | message, exit 1 | message, exit 1 |
| `jsonic -p no-such-plugin a:1` | message on standard error, exit **0** | message, exit 1 | message, exit 1 |

`ts/bin/jsonic` is one line:

```js
require('../dist/jsonic-cli').run(process.argv, console).catch((e) => console.error(e.message))
```

The rejected promise is caught, the message printed, and nothing assigns
`process.exitCode`, so the process exits 0. That mechanism was measured
under Node on its own: a script of exactly that shape prints the message
and exits 0.

A command that reports a parse failure and then tells the shell it
succeeded cannot be scripted against. Both ports return 1, and the repair
belongs in the TypeScript bin, which should set `process.exitCode = 1`
before printing.

Not expressible as a register row: the register compares the first line
of standard output, and a failed run prints nothing there in any runtime.
Pinned by `a_failure_exits_nonzero_where_typescript_exits_zero` in
`rs/tests/divergence_test.rs`.

## Plugin loading

| input | TypeScript | Go | Rust |
|---|---|---|---|
| `jsonic -p no-such-plugin a:1` | `Cannot find module 'no-such-plugin'` | `Plugin not found: no-such-plugin` | `Plugin not found: no-such-plugin` |
| `jsonic -p ../test/p0 -o plugin.p0.x=0 a:X` | loads the module | registry miss unless compiled in | registry miss unless compiled in |

`handle_plugins` in `ts/src/jsonic-cli.ts` calls `require(name)`, retries
`require('@tabnas/' + name)` for a bare name, and rethrows the original
failure. Neither Go nor Rust can load a module by name at run time, so
both resolve a reference against a compiled-in registry. Rust prints the
sentence Go already prints, rather than inventing a third wording for the
same condition.

The consequence for a user is not the message but the set: the stock
command resolves `debug`, `jsonic` and `json`, and anything else needs a
custom binary. The help text says so, which is the next entry.

Pinned by `an_unresolvable_plugin_reports_a_registry_miss`.

## The help text's Plugins section

| | Plugins section |
|---|---|
| TypeScript | names `directive, multisource, csv, toml, ...` as references found in "the ./plugin folder of the distribution" |
| Go | "Plugins are compiled into the binary (Go cannot load modules at runtime)", then `debug, jsonic, json` |
| Rust | "Plugins are compiled into the binary (Rust cannot load modules at runtime)", then `debug, jsonic, json` |

The two plugin EXAMPLES differ for the same reason: the canonical text
says `# Using plugins (e.g. npm install @tabnas/csv)`, and the ports
describe a custom binary instead.

Everything else agrees word for word, and
`the_help_text_outside_the_plugin_sections_is_the_canonical_text` proves
it rather than asserting it: the test reads the template literal out of
`ts/src/jsonic-cli.ts` and compares the flag surface and the tail against
what `-h` prints here. It allows one thing beyond the plugin sections.
Six lines of the canonical template end in a trailing space (`where `,
two flag descriptions, the "from highest" line and two `{"a":1} `
examples). Go dropped them and so does Rust: nothing a reader sees
changes, and an editor that strips trailing whitespace would otherwise
turn a no-op edit red.

`test/spec/help.tsv` pins `CONTAINS:Usage:` and passes in all three. A
row pinning the Plugins paragraph itself cannot live in `test/spec/`,
because every runtime runs that directory and one of them would have to
be wrong. Pinned by `the_help_text_describes_compiled_in_plugins`.

## The debug trace

| input | TypeScript | Go | Rust |
|---|---|---|---|
| `jsonic -d a:1` | description, engine log lines, `{"a":1}` | description, plugin trace lines, `{"a":1}` | description, plugin trace lines, `{"a":1}` |

`-d` in TypeScript installs `@tabnas/debug` and pushes `log=-1` onto the
metadata bag, and the engine's own log is what emits the token-by-token
trace. The Rust engine has no `log` metadata: tracing comes from the
debug plugin's subscribers. The argument loop still pushes `log=-1`,
because it is the canonical loop, and the metadata is simply inert.

The two ends agree exactly. `test/spec/args.tsv` pins the first line with
`CONTAINS:=== PARSE ===` and the suites pin the last line as the JSON
result; the lines between differ in count and in text.

Not expressible as a register row: the trace volume is engine dependent,
so a cell holding it would pin the engine's version rather than this
command's behaviour. Pinned by `the_debug_trace_comes_from_the_plugin`.

## Nesting past 127 containers

| input | TypeScript | Go | Rust |
|---|---|---|---|
| 128 nested lists | the nested value | the nested value | `ERROR:cancel` |
| 127 nested lists | the nested value | the nested value | the nested value |

This is the one divergence a fixture cell can express, and it is the one
row in [`test/divergent.tsv`](test/divergent.tsv).

The bound belongs to `tabnas-jsonic`, not to this repository, and is
recorded in that crate's own documentation: the engine walks a value with
the call stack to display, convert or drop it, and a source a few
thousand levels deep ended the process. This command does the same walk
twice more, once to merge each source and once to serialize, so the bound
is what keeps a piped file from taking the process down.

Go was measured at depths 127, 128 and 129 and has no limit. The
TypeScript column is inherited from `tabnas/jsonic`, whose own record
states that neither TypeScript nor Go limits nesting; it was not run
here. The row closes when that crate's bound does.

## An option the engine types as a function reference

| input | TypeScript | Go | Rust |
|---|---|---|---|
| `jsonic -o map.merge=true a:1` | `{"a":1}` | `{"a":1}` | `Grammar: options.map.merge must be a function reference or null`, exit 1 |
| `jsonic -o parser.start=true a:1` | `{"a":1}` | `{"a":1}` | `Grammar: options.parser.start must be a function reference or null`, exit 1 |
| `jsonic -o map.extend=false a:1` | `{"a":1}` | `{"a":1}` | `{"a":1}` |

`Jsonic.make(options)` deep-merges the bag into the defaults and
validates nothing, so JavaScript stores `true` where a function belongs
and only trips over it if the grammar ever calls it. `map.merge` is read
at one place only, and only for a REPEATED key
(`jsonic/ts/src/grammar.ts`: `ctx.cfg.map.merge ? ctx.cfg.map.merge(prev,
val, r, ctx) : deep(prev, val)`), so the first row above prints
`{"a":1}` there. Go stores the bag as `map[string]any` and checks it no
harder.

The Rust engine validates the option document as it installs it, so the
same argument is refused before anything is parsed. Six option paths
reachable from `-o` are typed this way today: `map.merge`, `text.modify`,
`parse.prepare.<name>`, `parse.budget.onCheck`, `parser.start` and
`config.modify.<name>`.

This is not repairable in this crate. A `-o` value is text, and no text
names a Rust function, so the option could never do what the canonical
command lets it half-do; and swallowing the rejection would hide every
real mistake in an option document. The third row is there to show the
rejection is about function references and not about `-o`.

Not expressible as a register row: the register reads a failed run as
`ERROR:<code>`, and this failure is the command reporting a grammar
build error, which carries no engine error code. Pinned by
`a_function_typed_option_is_refused_rather_than_ignored`.

## Differences that are not divergences

Worth separating out, because each looks like one:

- **An option value that does not parse.** `-o JSON.space='` throws out
  of `run()` in TypeScript and is reported here. Go ignores it and
  carries on, which is the Go port diverging from the canonical, not
  this one. Rust matches TypeScript, apart from the exit code recorded
  above.
- **The prototype-poisoning keys.** `jsonic __proto__:1` prints `{}` in
  all three, and so does `jsonic 'a:[{__proto__:1}]'` print
  `{"a":[{}]}`: `util.deep` merges every index the overlay names, an
  index past the end of the base included, so an APPENDED element
  reaches the guarded loop too. Pinned by the three `__proto__` rows of
  `test/spec/basic.tsv`, which all three runtimes run. The engine's `util.deep` skips `__proto__`, `constructor`
  and `prototype`, and `rs/src/merge.rs` is a port of that function
  rather than a call to `tabnas_jsonic::deep_merge`, which drops the
  guard when the base is not already a container. The module comment
  says why.
- **Engine-level behaviour**: lone surrogates folding to U+FFFD, the
  regular expression dialect, key order, and how a quote ends a text run.
  All of it comes from `tabnas/parser` and `tabnas/jsonic` and is
  recorded in those repositories, not here.

## Where Go diverges and Rust does not

Four behaviours still differ between the Go port and this one. They are
listed because a reader comparing the two ports will meet them, not
because this port diverges: on all four, Rust prints what the canonical
TypeScript prints. Two are defects in `go/cli/stringify.go`; the third,
key order, is ruled out of the value contract by admin `DECISIONS.md`
ADR-15 and is a difference rather than a defect; the fourth is not this
repository's to repair at all.

| input | TypeScript | Go | Rust |
|---|---|---|---|
| `jsonic -o 'JSON.replacer=[b,a]' a:1,b:2` | `{"b":2,"a":1}` | `{"a":1,"b":2}` | `{"b":2,"a":1}` |
| `jsonic -o 'JSON.space="2"' a:1` | two-space indent | indent of the character `2` | two-space indent |
| `jsonic -o 'JSON.replacer="[a]"' a:1,b:2` | `{"a":1}` | `{}` | `{"a":1}` |
| `jsonic 2:b,1:a` | `{"1":"a","2":"b"}` | `{"2":"b","1":"a"}` | `{"1":"a","2":"b"}` |
| `jsonic -o value.def.foo.val=1 a:true,b:foo` | `{"a":true,"b":1}` | `{"a":"true","b":1}` | `{"a":true,"b":1}` |

Each row is asserted, not merely written down: `test/divergent.tsv`
carries them with a cell per runtime, and the three suites named above
run their own column, so a row that gets repaired fails and must then be
deleted.

**Key order under a replacer.** `JSON.stringify` builds a PropertyList
from the replacer array and then walks THAT, looking each key up, so the
output order is the replacer's and not the object's. Measured under
Node, which is where the TypeScript column comes from, since this
behaviour is plain JavaScript rather than tabnas code:
`JSON.stringify({a:1,b:2}, ['b','a'])` is `{"b":2,"a":1}` and
`JSON.stringify({a:1,b:2,c:{a:3,b:4}}, ['c','a'])` is
`{"c":{"a":3},"a":1}`. The same walk drops a repeated entry, drops an
entry that is neither a string nor a number, and keeps an empty list
meaning "no key survives". Go filters the object's own keys instead, so
it agrees whenever the two orders happen to coincide, which every shared
fixture row does. Pinned by `a_replacer_list_sets_the_key_order` in
`rs/tests/stringify_test.rs`.

**Array-index keys enumerate first.** A JavaScript object does not
enumerate in insertion order alone: `[[OwnPropertyKeys]]` yields the
keys that are canonical array indices first, in ascending numeric order,
and the remaining string keys in the order they were created. Measured
under Node against `ts/src/jsonic-cli.ts`: `jsonic 2:b,1:a` prints
`{"1":"a","2":"b"}`, `jsonic b:1,10:x,9:y,a:2` prints
`{"9":"y","10":"x","b":1,"a":2}`, and the bounds hold as
`Object.keys` reports them, with `4294967294` sorting and `4294967295`,
`01`, `00`, `-1`, `-0` and `1.5` staying put. Go walks its
insertion-ordered entry list and prints the source order. This is why
the case is pinned by `array_index_keys_enumerate_before_the_others` in
`rs/tests/stringify_test.rs` rather than by a `test/spec/` row: every
runtime runs that directory, and a row here would fail the Go suite.

This one is NOT a Go defect to repair. ADR-15 rules map key order out of
the parsed-value contract, and names this exact phenomenon, so Go
preserving source order is a permitted difference and emulating
ECMAScript here would be a choice rather than an obligation. Rust
emulates it because matching the canonical command costs nothing over
an `IndexMap`; Go is free either way, and the row records the
difference. The same point was raised and withdrawn on
[tabnas/jsonic-cli#29](https://github.com/tabnas/jsonic-cli/issues/29).

**A string option value is parsed twice.** `ts/src/jsonic-cli.ts` ends
with `replacer = Jsonic(options.JSON.replacer)` and
`space = Jsonic(options.JSON.space)`, and the bag already holds a value
`handle_props` parsed. `Jsonic(x)` returns `x` unchanged for anything
that is not a string (`jsonic.ts`: `if (S.string === typeof src) { ... }
return src`), so the second pass matters only when the value IS a
string: `Jsonic("2")` is the number two and `Jsonic("[a]")` is the array
`["a"]`. Go keeps the literal string. `Jsonic("--")` is the string
`--`, which is why `test/spec/stringify.tsv` passes in all three either
way. Pinned by `a_string_json_option_is_parsed_again`.

**A value def replaces the built-in ones.** `-o value.def.foo.val=1`
adds a value def in all three runtimes. In Go it also DROPS the defs the
grammar ships, so `true` and `null` then lex as text: measured on
2026-09-22, `jsonic -o value.def.foo.val=1 a:true,b:foo` prints
`{"a":true,"b":1}` under `ts/src/jsonic-cli.ts` and the Rust binary, and
`{"a":"true","b":1}` under `go/cmd/jsonic`.

The repair is not in this repository. The option bag the Go command
builds is the right one, and a direct `jsonic.Make(tabnas.MapToOptions(
bag))` answers the same way, so the difference is below the command:
`parser/go`'s `options.go` rebuilds the configuration's value defs from
the supplied ones alone (`cfg.ValueDef = make(map[string]any)`, and
`cfg.ValueDefRe` truncated) where the canonical `util.deep` merges into
what is already there. Working around it here would mean this command
carrying its own copy of the grammar's default defs, which is grammar
knowledge that belongs in the engine and would drift the day the engine
changed. Recorded as a row rather than repaired, and the row closes when
`tabnas/parser` does.

## A source with no value in it

| input | TypeScript | Go | Rust |
|---|---|---|---|
| `jsonic a:1 '   '` | `{"a":1}` | `null` | `null` |
| `jsonic a:1 '#c'` | `{"a":1}` | `null` | `null` |
| `jsonic a:1 ''` | `{"a":1}` | `{"a":1}` | `{"a":1}` |

A source holding only whitespace, or only a comment, parses to
`undefined` in the canonical command, so `util.deep(data, {val:
undefined})` merges nothing and what came before survives. Both engines
answer `null` for the same source, and the merge then replaces the value
built up so far. The third row is the case that already agrees: a source
of ZERO LENGTH is the engines' empty result, which the Go command forces
to the Undefined sentinel and the Rust engine answers with, so an empty
`--file` or an empty pipe is a no-op in all three.

So the difference is not "empty source" but WHICH sources count as
empty: the canonical takes any source that yields no value, and the
engines take a source with no bytes. It bites hardest where the merge
matters, `jsonic -f config.jsonic` with a terminal or a whitespace-only
pipe answering standard input, where the file's value is replaced by
`null` rather than kept.

Not repairable in this repository, and the same in both ports. Deciding
that a source holds no value takes lexing it, and this command has no
grammar of its own; the fix belongs where the empty result is decided,
in `tabnas/jsonic`. Recorded as two register rows, asserted by all three
suites, and they close when the engines do.

## Repaired, and no longer divergent

Two behaviours listed here until 2026-09-22 were Go defects, reported as
[tabnas/jsonic-cli#29](https://github.com/tabnas/jsonic-cli/issues/29)
and repaired in `go/cli/stringify.go`. They are recorded as repairs
rather than deleted, because the classes they belong to keep recurring
across this fleet.

**Number to string.** `JSON.stringify` writes a number with JavaScript's
`Number::toString` (ECMA-262 6.1.6.1.20), which uses fixed notation from
`1e-6` up to just under `1e21`, an exponent outside that range with no
leading zero in it, and breaks a shortest-form tie to the even digit. Go
used `strconv.FormatFloat(f, 'g', -1, 64)`, which switches to an
exponent far earlier and rounds a tie away from zero: measured under
Node against `ts/src/jsonic-cli.ts`, `jsonic a:1658206780088562.2`
prints `{"a":1658206780088562.2}` and Go printed
`{"a":1.6582067800885622e+15}`; `jsonic a:0.000001` prints
`{"a":0.000001}` and Go printed `{"a":1e-06}`. `jsNumberToString` in
`go/cli/stringify.go` now carries the specification's algorithm, the one
`csv/go`, `csv/rs`, `xml/rs` and `rs/src/stringify.rs` already carry:
take the digit count from the shortest form, then take the digits from a
fixed-precision render at that width. Fuzzed against Node over 221,898
doubles with no mismatch. Pinned by the five number rows of
`test/spec/basic.tsv`, which all three runtimes run, and by
`TestFormatNumberIsJavaScriptNumberToString` in
`go/cli/stringify_test.go`; Rust keeps `numbers_match_json_stringify`
and `numbers_at_a_shortest_form_tie_round_to_even`.

**A string `JSON.space` cut by bytes.** TypeScript cuts with
`substring(0, 10)`, which counts UTF-16 code units. Go cut with a byte
slice (`v[:10]`), which splits a multi-byte character, so the command's
standard output was not valid UTF-8 at all, once per indent level:
`jsonic -o 'JSON.space=abcdefghi<astral>' 'a:{b:1}'` emitted the nine
ASCII characters plus the single byte `f0`. Both ports now count UTF-16
code units. Where the cut lands INSIDE an astral character, TypeScript
holds the lone high surrogate and Node writes it to a UTF-8 stream as
U+FFFD, so both ports write U+FFFD there. Pinned by the astral `space`
row of `test/spec/stringify.tsv`, by
`TestSpaceCutThroughAnAstralCharacter` in `go/cli/stringify_test.go`,
and in Rust by `a_string_space_is_cut_at_ten_utf16_units` and
`a_space_cut_through_an_astral_character_keeps_its_half`.

## What was compared, and how widely

Beyond the shared fixtures, the Rust command was diffed against the Go
one over generated argument vectors: the flag surface crossed with a
corpus of sources, then ordered pairs of sources, each with and without
piped input. Standard output and the success or failure of the run agree
on every one of them EXCEPT the families recorded above. The verifying
session ran two sweeps of its own over the built binaries:

- 624 vectors over the `JSON` option surface: 38 disagreements, 4 of
  them the replacer key order and 34 the twice-parsed string option.
- 1,742 vectors over the flag surface, the engine options and ordered
  pairs of sources: 136 disagreements, 86 of them the number formatting
  below 1e-6 and 50 the function-typed option.

Rust matches TypeScript on the first three of those families. The
fourth, the function-typed option, is the one recorded against Rust.

Those sweeps measured Rust against GO, because the canonical command
could not be run in that session. The session of 2026-09-22 ran it, and
diffed the Rust binary against `run(argv, console)` directly over 781
argument vectors: the `JSON` option surface, the engine options, the
metadata flags, `--`, `-` and the value-less trailing flags, each
crossed with a corpus of twenty-three sources, plus ordered pairs of
sources. Two runs agree when the first printed entry and the success or
failure of the run agree.

Twenty-four vectors disagreed, and every one of them is a family already
recorded here or an artefact of how the canonical command was read:

- 20 are the astral `JSON.space` cut. The comparison read the
  TypeScript string IN PROCESS, where it holds the lone high surrogate,
  while the Rust column came from the binary's standard output. On the
  STREAM the two agree: `node ts/bin/jsonic -o 'JSON.space=abcdefghi<astral>' a:1`
  writes `ef bf bd` where the half was, which is the U+FFFD Rust writes.
- 2 are the help text's Plugins section, recorded above.
- 2 are `-p json` and `-p @tabnas/json`, which the canonical command
  could not resolve in this checkout: `@tabnas/json` is neither a
  dependency nor a peer dependency of `ts/package.json`, so
  `require` reported `Cannot find module 'json'` while the Rust binary
  carries the plugin in its registry. That is the compiled-in registry
  already recorded under "Plugin loading", seen from the other side.

No other vector disagreed, so no unrecorded divergence was found.
