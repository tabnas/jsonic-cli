/* Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
contributors, MIT License */

//! A port of JavaScript's `JSON.stringify(value, replacer, space)` over
//! the engine's [`tabnas::Value`]. The port of `go/cli/stringify.go`.
//!
//! The CLI's whole output contract is this function: whatever the
//! canonical command prints for a value, this prints for the same value,
//! key order and number formatting included.

use std::collections::{BTreeSet, HashMap};

use serde_json::{Map, Value as Json};
use tabnas::Value;

/// One of the two `JSON` options, read the way the canonical command
/// reads it.
///
/// `ts/src/jsonic-cli.ts` runs each through vanilla jsonic a SECOND time:
///
/// ```text
/// replacer = Jsonic(options.JSON.replacer)
/// space = Jsonic(options.JSON.space)
/// ```
///
/// `Jsonic(x)` returns `x` unchanged for anything that is not a string
/// (`jsonic.ts`: `if (S.string === typeof src) { ... } return src`), so
/// the second pass only bites when the option value IS a string, which is
/// how `-o 'JSON.replacer="[a,c]"'` becomes an array and
/// `-o 'JSON.space="2"'` becomes the number two. Skipping the second pass
/// leaves the literal string, which is a different answer.
///
/// A string that does not parse is an error, as it is in TypeScript,
/// where `Jsonic(...)` throws out of `run()`.
pub(crate) fn json_option(
    json_bag: Option<&Map<String, Json>>,
    key: &str,
) -> Result<Option<Json>, String> {
    let Some(raw) = json_bag.and_then(|bag| bag.get(key)) else {
        return Ok(None);
    };
    let Json::String(text) = raw else {
        return Ok(Some(raw.clone()));
    };
    let parsed = tabnas_jsonic::parse(text).map_err(|error| error.to_string())?;
    Ok(Some(crate::args::to_json(&parsed)))
}

/// The `JSON.replacer` option as an ORDERED key list, already resolved by
/// [`json_option`].
///
/// The TypeScript CLI finishes it in three steps:
///
/// ```text
/// replacer = Array.isArray(replacer) ? replacer
///          : null == replacer ? null
///          : [replacer]
/// ```
///
/// and `JSON.stringify` then turns that into its PropertyList: each entry
/// that is a string or a number becomes a key, in order, without
/// repeats; every other entry is DROPPED rather than becoming an empty
/// key. `None` means no filtering at all, and an empty list means every
/// key is filtered out.
pub(crate) fn parse_replacer(value: Option<&Json>) -> Option<Vec<String>> {
    let entries: Vec<String> = match value? {
        Json::Null => return None,
        Json::Array(items) => items.iter().filter_map(key_string).collect(),
        single => key_string(single).into_iter().collect(),
    };
    let mut seen = BTreeSet::new();
    Some(
        entries
            .into_iter()
            .filter(|key| seen.insert(key.clone()))
            .collect(),
    )
}

/// The `JSON.space` option as an indent string, exactly as
/// `JSON.stringify` reads its third argument: a number is clamped to
/// `0..=10` and becomes that many spaces, a string is used as it stands
/// and truncated to ten UTF-16 code units, and anything else means no
/// indent.
pub(crate) fn parse_space(value: Option<&Json>) -> String {
    let Some(raw) = value else {
        return String::new();
    };
    match raw {
        Json::String(text) => truncate_utf16(text, 10),
        Json::Number(number) => match number.as_f64() {
            Some(count) => " ".repeat(count.floor().clamp(0.0, 10.0) as usize),
            None => String::new(),
        },
        _ => String::new(),
    }
}

/// The leading `units` UTF-16 code units of `text`.
///
/// `JSON.stringify` slices a string `space` with `substring(0, 10)`,
/// which counts UTF-16 code units; a Rust `str` counts characters, and an
/// astral character is two units, so `chars().take(10)` would keep too
/// much.
///
/// A character STRADDLING the boundary leaves JavaScript holding the high
/// surrogate on its own. A Rust string cannot hold half a surrogate pair,
/// and dropping it is not equivalent: Node writes an unpaired surrogate to
/// a UTF-8 stream as U+FFFD, so the canonical command's indent has a
/// replacement character where the gap is. Measured under Node with
/// `-o 'JSON.space="abcdefghi<astral>"'`, whose indent is the nine ASCII
/// characters plus the three bytes `ef bf bd`. U+FFFD is therefore what
/// the truncated half becomes here, rather than nothing.
fn truncate_utf16(text: &str, units: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let width = ch.len_utf16();
        if units < used + width {
            if used < units {
                out.push(char::REPLACEMENT_CHARACTER);
            }
            break;
        }
        out.push(ch);
        used += width;
    }
    out
}

