/* Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
contributors, MIT License */

//! The CLI's source merge: a port of the engine's `util.deep`, which is
//! what `ts/src/jsonic-cli.ts` folds each parsed source in with.
//!
//! This is NOT [`tabnas_jsonic::deep_merge`], and the difference is the
//! whole reason the module exists. That function merges two values when
//! BOTH are containers and otherwise takes the overlay as it stands, so
//! a prototype-poisoning key survives whenever the base is not already a
//! container of the same kind: with the seed value `null`, `jsonic
//! __proto__:1` printed `{"__proto__":1}` through it. TypeScript's
//! `deep(base, over)` reaches the guarded loop even then, because a
//! non-container base makes it merge the overlay into a FRESH empty
//! container of the overlay's kind, and that loop skips `__proto__`,
//! `constructor` and `prototype` at every level. `{}` is what the
//! canonical command prints, and what the Go port prints.
//!
//! Rust has no prototype chain to poison, so nothing here is a security
//! fix. It is an output-parity fix: the command prints what the canonical
//! command prints for the same argument.

use indexmap::IndexMap;
use tabnas::{ListRef, MapRef, Value};

/// Keys `util.deep` refuses to copy. In JavaScript each one reaches the
/// prototype chain; here they are skipped to keep the same output.
const DANGEROUS: [&str; 3] = ["__proto__", "constructor", "prototype"];

/// Merge `over` into `base`, the way `util.deep(base, over)` does.
///
/// Objects merge by key and arrays by index, with anything the overlay
/// does not mention left as it was. A scalar overlay replaces the base, an
/// `undefined` overlay leaves it alone, and a container overlay onto a
/// base that is not a container of the same kind is copied into a fresh
/// one, which is where the dangerous keys are dropped.
///
/// An index the base does not reach yet is not a special case: it merges
/// onto the `undefined` that TypeScript reads there, so an element the
/// overlay APPENDS passes through the same guarded loop as one it
/// replaces.
pub(crate) fn deep(base: Value, over: Value) -> Value {
    let base_kind = kind(&base);
    let over_kind = kind(&over);

    match (base_kind, over_kind) {
        (Kind::Object, Kind::Object) => {
            let mut entries = into_entries(base);
            for (key, value) in into_entries(over) {
                if DANGEROUS.contains(&key.as_str()) {
                    continue;
                }
                // `base[k] = deep(base[k], over[k])` in TypeScript, which
                // leaves an EXISTING key where it already sat. Taking the
                // previous value out with `insert` rather than
                // `shift_remove` is what keeps that position, and the
                // position is the printed key order.
                let previous = entries
                    .insert(key.clone(), Value::Undefined)
                    .unwrap_or(Value::Undefined);
                entries.insert(key, deep(previous, value));
            }
            Value::object(entries)
        }
        (Kind::Array, Kind::Array) => {
            let mut items = into_items(base);
            for (at, value) in into_items(over).into_iter().enumerate() {
                // `base[k] = deep(base[k], over[k])` for EVERY index the
                // overlay mentions, one past the end of the base
                // included: `base[k]` reads as `undefined` there, and
                // TypeScript still runs the merge, which copies a
                // container overlay into a fresh container through the
                // guarded loop. Taking the overlay element as it stands
                // instead skipped that loop, so a dangerous key inside a
                // newly appended object survived.
                let previous = if at < items.len() {
                    std::mem::replace(&mut items[at], Value::Undefined)
                } else {
                    items.push(Value::Undefined);
                    Value::Undefined
                };
                items[at] = deep(previous, value);
            }
            Value::array(items)
        }
        // `undefined === over ? base` — an empty source is a no-op.
        (_, Kind::Undefined) => base,
        // A container onto a base that cannot take it is copied into a
        // fresh one of its own kind, through the guarded loop above.
        (_, Kind::Object) => deep(Value::object(IndexMap::new()), over),
        (_, Kind::Array) => deep(Value::array(Vec::new()), over),
        // Every other overlay simply replaces the base.
        (_, Kind::Scalar) => over,
    }
}

/// What `util.deep` treats a value as. A `Text` is a STRING in
/// TypeScript, carrying its quote on a non-enumerable property, so it
/// merges as a scalar and not as a container.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Undefined,
    Scalar,
    Object,
    Array,
}

