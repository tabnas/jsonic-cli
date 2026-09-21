/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! The output contract: what the command prints for a value is what
//! `JSON.stringify` prints for it, key order, number formatting, escaping
//! and the `replacer` and `space` arguments included.
//!
//! Every expectation in `numbers_match_json_stringify` was MEASURED by
//! running `JSON.stringify(<value>)` under Node, not derived from the
//! Rust code, because a formatter checked against itself proves nothing.
//! Rust's own `{}` never writes exponential notation and JavaScript's
//! `Number::toString` does past 1e21 and below 1e-6, so the two disagree
//! on exactly the values these rows hold.

mod common;

use common::run;

/// Print one jsonic source and read back the single printed entry.
fn out(source: &str) -> String {
    let result = run(&[source], "");
    assert_eq!(result.code, 0, "{source}: {}", result.stderr);
    result.first().to_string()
}

#[test]
fn numbers_match_json_stringify() {
    // source , what `JSON.stringify` prints for the same number.
    for (source, want) in [
        ("0", "0"),
        ("-0", "0"),
        ("1", "1"),
        ("-1", "-1"),
        ("1e3", "1000"),
        ("1e20", "100000000000000000000"),
        ("1e21", "1e+21"),
        ("1e22", "1e+22"),
        ("1.5e22", "1.5e+22"),
        ("0.1", "0.1"),
        ("0.5", "0.5"),
        ("-2.25", "-2.25"),
        ("1e-6", "0.000001"),
        ("1e-7", "1e-7"),
        ("1.2345e-8", "1.2345e-8"),
        ("123456789012345678901234", "1.2345678901234569e+23"),
        ("18446744073709551615", "18446744073709552000"),
        ("0xFFFFFFFFFFFFFFFF", "18446744073709552000"),
        ("9007199254740993", "9007199254740992"),
        ("1e100", "1e+100"),
        ("5e-324", "5e-324"),
        ("1.7976931348623157e308", "1.7976931348623157e+308"),
        ("1234.5678", "1234.5678"),
        ("100.5", "100.5"),
        ("0.000001234", "0.000001234"),
        ("3.0000000000000004", "3.0000000000000004"),
        ("1e-21", "1e-21"),
        ("6.02e23", "6.02e+23"),
    ] {
        assert_eq!(out(&format!("a:{source}")), format!("{{\"a\":{want}}}"));
    }
}

/// Keys come out in SOURCE order, which is what `JSON.stringify` does
/// over a JavaScript object, and not in sorted order.
#[test]
fn keys_keep_their_source_order() {
    assert_eq!(
        out("z:1,a:2,m:{y:1,b:2}"),
        r#"{"z":1,"a":2,"m":{"y":1,"b":2}}"#
    );
}

/// The escaping `JSON.stringify` performs, measured against Node:
/// `JSON.stringify({a:"q\"\\\n\r\t\b\f\u0001\u007f é"})`.
#[test]
fn strings_escape_like_json_stringify() {
    let source = r#"a:'q"\\\n\r\t\b\f\u0001\u007f é'"#;
    let out = run(&[source], "");
    assert_eq!(out.code, 0, "{}", out.stderr);
    // DEL (U+007F) is NOT escaped by `JSON.stringify`; only C0 controls
    // below U+0020 are.
    let want = format!(
        "{{\"a\":\"q\\\"\\\\\\n\\r\\t\\b\\f\\u0001{} é\"}}",
        '\u{7f}'
    );
    assert_eq!(out.first(), want);
}