/// The numeric value of `key` when it is a canonical ARRAY INDEX, which
/// is what a JavaScript object enumerates ahead of its other keys.
///
/// The specification wants an integer index (a canonical numeric string
/// whose value is a non-negative integer) below `2^32 - 1`, that last
/// value being reserved as an array's length. Three consequences are easy
/// to get wrong, and all three were measured under Node with
/// `Object.keys`:
///
/// - `"4294967294"` is an index and `"4294967295"` is not, so the upper
///   bound excludes `u32::MAX`.
/// - `"01"` is not an index, because `ToString(1)` is `"1"`: a leading
///   zero makes the string non-canonical. Nor is `"00"` or `"0.0"`.
/// - `"-1"` and `"-0"` are not indices either, so no sign is accepted.
fn array_index(key: &str) -> Option<u32> {
    let bytes = key.as_bytes();
    // The largest index, 4294967294, has ten digits, so an eleventh digit
    // puts the key out of range whatever it says.
    if bytes.is_empty() || 10 < bytes.len() {
        return None;
    }
    if !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    if 1 < bytes.len() && bytes[0] == b'0' {
        return None;
    }
    let value: u64 = key.parse().ok()?;
    (value < u32::MAX as u64).then_some(value as u32)
}

/// A replacer entry as the string key it filters by, or `None` when
/// `JSON.stringify` would ignore it. Only strings and numbers become
/// keys.
fn key_string(value: &Json) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Number(number) => number.as_f64().map(format_number),
        _ => None,
    }
}

/// Serialize `value` the way `JSON.stringify(value, replacer, space)`
/// does.
///
/// Object keys come out in the order the source wrote them, because the
/// engine's object is an `IndexMap` and a JavaScript object enumerates the
/// same way. `undefined` has no JSON form: `JSON.stringify` answers with
/// the JavaScript value `undefined`, and `console.log` of that prints the
/// word, which is what the canonical command shows.
pub(crate) fn stringify(value: &Value, replacer: Option<&[String]>, space: &str) -> String {
    if value.is_undefined() {
        return "undefined".to_string();
    }
    let mut out = String::new();
    write_value(&mut out, value, replacer, space, "");
    out
}

fn write_value(
    out: &mut String,
    value: &Value,
    replacer: Option<&[String]>,
    space: &str,
    indent: &str,
) {
    match value {
        Value::Undefined | Value::Null => out.push_str("null"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => out.push_str(&format_number(*number)),
        Value::String(text) => write_string(out, text),
        Value::Text(text) => write_string(out, &text.string),
        Value::Array(items) => write_array(out, items, replacer, space, indent),
        Value::ListRef(list) => write_array(out, &list.value, replacer, space, indent),
        Value::Object(entries) => write_object(out, entries.iter(), replacer, space, indent),
        Value::MapRef(map) => write_object(out, map.value.iter(), replacer, space, indent),
    }
}

fn write_object<'a>(
    out: &mut String,
    entries: impl Iterator<Item = (&'a String, &'a Value)>,
    replacer: Option<&[String]>,
    space: &str,
    indent: &str,
) {
    let mut entries: Vec<(&String, &Value)> = entries.collect();

    // A JavaScript object does NOT enumerate in insertion order alone.
    // `[[OwnPropertyKeys]]` yields the ARRAY-INDEX keys first, in
    // ascending numeric order, and only then the remaining string keys in
    // the order they were created. So the source `2:b,1:a` prints
    // `{"1":"a","2":"b"}` under Node while the engine's `IndexMap` holds
    // `2` before `1`. Restoring that order here is what keeps the printed
    // key order the canonical command's; see [`array_index`] for which
    // keys count.
    entries.sort_by_key(|(key, _)| match array_index(key) {
        Some(index) => (0u8, index),
        None => (1u8, 0),
    });

    // With a replacer list, `JSON.stringify` walks the PropertyList and
    // looks each key up, so the OUTPUT order is the replacer's order and
    // not the object's: `JSON.stringify({a:1,b:2}, ['b','a'])` is
    // `{"b":2,"a":1}`, measured under Node. Without one the object's own
    // order stands. Either way a key whose value is `undefined` is
    // omitted, as it is in JavaScript.
    //
    // The lookup is KEYED, and built once per object. Scanning the entry
    // list for each replacer name instead makes serializing an object
    // with `n` keys through a replacer naming `n` of them cost `n^2`
    // comparisons, and both the object and `JSON.replacer` come from the
    // command line, so that is untrusted input deciding how much work the
    // command does.
    let kept: Vec<(&String, &Value)> = match replacer {
        Some(keys) => {
            let index: HashMap<&str, &Value> = entries
                .iter()
                .map(|(key, value)| (key.as_str(), *value))
                .collect();
            keys.iter()
                .filter_map(|key| {
                    index
                        .get(key.as_str())
                        .copied()
                        .filter(|value| !value.is_undefined())
                        .map(|value| (key, value))
                })
                .collect()
        }
        None => entries
            .into_iter()
            .filter(|(_, value)| !value.is_undefined())
            .collect(),
    };

    if kept.is_empty() {
        out.push_str("{}");
        return;
    }

    let inner = format!("{indent}{space}");
    out.push('{');
    for (at, (key, value)) in kept.iter().enumerate() {
        if 0 < at {
            out.push(',');
        }
        if !space.is_empty() {
            out.push('\n');
            out.push_str(&inner);
        }
        write_string(out, key);
        out.push(':');
        if !space.is_empty() {
            out.push(' ');
        }
        write_value(out, value, replacer, space, &inner);
    }
    if !space.is_empty() {
        out.push('\n');
        out.push_str(indent);
    }
    out.push('}');
}

