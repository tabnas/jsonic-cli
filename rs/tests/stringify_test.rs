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