/// `JSON.space` clamps and indents the way `JSON.stringify` does, and a
/// string space is used at every depth.
#[test]
fn space_matches_json_stringify() {
    assert_eq!(
        run(&["-o", "JSON.space=2", "a:1"], "").first(),
        "{\n  \"a\": 1\n}"
    );
    assert_eq!(
        run(&["-o", "JSON.space=20", "a:1"], "").first(),
        "{\n          \"a\": 1\n}"
    );
    assert_eq!(
        run(&["-o", "JSON.space=-1", "a:1"], "").first(),
        r#"{"a":1}"#
    );
    assert_eq!(
        run(&["-o", "JSON.space=0", "a:1"], "").first(),
        r#"{"a":1}"#
    );
    assert_eq!(
        run(&["-o", r#"JSON.space="--""#, "a:b:1"], "").first(),
        "{\n--\"a\": {\n----\"b\": 1\n--}\n}"
    );
    // A space with an array and an empty container, measured from
    // `JSON.stringify({a:[1,{}],b:[]}, null, 2)`.
    assert_eq!(
        run(&["-o", "JSON.space=2", "a:[1,{}],b:[]"], "").first(),
        "{\n  \"a\": [\n    1,\n    {}\n  ],\n  \"b\": []\n}"
    );
}

/// The replacer whitelist filters object keys at every depth, leaves
/// array elements alone, and coerces a numeric entry to its string form.
#[test]
fn replacer_matches_json_stringify() {
    assert_eq!(
        run(&["-o", "JSON.replacer=[a,c]", "a:1,b:2,c:{a:3,b:4}"], "").first(),
        r#"{"a":1,"c":{"a":3}}"#
    );
    assert_eq!(
        run(&["-o", "JSON.replacer=1", "a:1,1:2"], "").first(),
        r#"{"1":2}"#
    );
    assert_eq!(
        run(&["-o", "JSON.replacer=[b]", "a:[{a:1,b:2}]"], "").first(),
        r#"{}"#
    );
    assert_eq!(
        run(&["-o", "JSON.replacer=[a,b]", "a:[{a:1,c:2}]"], "").first(),
        r#"{"a":[{"a":1}]}"#
    );
}

/// A replacer list decides the OUTPUT ORDER, not just which keys survive.
/// `JSON.stringify` walks its PropertyList and looks each key up, so the
/// object's own order is not what comes out.
///
/// Measured under Node:
///
/// ```text
/// JSON.stringify({a:1,b:2}, ['b','a'])            -> {"b":2,"a":1}
/// JSON.stringify({a:1,b:2,c:{a:3,b:4}}, ['c','a']) -> {"c":{"a":3},"a":1}
/// JSON.stringify({a:1,b:2}, ['a','a','b'])        -> {"a":1,"b":2}
/// JSON.stringify({a:1}, ['z','a'])                -> {"a":1}
/// JSON.stringify({a:1}, [true,'a'])               -> {"a":1}
/// JSON.stringify({a:1}, [])                       -> {}
/// ```
#[test]
fn a_replacer_list_sets_the_key_order() {
    assert_eq!(
        run(&["-o", "JSON.replacer=[b,a]", "a:1,b:2"], "").first(),
        r#"{"b":2,"a":1}"#
    );
    assert_eq!(
        run(&["-o", "JSON.replacer=[c,a]", "a:1,b:2,c:{a:3,b:4}"], "").first(),
        r#"{"c":{"a":3},"a":1}"#
    );
    // A repeated entry appears once, at its first position.
    assert_eq!(
        run(&["-o", "JSON.replacer=[a,a,b]", "a:1,b:2"], "").first(),
        r#"{"a":1,"b":2}"#
    );
    // An entry naming no key contributes nothing.
    assert_eq!(
        run(&["-o", "JSON.replacer=[z,a]", "a:1"], "").first(),
        r#"{"a":1}"#
    );
    // Only strings and numbers become keys; anything else is dropped
    // rather than becoming an empty key.
    assert_eq!(
        run(&["-o", "JSON.replacer=[true,a]", "a:1"], "").first(),
        r#"{"a":1}"#
    );
    // An empty list keeps nothing, which is not the same as no list.
    assert_eq!(run(&["-o", "JSON.replacer=[]", "a:1"], "").first(), "{}");
}

/// A `JSON` option whose value is a STRING is parsed a second time, as
/// `replacer = Jsonic(options.JSON.replacer)` and
/// `space = Jsonic(options.JSON.space)` do in `ts/src/jsonic-cli.ts`.
/// `Jsonic(x)` returns `x` unchanged for a non-string, so only the
/// string case is affected.
#[test]
fn a_string_json_option_is_parsed_again() {
    // `Jsonic("2")` is the number two, so this indents by two spaces
    // rather than by the character `2`.
    assert_eq!(
        run(&["-o", r#"JSON.space="2""#, "a:1"], "").first(),
        "{\n  \"a\": 1\n}"
    );
    // `Jsonic("[a]")` is the array `["a"]`, so this is a whitelist and
    // not a single key spelt `[a]`.
    assert_eq!(
        run(&["-o", r#"JSON.replacer="[a]""#, "a:1,b:2"], "").first(),
        r#"{"a":1}"#
    );
    // `Jsonic("--")` is the string `--`, so the fixture case is
    // unchanged by the second pass.
    assert_eq!(
        run(&["-o", r#"JSON.space="--""#, "a:1"], "").first(),
        "{\n--\"a\": 1\n}"
    );
    // `Jsonic("  ")` is nothing at all, and no indent is what
    // `JSON.stringify(v, null, undefined)` produces.
    assert_eq!(
        run(&["-o", r#"JSON.space="  ""#, "a:1"], "").first(),
        r#"{"a":1}"#
    );
    // A string that does not parse is an error, as it is in TypeScript,
    // where `Jsonic(...)` throws out of `run()`.
    let failed = run(&["-o", r#"JSON.space="'""#, "a:1"], "");
    assert_eq!(failed.code, 1);
    assert!(failed.stderr.contains("unterminated"), "{}", failed.stderr);
}

/// A string `space` is truncated to ten UTF-16 CODE UNITS, which is what
/// `substring(0, 10)` counts. Measured under Node:
/// `JSON.stringify({a:1}, null, '\u{1F600}'.repeat(6))` keeps five of
/// the six astral characters.
#[test]
fn a_string_space_is_cut_at_ten_utf16_units() {
    assert_eq!(
        run(&["-o", r#"JSON.space="abcdefghijklmno""#, "a:1"], "").first(),
        "{\nabcdefghij\"a\": 1\n}"
    );
    let six = "\u{1F600}".repeat(6);
    assert_eq!(
        run(&["-o", &format!("JSON.space=\"{six}\""), "a:1"], "").first(),
        format!("{{\n{}\"a\": 1\n}}", "\u{1F600}".repeat(5))
    );
}

/// A JavaScript object does not enumerate in insertion order alone: its
/// ARRAY-INDEX keys come first, in ascending numeric order, and the rest
/// follow in the order they were created. `JSON.stringify` walks that
/// order, so the printed key order does too.
///
/// Measured under Node against `ts/src/jsonic-cli.ts`, one command per
/// row below. The boundary cases were measured with `Object.keys` as
/// well:
///
/// ```text
/// Object.keys({'2':2,'1':1,'0':0,'01':0,'-1':0,'1.5':0,' 1':0,
///              '4294967294':0,'4294967295':0,'-0':0,'00':0,'a':0})
/// -> ['0','1','2','4294967294','01','-1','1.5',' 1','4294967295',
///     '-0','00','a']
/// ```
///
/// So `4294967294` is the largest array index, `4294967295` is not one
/// (it is reserved as an array's length), and a leading zero or a sign
/// makes the string non-canonical and therefore an ordinary key.
///
/// This case is NOT in `test/spec/`: the Go port keeps the source order
/// here (`go/cli/stringify.go` walks the insertion-ordered entry list),
/// so a shared row would break the Go suite. It is recorded against Go in
/// `DIVERGENCE.md`.
#[test]
fn array_index_keys_enumerate_before_the_others() {
    for (source, want) in [
        ("2:b,1:a", r#"{"1":"a","2":"b"}"#),
        // Ascending NUMERIC order, not lexicographic: 9 before 10.
        ("b:1,10:x,9:y,a:2", r#"{"9":"y","10":"x","b":1,"a":2}"#),
        // The upper bound, measured: 4294967294 sorts, 4294967295 does not.
        (
            "4294967295:a,4294967294:b",
            r#"{"4294967294":"b","4294967295":"a"}"#,
        ),
        // A leading zero is not a canonical numeric string.
        ("01:a,1:b", r#"{"1":"b","01":"a"}"#),
        ("00:a,0:b", r#"{"0":"b","00":"a"}"#),
        // Neither is a sign, on either zero.
        ("-1:a,0:b", r#"{"0":"b","-1":"a"}"#),
        ("-0:a,0:b", r#"{"0":"b","-0":"a"}"#),
        // Nor a fraction, nor surrounding space.
        ("1.5:a,1:b", r#"{"1":"b","1.5":"a"}"#),
        // Every depth, not just the top level.
        (
            "2:b,1:a,x:{5:q,0:p}",
            r#"{"1":"a","2":"b","x":{"0":"p","5":"q"}}"#,
        ),
        // Non-index keys keep the order the source wrote them in.
        ("z:1,a:2,m:{y:1,b:2}", r#"{"z":1,"a":2,"m":{"y":1,"b":2}}"#),
    ] {
        assert_eq!(out(source), want, "for {source}");
    }
}

/// A `space` cut THROUGH an astral character keeps the half JavaScript
/// keeps, spelt U+FFFD.
///
/// `substring(0, 10)` counts UTF-16 code units, so a string whose tenth
/// unit is the first half of a surrogate pair leaves JavaScript holding
/// that half on its own. A Rust string cannot hold half a pair, and
/// dropping it is not the same answer: Node writes an unpaired surrogate
/// to a UTF-8 stream as U+FFFD. Measured under Node against
/// `ts/src/jsonic-cli.ts`, whose indent for the command below is the nine
/// ASCII characters followed by the bytes `ef bf bd`.
///
/// This case is NOT in `test/spec/`: the Go port cuts the string with a
/// BYTE slice, so it emits the first byte of the astral character and
/// nothing else, and a shared row would break the Go suite.
#[test]
fn a_space_cut_through_an_astral_character_keeps_its_half() {
    assert_eq!(
        run(&["-o", "JSON.space=\"abcdefghi\u{1F600}\"", "a:1"], "").first(),
        "{\nabcdefghi\u{FFFD}\"a\": 1\n}"
    );
    // One unit earlier the character fits whole, and nothing is added.
    assert_eq!(
        run(&["-o", "JSON.space=\"abcdefgh\u{1F600}\"", "a:1"], "").first(),
        "{\nabcdefgh\u{1F600}\"a\": 1\n}"
    );
    // A cut that lands on a character boundary adds no replacement.
    assert_eq!(
        run(&["-o", "JSON.space=\"abcdefghij\u{1F600}\"", "a:1"], "").first(),
        "{\nabcdefghij\"a\": 1\n}"
    );
}

/// Where two shortest digit strings are equally close to the value,
/// ECMAScript takes the one ending in an EVEN digit.
///
/// Rust's shortest formatter rounds such a midpoint away from zero, so
/// deriving the answer from `format!("{number:e}")` alone printed a
/// different last digit. Each row was measured under Node against
/// `ts/src/jsonic-cli.ts` by running `jsonic a:<source>`, and the whole
/// formatter was fuzzed against Node over 81,884 doubles with no
/// mismatch.
///
/// These are NOT in `test/spec/`: the Go port prints these values in
/// exponential notation (`1.6582067800885622e+15`), which is the number
/// formatting already recorded against Go in `DIVERGENCE.md`, so a shared
/// row would break the Go suite.
#[test]
fn numbers_at_a_shortest_form_tie_round_to_even() {
    for (source, want) in [
        ("1658206780088562.2", "1658206780088562.2"),
        ("165793407361858.12", "165793407361858.12"),
        ("-1149636667324797.2", "-1149636667324797.2"),
        ("34461792646535.312", "34461792646535.312"),
        ("213751243550068.62", "213751243550068.62"),
        ("-264310078315752.62", "-264310078315752.62"),
    ] {
        assert_eq!(
            out(&format!("a:{source}")),
            format!("{{\"a\":{want}}}"),
            "for {source}"
        );
    }
}