fn write_array(
    out: &mut String,
    items: &[Value],
    replacer: Option<&[String]>,
    space: &str,
    indent: &str,
) {
    if items.is_empty() {
        out.push_str("[]");
        return;
    }
    let inner = format!("{indent}{space}");
    out.push('[');
    for (at, item) in items.iter().enumerate() {
        if 0 < at {
            out.push(',');
        }
        if !space.is_empty() {
            out.push('\n');
            out.push_str(&inner);
        }
        // A replacer array filters OBJECT KEYS, never array elements, but
        // it still reaches the objects nested inside one. An `undefined`
        // element becomes `null` rather than vanishing.
        write_value(out, item, replacer, space, &inner);
    }
    if !space.is_empty() {
        out.push('\n');
        out.push_str(indent);
    }
    out.push(']');
}

/// A number the way `JSON.stringify` writes one: a non-finite number is
/// `null`, and everything else is JavaScript's `Number::toString`.
pub(crate) fn format_number(number: f64) -> String {
    if !number.is_finite() {
        return "null".to_string();
    }
    if number == 0.0 {
        // Both zeroes print as `0`; JavaScript has no `-0` literal in JSON.
        return "0".to_string();
    }
    let sign = if number < 0.0 { "-" } else { "" };
    format!("{sign}{}", positive_to_string(number.abs()))
}

/// ECMAScript `Number::toString` for a finite, positive number.
///
/// The specification wants the SHORTEST digit string `s` that reads back
/// as `number` (length `k`) and the exponent `n` placing the decimal point,
/// then chooses between fixed and exponential notation from those two.
/// Rust's own `{}` never chooses exponential, which is why this cannot
/// just be `format!("{number}")`.
///
/// Rust's `{:e}` gives digits of exactly that shortest length, but not
/// always the same digits: where two `k`-digit strings are equally close
/// to `number`, ECMAScript takes the one ending in an EVEN digit and
/// Rust's shortest formatter rounds away from zero. The double from
/// `a:1658206780088562.2` is such a midpoint, and deriving the answer from
/// `{:e}` alone printed `1658206780088562.3` where Node prints
/// `1658206780088562.2`.
///
/// So `k` comes from `{:e}` and the digits come from asking for exactly
/// that many with `{:.*e}`, whose fixed-precision form is exactly rounded.
/// The same repair is in `csv/rs`'s `js_number_to_string` and in
/// `xml/rs`; this is a port of it. Fuzzed against Node over 81,884
/// doubles (random bit patterns, decimal-scaled values, midpoint-prone
/// halves, powers of ten, subnormals and the extremes) with no mismatch;
/// the `{:e}`-only form missed 670 of them.
fn positive_to_string(number: f64) -> String {
    let shortest = format!("{number:e}");
    let shortest_k = shortest
        .split_once('e')
        .map(|(mantissa, _)| mantissa.chars().filter(char::is_ascii_digit).count())
        .expect("Rust's {:e} always writes an exponent");

    let exponential = format!("{:.*e}", shortest_k - 1, number);
    let (mantissa, exponent) = exponential
        .split_once('e')
        .expect("Rust's {:e} always writes an exponent");
    // Rounding to `k` digits can leave trailing zeros, and a carry can
    // leave one digit too many; dropping them keeps `s` shortest, which
    // is what `k` means.
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let k = digits.len() as i32;
    let n = exponent
        .parse::<i32>()
        .expect("Rust's {:e} always writes a decimal exponent")
        + 1;

    if k <= n && n <= 21 {
        return format!("{digits}{}", "0".repeat((n - k) as usize));
    }
    if 0 < n && n <= 21 {
        let at = n as usize;
        return format!("{}.{}", &digits[..at], &digits[at..]);
    }
    if -6 < n && n <= 0 {
        return format!("0.{}{digits}", "0".repeat((-n) as usize));
    }
    let power = n - 1;
    let sign = if power < 0 { "-" } else { "+" };
    if k == 1 {
        format!("{digits}e{sign}{}", power.abs())
    } else {
        format!("{}.{}e{sign}{}", &digits[..1], &digits[1..], power.abs())
    }
}

