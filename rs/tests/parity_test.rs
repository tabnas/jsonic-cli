/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! Cross-runtime conformance, driven by the shared `test/spec/*.tsv`
//! fixtures at the repository root (see `../test/AGENTS.md`).
//!
//! The fixture loader, the escape codec, the expectation vocabulary and
//! the row loop all come from `tabnas_support`, the Rust half of
//! `@tabnas/support`, whose TypeScript and Go halves run the SAME files in
//! `ts/test/parity.test.js` and `go/cli/parity_test.go`. So the three
//! commands cannot drift without one of them going red, and neither can
//! the three loaders.
//!
//! What is left here is only what is specific to this command: an `argv`
//! JSON array in, the FIRST printed line out, and the `CONTAINS:` form
//! the long help text is pinned with.

mod common;

use std::path::Path;

use tabnas_support::{
    find_spec_dir, load_spec_dir, parse_expect, Failure, Row, Runner, SpecOptions, Value,
};

/// The prefix an expected cell carries when only a fragment of a long
/// output is pinned.
const CONTAINS: &str = "CONTAINS:";

/// Every fixture in the spec directory. The files are discovered by
/// listing, so adding a `.tsv` runs it in all three runtimes without
/// touching any runner.
#[test]
fn spec() {
    let dir = find_spec_dir(Some(Path::new(env!("CARGO_MANIFEST_DIR")))).expect("test/spec");
    let specs = load_spec_dir(&dir, &SpecOptions::default()).expect("the fixtures load");
    assert!(!specs.is_empty(), "{}: no fixtures", dir.display());
    for spec in &specs {
        assert!(!spec.rows.is_empty(), "{}: no rows", spec.file);
    }

    Runner::new_with_row(run_row)
        .parse_expected(|expected, _row| {
            // A `CONTAINS:` cell's expectation IS the substring, which
            // `run_row` answers with when the output holds it.
            match expected.strip_prefix(CONTAINS) {
                Some(want) => Ok(Value::String(want.to_string())),
                None => parse_expect(expected),
            }
        })
        .input("argv")
        .expected("stdout")
        .dir(&dir);
}

/// One fixture row: the `argv` column is a JSON array of arguments and
/// the optional `stdin` column the text piped in. The answer is the first
/// printed line, or the pinned substring when the row holds one and the
/// output contains it, so that a row which fails reports what actually
/// came out.
fn run_row(input: &str, row: &Row) -> Result<Value, Failure> {
    let argv: Vec<String> = serde_json::from_str(input)
        .map_err(|error| Failure::message(format!("argv is not a JSON array: {error}")))?;
    let list: Vec<&str> = argv.iter().map(String::as_str).collect();

    let out = common::run(&list, &row.unesc_named("stdin"));
    let got = out.first().to_string();

    if let Some(want) = row.named("stdout").strip_prefix(CONTAINS) {
        if got.contains(want) {
            return Ok(Value::String(want.to_string()));
        }
    }
    Ok(Value::String(got))
}

/// Every fixture file is run. A file that stopped being discovered would
/// otherwise pass silently, which is the one failure a directory-driven
/// runner cannot report for itself.
#[test]
fn every_fixture_is_run() {
    let dir = find_spec_dir(Some(Path::new(env!("CARGO_MANIFEST_DIR")))).expect("test/spec");
    let specs = load_spec_dir(&dir, &SpecOptions::default()).expect("the fixtures load");
    let names: Vec<&str> = specs.iter().map(|spec| spec.file.as_str()).collect();
    for baseline in [
        "args.tsv",
        "bad-args.tsv",
        "basic.tsv",
        "help.tsv",
        "stdin.tsv",
        "stringify.tsv",
    ] {
        assert!(
            names.iter().any(|name| name.ends_with(baseline)),
            "{baseline} is no longer discovered; found {names:?}"
        );
    }
}
