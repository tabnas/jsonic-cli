/* Copyright (c) 2013-2026 Richard Rodger and other contributors, MIT License */

//! The port of `ts/test/cli.test.js` and of `go/cli/run_test.go`. Each
//! test mirrors one block of the TypeScript suite: the same arguments,
//! the same expected output. The TypeScript tests call `run()` in process
//! with a fake console and read `cn.d.log[0][0]`; these call `capture()`
//! and read the printed entries.
//!
//! Two TypeScript behaviours have no Rust equivalent and are adapted
//! rather than dropped, exactly as the Go port adapts them:
//!
//! - **Loading a plugin by `require(<reference>)`.** Rust cannot load a
//!   module by name at run time, so the four export-shape fixtures
//!   (`p0`, `p1`, `p2`, `pa-qa`) become compiled-in plugins injected
//!   under the same reference names. What survives the adaptation is the
//!   option plumbing; the export shapes themselves have no counterpart.
//! - **Reading `./test/foo.jsonic`.** These read
//!   `rs/tests/testdata/foo.jsonic`, committed with the same contents.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use common::{run, run_streams, run_with, test_plugins, value_def_plugin};

/// A committed `--file` fixture, as an absolute path: an integration test
/// runs from the crate root, and naming the file absolutely keeps the
/// case about `--file` rather than about the working directory.
fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join(name)
        .to_str()
        .expect("a utf-8 path")
        .to_string()
}