/// A double-quoted, JSON-escaped string, escaping exactly what
/// `JSON.stringify` escapes. A Rust `str` is always well formed UTF-8, so
/// the lone-surrogate case JavaScript escapes cannot arise: the engine
/// folds an unpaired surrogate to U+FFFD while lexing.
fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            ch if ch < ' ' => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::*;

    #[test]
    fn numbers_print_as_javascript_prints_them() {
        for (number, text) in [
            (1.0, "1"),
            (-0.0, "0"),
            (0.1, "0.1"),
            (1e3, "1000"),
            (1.5, "1.5"),
            (-2.25, "-2.25"),
            (18446744073709551615.0, "18446744073709552000"),
            (1e21, "1e+21"),
            (1.5e22, "1.5e+22"),
            (1e-7, "1e-7"),
            (0.000001, "0.000001"),
            (1.2345e-8, "1.2345e-8"),
            (f64::NAN, "null"),
            (f64::INFINITY, "null"),
        ] {
            assert_eq!(format_number(number), text, "for {number}");
        }
    }

    /// Serializing through a replacer must stay near linear in the
    /// number of keys.
    ///
    /// Looking each replacer name up by scanning the object's entry list
    /// costs `n` comparisons per name, so an object with `n` keys and a
    /// replacer naming all of them cost `n^2`. Both sides are
    /// user-controlled (`-o JSON.replacer=[...]` and the source), which is
    /// what makes it the untrusted-input concern `../AGENTS.md` sets out
    /// rather than a matter of taste.
    ///
    /// The check is machine-INDEPENDENT: it multiplies the key count by
    /// `GROWTH` and compares the two timings from the SAME run on the
    /// SAME machine, so a slow box cannot make it flaky. Linear work
    /// grows by `GROWTH`; quadratic work grows by `GROWTH * GROWTH`, and
    /// the budget sits between the two. There is deliberately no
    /// wall-clock figure.
    #[test]
    fn a_replacer_lookup_stays_near_linear() {
        const KEYS: usize = 2_000;
        const GROWTH: usize = 4;
        // Halfway between linear (4x) and quadratic (16x), on a log
        // scale, leaving room for allocation noise on either side.
        const BUDGET: f64 = 8.0;

        // Warm the paths so the comparison is steady state.
        replacer_nanos(KEYS / 8);

        let small = replacer_nanos(KEYS);
        let large = replacer_nanos(KEYS * GROWTH);

        // Guard the baseline before dividing by it: a zero measurement
        // makes every ratio vacuously true, which is a green test
        // asserting nothing.
        assert!(
            0 < small,
            "serializing {KEYS} keys measured as zero elapsed time: this is \
             testing the clock, not the code"
        );

        let ratio = large as f64 / small as f64;
        eprintln!(
            "replacer lookup: {KEYS} keys={small}ns, {} keys={large}ns, ratio={ratio:.2}x",
            KEYS * GROWTH
        );
        assert!(
            ratio < BUDGET,
            "serializing {GROWTH} times as many keys through a replacer took {ratio:.1}x \
             as long (want under {BUDGET}x, linear is about {GROWTH}x). The replacer \
             lookup is scanning the entry list per name again, which is quadratic in \
             the key count and reachable from the command line."
        );
    }

    /// Nanoseconds spent serializing an object of `keys` keys through a
    /// replacer that names every one of them.
    fn replacer_nanos(keys: usize) -> u128 {
        let names: Vec<String> = (0..keys).map(|at| format!("k{at:07}")).collect();
        let entries: IndexMap<String, Value> = names
            .iter()
            .enumerate()
            .map(|(at, name)| (name.clone(), Value::Number(at as f64)))
            .collect();
        let value = Value::object(entries);

        let started = std::time::Instant::now();
        let out = stringify(&value, Some(&names), "");
        let elapsed = started.elapsed().as_nanos();
        assert!(out.len() > keys, "the whole object was serialized");
        elapsed
    }

    #[test]
    fn control_characters_escape() {
        let mut out = String::new();
        write_string(&mut out, "a\u{1}b\nc");
        assert_eq!(out, r#""a\u0001b\nc""#);
    }
}
