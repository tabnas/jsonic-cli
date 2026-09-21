/* Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
contributors, MIT License */

//! A port of JavaScript's `JSON.stringify(value, replacer, space)` over
//! the engine's [`tabnas::Value`]. The port of `go/cli/stringify.go`.
//!
//! The CLI's whole output contract is this function: whatever the
//! canonical command prints for a value, this prints for the same value,
//! key order and number formatting included.

use std::collections::BTreeSet;

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
/// much. A character straddling the boundary is dropped whole: JavaScript
/// would keep half of it, and half of a surrogate pair is not a value a
/// Rust string can hold.
fn truncate_utf16(text: &str, units: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let width = ch.len_utf16();
        if units < used + width {
            break;
        }
        out.push(ch);
        used += width;
    }
    out
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
    let entries: Vec<(&String, &Value)> = entries.collect();

    // With a replacer list, `JSON.stringify` walks the PropertyList and
    // looks each key up, so the OUTPUT order is the replacer's order and
    // not the object's: `JSON.stringify({a:1,b:2}, ['b','a'])` is
    // `{"b":2,"a":1}`, measured under Node. Without one the object's own
    // order stands. Either way a key whose value is `undefined` is
    // omitted, as it is in JavaScript.
    let kept: Vec<(&String, &Value)> = match replacer {
        Some(keys) => keys
            .iter()
            .filter_map(|key| {
                entries
                    .iter()
                    .find(|(name, _)| *name == key)
                    .copied()
                    .filter(|(_, value)| !value.is_undefined())
            })
            .collect(),
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
/// The shortest round-trippable digits come from Rust's `{:e}`, which uses
/// the same shortest-representation algorithm as `{}` but states the
/// decimal exponent instead of laying the digits out. With the digit
/// string `s` (length `k`) and the exponent `n` such that the value is
/// `0.s * 10^n`, the ECMAScript rules choose between fixed and exponential
/// notation; Rust's own `{}` never chooses exponential, which is why this
/// cannot just be `format!("{number}")`.
fn positive_to_string(number: f64) -> String {
    let scientific = format!("{number:e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("Rust's {:e} always writes an exponent");
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

    #[test]
    fn control_characters_escape() {
        let mut out = String::new();
        write_string(&mut out, "a\u{1}b\nc");
        assert_eq!(out, r#""a\u0001b\nc""#);
    }
}