fn kind(value: &Value) -> Kind {
    match value {
        Value::Undefined => Kind::Undefined,
        Value::Object(_) | Value::MapRef(_) => Kind::Object,
        Value::Array(_) | Value::ListRef(_) => Kind::Array,
        _ => Kind::Scalar,
    }
}

/// An object value's entries, unwrapping a `MapRef`.
fn into_entries(value: Value) -> IndexMap<String, Value> {
    match value {
        Value::Object(entries) => unwrap(entries),
        Value::MapRef(map) => match std::sync::Arc::try_unwrap(map) {
            Ok(MapRef { value, .. }) => value,
            Err(shared) => shared.value.clone(),
        },
        _ => IndexMap::new(),
    }
}

/// An array value's items, unwrapping a `ListRef`.
fn into_items(value: Value) -> Vec<Value> {
    match value {
        Value::Array(items) => unwrap(items),
        Value::ListRef(list) => match std::sync::Arc::try_unwrap(list) {
            Ok(ListRef { value, .. }) => value,
            Err(shared) => shared.value.clone(),
        },
        _ => Vec::new(),
    }
}

/// The contents of an `Arc`, cloned only when it is shared.
fn unwrap<T: Clone>(shared: std::sync::Arc<T>) -> T {
    std::sync::Arc::try_unwrap(shared).unwrap_or_else(|shared| (*shared).clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Value {
        tabnas_jsonic::parse(source).expect("the fixture parses")
    }

    fn merged(sources: &[&str]) -> String {
        let mut data = Value::Null;
        for source in sources {
            data = deep(data, parse(source));
        }
        crate::stringify::stringify(&data, None, "")
    }

    #[test]
    fn objects_merge_by_key_and_arrays_by_index() {
        assert_eq!(merged(&["a:b:1", "a:c:2"]), r#"{"a":{"b":1,"c":2}}"#);
        assert_eq!(merged(&["[1,2,3]", "[9]"]), "[9,2,3]");
        assert_eq!(merged(&["a:1", "b:2", "a:3"]), r#"{"a":3,"b":2}"#);
    }

    #[test]
    fn an_empty_source_leaves_the_base_alone() {
        assert_eq!(merged(&["a:1", ""]), r#"{"a":1}"#);
        assert_eq!(merged(&[""]), "null");
    }

    #[test]
    fn a_scalar_overlay_replaces_the_base() {
        assert_eq!(merged(&["a:1", "2"]), "2");
        assert_eq!(merged(&["[1,2]", "a:1"]), r#"{"a":1}"#);
    }

    #[test]
    fn the_dangerous_keys_are_dropped_at_every_level() {
        for key in DANGEROUS {
            assert_eq!(merged(&[&format!("{key}:1")]), "{}", "top level {key}");
            assert_eq!(
                merged(&[&format!("a:{{{key}:1}}")]),
                r#"{"a":{}}"#,
                "nested {key}"
            );
            assert_eq!(
                merged(&["a:{b:1}", &format!("a:{{{key}:1}}")]),
                r#"{"a":{"b":1}}"#,
                "merged {key}"
            );
        }
    }

    /// An APPENDED array element reaches the guarded loop too.
    ///
    /// TypeScript merges every index the overlay mentions, so an element
    /// past the end of the base is merged into the `undefined` sitting
    /// there and copied into a fresh container. Measured under Node
    /// against `ts/src/jsonic-cli.ts`: `jsonic 'a:[{__proto__:1}]'`
    /// prints `{"a":[{}]}`, and `jsonic '[1]' '[{__proto__:1},{__proto__:2}]'`
    /// prints `[{},{}]`.
    #[test]
    fn the_dangerous_keys_are_dropped_from_an_appended_element() {
        for key in DANGEROUS {
            assert_eq!(
                merged(&[&format!("a:[{{{key}:1}}]")]),
                r#"{"a":[{}]}"#,
                "fresh array, {key}"
            );
            assert_eq!(
                merged(&[&format!("[{{{key}:1}}]")]),
                "[{}]",
                "top level array, {key}"
            );
            // Index 0 replaces an existing element and index 1 appends
            // one: both must be guarded, which is what caught the defect.
            assert_eq!(
                merged(&["[1]", &format!("[{{{key}:1}},{{{key}:2}}]")]),
                "[{},{}]",
                "replaced and appended, {key}"
            );
            assert_eq!(
                merged(&[&format!("a:[[{{{key}:1}}]]")]),
                r#"{"a":[[{}]]}"#,
                "nested array, {key}"
            );
        }
    }
}