#[test]
fn basic() {
    let out = run(&["a:1"], "");
    assert_eq!(out.first(), r#"{"a":1}"#);
    assert_eq!(out.code, 0);

    let out = run(&["-o", "number.lex=false", "a:1"], "");
    assert_eq!(out.first(), r#"{"a":"1"}"#);
}

#[test]
fn args() {
    for flag in ["-h", "--help"] {
        let out = run(&[flag], "");
        assert!(out.first().contains("Usage:"), "{flag}: {}", out.first());
        assert_eq!(out.code, 0);
    }

    // Several positional sources, each parsed and merged.
    let out = run(&["a:1", "b:[2]", "c:{x:1}"], "");
    assert_eq!(out.first(), r#"{"a":1,"b":[2],"c":{"x":1}}"#);

    // A file, then a positional source, merged.
    let out = run(&["-f", &fixture("foo.jsonic"), "zed:2"], "");
    assert_eq!(out.first(), r#"{"bar":1,"zed":2}"#);

    // Two `--file` sources, merged last-wins, in both spellings.
    let out = run(
        &[
            "-f",
            &fixture("foo.jsonic"),
            "--file",
            &fixture("bar.jsonic"),
        ],
        "",
    );
    assert_eq!(out.first(), r#"{"bar":1,"qaz":2}"#);

    // An unknown flag is a source, not an error.
    let out = run(&["--not-an-arg-so-ignored", "a:1"], "");
    assert_eq!(out.first(), r#"{"a":1}"#);

    // Standard input alone.
    let out = run(&[], "{a:1}");
    assert_eq!(out.first(), r#"{"a":1}"#);

    // `-` is the alias for standard input, alone and with a source.
    let out = run(&["-"], "{a:1}");
    assert_eq!(out.first(), r#"{"a":1}"#);
    let out = run(&["-", "b:2"], "{a:1}");
    assert_eq!(out.first(), r#"{"a":1,"b":2}"#);
}

/// Both spellings of every flag that has a long form, and the `--k=v`
/// form the canonical command does NOT accept.
#[test]
fn long_and_short_spellings() {
    for (long, short) in [
        ("--option", "-o"),
        ("--meta", "-m"),
        ("--file", "-f"),
        ("--plugin", "-p"),
    ] {
        assert_eq!(
            run(&[long, "x=1", "a:1"], "").first(),
            run(&[short, "x=1", "a:1"], "").first(),
            "{long} and {short} disagree",
        );
    }
    assert_eq!(run(&["--nice", "a:1"], "").first(), "{\n  \"a\": 1\n}");
    assert_eq!(run(&["-n", "a:1"], "").first(), "{\n  \"a\": 1\n}");

    // `--option=JSON.space=2` is not `--option`: the argument loop
    // compares whole tokens, so the `--k=v` form falls through to the
    // sources and is parsed as relaxed JSON. Pinned because it is the
    // difference between a flag that is silently ignored and one that is
    // silently a source.
    let out = run(&["--option=JSON.space=2", "a:1"], "");
    assert_eq!(out.first(), r#"{"a":1}"#);
    let out = run(&["--option=JSON.space=2"], "");
    assert_eq!(out.first(), r#""--option=JSON.space=2""#);
    let out = run(&["--nice=1", "a:1"], "");
    assert_eq!(out.first(), r#"{"a":1}"#);
}

#[test]
fn bad_args() {
    // An empty `--file` is skipped rather than read.
    assert_eq!(run(&["-f", "", "a:1"], "").first(), r#"{"a":1}"#);

    // Empty and malformed `-o` values are no-ops.
    for option in ["", "=", "bad="] {
        let out = run(&["-o", option, "a:1"], "");
        assert_eq!(out.first(), r#"{"a":1}"#, "-o {option:?}");
        assert_eq!(out.code, 0);
    }

    // A value-taking flag in the last position consumes nothing.
    for flag in ["-o", "-m", "-f", "-p"] {
        let out = run(&["a:1", flag], "");
        assert_eq!(out.first(), r#"{"a":1}"#, "trailing {flag}");
        assert_eq!(out.code, 0);
    }
}

/// A `--file` that does not exist is reported and fails, rather than
/// being ignored. The TypeScript `Fs.readFileSync` throws out of `run()`
/// on the same input.
#[test]
fn a_missing_file_fails() {
    let out = run(&["-f", "no-such-file.jsonic", "a:1"], "");
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.contains("no-such-file.jsonic"),
        "stderr: {}",
        out.stderr
    );
}

/// A source that does not parse is reported and fails.
#[test]
fn a_bad_source_fails() {
    let out = run(&["a:1,"], "");
    assert_eq!(out.code, 0, "a trailing comma is legal jsonic");

    let out = run(&["}"], "");
    assert_eq!(out.code, 1);
    assert!(!out.stderr.is_empty(), "a parse failure says why");
}

/// `--debug` / `-d` installs the debug plugin, prints the grammar
/// description ahead of the parse and still emits the JSON result last.
/// The trace volume is engine dependent, so the two ends are pinned, not
/// a line count.
#[test]
fn debug_flag() {
    for flag in ["-d", "--debug"] {
        let out = run(&[flag, "a:1"], "");
        assert_eq!(out.code, 0, "{flag}");
        assert!(
            out.first().contains("=== PARSE ==="),
            "{flag}: no describe header in {:?}",
            out.first()
        );
        assert!(1 < out.lines.len(), "{flag}: nothing traced");
        assert_eq!(out.last(), r#"{"a":1}"#, "{flag}");
    }

    // The long form, with options that reach both the debug plugin and
    // the engine, and `--` ending flag parsing: `value.lex=false` keeps
    // `true` a string.
    let out = run(
        &[
            "--debug",
            "-o",
            "debug.maxlen=11",
            "--option",
            "value.lex=false",
            "--",
            "a:true",
        ],
        "",
    );
    assert_eq!(out.code, 0);
    assert!(out.first().contains("=== PARSE ==="));
    assert_eq!(out.last(), r#"{"a":"true"}"#);

    // `-p debug` without `-d` describes the grammar and traces nothing,
    // which is what TypeScript does without the `log=-1` metadata: the
    // description and the result are all that is printed.
    let out = run(&["-p", "debug", "a:1"], "");
    assert_eq!(out.lines.len(), 2, "{:?}", out.lines);
    assert_eq!(out.last(), r#"{"a":1}"#);

    // Nothing a `-o` can say turns the `-d` trace off, as nothing can in
    // TypeScript, where the trace comes from the metadata `-d` pushes.
    let out = run(&["-d", "-o", "plugin.debug.trace=false", "a:1"], "");
    assert!(2 < out.lines.len(), "{:?}", out.lines.len());
    assert_eq!(out.last(), r#"{"a":1}"#);
}

/// `-p` against the real compiled-in registry: the shipped binary's
/// built-in plugins are debug, jsonic and json, resolvable by bare name
/// and by the `@tabnas/<name>` form.
#[test]
fn builtin_plugins() {
    let registry = tabnas_jsonic_cli::plugins();
    for name in ["debug", "jsonic", "json"] {
        assert!(registry.contains_key(name), "built-in {name} is missing");
    }

    // `-p debug` prints the grammar description before the result, as
    // `-d` does.
    let out = run(&["-p", "debug", "a:1"], "");
    assert_eq!(out.code, 0);
    assert!(out.first().contains("=== PARSE ==="), "{:?}", out.lines);
    assert_eq!(out.last(), r#"{"a":1}"#);

    // The `@tabnas/<name>` reference resolves to the same plugin, and
    // prints NO describe header: TypeScript keys that header by the
    // reference (`null != plugins.debug`), so only the bare name turns it
    // on. Go keys it the same way.
    let out = run(&["-p", "@tabnas/debug", "a:1"], "");
    assert_eq!(out.code, 0);
    assert_eq!(out.last(), r#"{"a":1}"#);
    assert!(
        !out.first().contains("=== PARSE ==="),
        "a scoped reference printed the describe header: {:?}",
        out.first()
    );

    // `-p jsonic` is the grammar itself: a no-op on the CLI's instance.
    let out = run(&["-p", "jsonic", "a:1"], "");
    assert_eq!(out.code, 0);
    assert_eq!(out.first(), r#"{"a":1}"#);

    // `-p json` restricts the parser to standard JSON.
    let out = run(&["-p", "json", r#"{"a":1}"#], "");
    assert_eq!(out.code, 0);
    assert_eq!(out.first(), r#"{"a":1}"#);
    let out = run(&["-p", "json", "a:1"], "");
    assert_eq!(out.code, 1, "relaxed source under -p json: {:?}", out.lines);

    // An unresolvable reference is an error. TypeScript throws the
    // original `require` failure out of `run()`; here, as in Go, the name
    // simply is not in the registry.
    let out = run(&["-p", "no-such-plugin", "a:1"], "");
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.contains("no-such-plugin"),
        "stderr: {}",
        out.stderr
    );
}

/// `register_plugin` extends the registry for a custom binary.
#[test]
fn a_registered_plugin_resolves() {
    tabnas_jsonic_cli::register_plugin(
        "cli-test-fixture",
        value_def_plugin("cli-test-fixture", "F", "f"),
    );
    let out = run(
        &[
            "-p",
            "cli-test-fixture",
            "-o",
            "plugin.cli-test-fixture.f=9",
            "a:F",
        ],
        "",
    );
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert_eq!(out.first(), r#"{"a":9}"#);
}

#[test]
fn plugin_options() {
    let plugins = test_plugins();

    // p0: value def X becomes 0.
    let out = run_with(&["-p", "p0", "-o", "plugin.p0.x=0", "a:X"], "", &plugins);
    assert_eq!(out.first(), r#"{"a":0}"#);

    // p0 with the key overridden to W.
    let out = run_with(
        &[
            "-p",
            "p0",
            "-o",
            "plugin.p0.x=0",
            "-o",
            "plugin.p0.s=W",
            "a:W",
        ],
        "",
        &plugins,
    );
    assert_eq!(out.first(), r#"{"a":0}"#);

    // p1, with the option given before the plugin.
    let out = run_with(&["-o", "plugin.p1.y=1", "-p", "p1", "a:Y"], "", &plugins);
    assert_eq!(out.first(), r#"{"a":1}"#);

    // p0 and p1 together.
    let out = run_with(
        &[
            "-o",
            "plugin.p0.x=0",
            "-p",
            "p0",
            "-o",
            "plugin.p1.y=1",
            "-p",
            "p1",
            "a:X,b:Y",
        ],
        "",
        &plugins,
    );
    assert_eq!(out.first(), r#"{"a":0,"b":1}"#);

    // p2.
    let out = run_with(&["-p", "p2", "-o", "plugin.p2.z=2", "a:Z"], "", &plugins);
    assert_eq!(out.first(), r#"{"a":2}"#);

    // pa-qa.
    let out = run_with(
        &["-p", "paqa", "-o", "plugin.paqa.q=3", "a:Q"],
        "",
        &plugins,
    );
    assert_eq!(out.first(), r#"{"a":3}"#);

    // A path-like reference resolves by its base name, which stands in
    // for the TypeScript `require('../test/p0')`. The option still
    // arrives, because the engine keys the option sub-bag by the
    // PLUGIN'S name, not by the reference that found it.
    let out = run_with(
        &["-p", "../test/p0.js", "-o", "plugin.p0.x=0", "a:X"],
        "",
        &plugins,
    );
    assert_eq!(out.first(), r#"{"a":0}"#, "stderr: {}", out.stderr);
}

#[test]
fn stringify() {
    assert_eq!(
        run(&["-o", "JSON.space=2", "a:1"], "").first(),
        "{\n  \"a\": 1\n}"
    );
    assert_eq!(run(&["-n", "a:1"], "").first(), "{\n  \"a\": 1\n}");
    assert_eq!(
        run(&["-o", "JSON.replacer=[b]", "a:1,b:2"], "").first(),
        r#"{"b":2}"#
    );
    assert_eq!(
        run(&["-o", "JSON.replacer=b", "a:1,b:2"], "").first(),
        r#"{"b":2}"#
    );
}

/// The streaming entry point the binary calls writes the same text, one
/// newline-terminated line per printed entry.
#[test]
fn the_streaming_entry_point_agrees() {
    let (code, stdout, stderr) = run_streams(&["a:1"], "");
    assert_eq!(code, 0);
    assert_eq!(stdout, "{\"a\":1}\n");
    assert!(stderr.is_empty());

    let (code, stdout, _) = run_streams(&["-n", "a:1"], "");
    assert_eq!(code, 0);
    assert_eq!(stdout, "{\n  \"a\": 1\n}\n");
}

/// A plugin that fails to install is reported rather than panicking.
#[test]
fn a_failing_plugin_is_reported() {
    let mut extra: BTreeMap<String, tabnas::Plugin> = BTreeMap::new();
    extra.insert(
        "boom".to_string(),
        tabnas::Plugin::new("boom", |_parser, _options| {
            Err(tabnas::PluginError("no".to_string()))
        }),
    );
    let out = run_with(&["-p", "boom", "a:1"], "", &extra);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("no"), "stderr: {}", out.stderr);
}
