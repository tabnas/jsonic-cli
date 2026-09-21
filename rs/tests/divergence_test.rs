/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! The recorded divergences from the canonical TypeScript command.
//!
//! `../test/divergent.tsv` is the executable register: one row per input
//! where this port produces a different RESULT, run through
//! `tabnas_support::Register`, which asserts the `rust` column and fails
//! just as loudly when a divergence is FIXED as when one regresses.
//!
//! Some disagreements a fixture cell cannot express, because the register
//! compares one printed value and these are about the exit code, standard
//! error, or text whose volume is engine dependent. Those are pinned
//! here, one test each, and described in `../DIVERGENCE.md`.

mod common;

use std::path::Path;

use common::run;
use tabnas_support::{parse_expect, Failure, Register, Row, Runner, Value};

/// The register file: `test/divergent.tsv`, beside the shared `spec`
/// directory rather than inside it. Everything in `test/spec` is
/// discovered and run by all three suites, and this file records what one
/// port does.
fn register_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rs/ has a parent")
        .join("test")
        .join("divergent.tsv")
}

/// One register row: the `argv` column is a JSON array of arguments, the
/// answer is the first printed line, and a failed run answers with the
/// engine's error code so an `ERROR:<code>` cell can pin it.
fn run_row(input: &str, row: &Row) -> Result<Value, Failure> {
    let argv: Vec<String> = serde_json::from_str(input)
        .map_err(|error| Failure::message(format!("argv is not a JSON array: {error}")))?;
    let list: Vec<&str> = argv.iter().map(String::as_str).collect();

    let out = run(&list, &row.unesc_named("stdin"));
    if out.code != 0 {
        return Err(Failure::new(error_code(&out.stderr)).with_message(out.stderr));
    }
    Ok(Value::String(out.first().to_string()))
}

/// The code out of a rendered engine report, which opens
/// `[jsonic/<code>]:`. The report is colourised, so the tag is found
/// rather than read off the front.
fn error_code(stderr: &str) -> String {
    stderr
        .split_once("[jsonic/")
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(code, _)| code.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[test]
fn register() {
    let path = register_path();
    assert!(path.is_file(), "{} is missing", path.display());
    Register::new(
        Runner::new_with_row(run_row)
            .parse_expected(|expected, _row| parse_expect(expected))
            .input("argv"),
        "rust",
        &["ts", "go", "rust"],
    )
    .file(&path);
}

/// A failure sets a NONZERO exit code here, and does not in the canonical
/// command.
///
/// `ts/bin/jsonic` is
/// `run(process.argv, console).catch((e) => console.error(e.message))`.
/// The rejection is caught, the message printed, and nothing sets
/// `process.exitCode`, so the canonical `jsonic` exits 0 after reporting a
/// parse failure, a missing `--file` or an unresolvable `-p`. A shell that
/// tests `$?` cannot tell a failed parse from a successful one. Both ports
/// exit 1 instead, which is the behaviour a command line expects, and the
/// repair belongs in the TypeScript bin.
///
/// Invisible to the shared fixtures: they compare the first line of
/// standard output, and a failed run prints nothing there in any runtime.
#[test]
fn a_failure_exits_nonzero_where_typescript_exits_zero() {
    for args in [
        vec!["}"],
        vec!["-f", "no-such-file.jsonic"],
        vec!["-p", "no-such-plugin", "a:1"],
        vec!["-o", "JSON.space='"],
    ] {
        let out = run(&args, "");
        assert_eq!(out.code, 1, "{args:?} did not exit 1");
        assert!(out.lines.is_empty(), "{args:?} printed to stdout");
        assert!(!out.stderr.is_empty(), "{args:?} failed silently");
    }
}

/// An unresolvable `-p` reports a registry miss, not a module-loader
/// failure.
///
/// TypeScript throws `require`'s own error, whose message is
/// `Cannot find module 'no-such-plugin'` with the code
/// `MODULE_NOT_FOUND`. Rust cannot load a module by name at all, so a
/// reference resolves against the compiled-in registry and a miss is
/// reported as one. Go makes the same adaptation and prints the same
/// sentence.
#[test]
fn an_unresolvable_plugin_reports_a_registry_miss() {
    let out = run(&["-p", "no-such-plugin", "a:1"], "");
    assert_eq!(out.code, 1);
    assert_eq!(out.stderr.trim(), "Plugin not found: no-such-plugin");
}

/// The help text names the plugins that are COMPILED IN, where the
/// canonical text names module references a distribution can install.
///
/// Both texts share everything else, and the shared `help.tsv` fixture
/// pins the part they share. This pins the part they do not, so that a
/// help text quietly drifting back to a promise Rust cannot keep fails
/// here.
#[test]
fn the_help_text_describes_compiled_in_plugins() {
    let out = run(&["-h"], "");
    let help = out.first();
    assert!(help.contains("Usage:"), "the shared part is still there");
    assert!(
        help.contains("Plugins are compiled into the binary"),
        "the Plugins section does not say plugins are compiled in"
    );
    assert!(
        help.contains("debug, jsonic, json"),
        "the Plugins section does not name the built-in references"
    );
    assert!(
        !help.contains("./plugin folder of the distribution"),
        "the Plugins section promises runtime module loading"
    );
}

/// `--debug` traces through `tabnas-debug`, not through the engine log
/// the canonical command switches on.
///
/// TypeScript's `-d` installs `@tabnas/debug` and adds `--meta log=-1`,
/// and the engine's log is what produces the token-by-token trace. The
/// Rust engine has no `log` meta: the trace comes from the debug plugin's
/// own subscribers. So the two ends agree exactly, the description header
/// and the JSON result, and the lines between them do not. Go makes the
/// same adaptation.
///
/// Not expressible as a register row: the trace volume is engine
/// dependent, so a cell holding it would pin the engine's version rather
/// than this command's behaviour.
#[test]
fn the_debug_trace_comes_from_the_plugin() {
    let out = run(&["-d", "a:1"], "");
    assert_eq!(out.code, 0);
    assert!(out.first().contains("=== PARSE ==="), "{:?}", out.first());
    assert_eq!(out.last(), r#"{"a":1}"#);
    assert!(
        2 < out.lines.len(),
        "nothing was traced between the two ends: {:?}",
        out.lines.len()
    );
    // `log=-1` is still pushed onto the meta bag, because the argument
    // loop is the canonical one; it simply means nothing to this engine.
    assert_eq!(
        run(&["-m", "log=-1", "a:1"], "").first(),
        r#"{"a":1}"#,
        "the log meta is inert, not an error"
    );
}

/// Everything in the help text OUTSIDE the plugin sections is the
/// canonical text, read from `ts/src/jsonic-cli.ts` rather than copied
/// here, so the two cannot drift apart without this going red.
///
/// Two allowances, both measured against that file:
///
/// - **Trailing whitespace.** Six lines of the canonical template end in
///   a space. The Go port dropped them and so does this one; nothing
///   prints differently that a reader can see, and an editor that strips
///   trailing whitespace would otherwise break the comparison for a
///   change that touched nothing.
/// - **The plugin sections.** The `Plugins` block and the two plugin
///   examples describe module references the canonical command can load
///   and this one cannot. That is the divergence
///   `the_help_text_describes_compiled_in_plugins` pins.
#[test]
fn the_help_text_outside_the_plugin_sections_is_the_canonical_text() {
    let canonical = canonical_help();
    let ours = run(&["-h"], "").first().to_string();

    // The flag surface: everything up to the `Plugins` heading.
    let head = |text: &str| -> String {
        let at = text.find("\nPlugins").expect("a Plugins heading");
        trimmed(&text[..at])
    };
    assert_eq!(head(&ours), head(&canonical), "the flag surface differs");

    // The tail, from the last example the two share.
    let tail = |text: &str| -> String {
        let at = text
            .find("# Full debug tracing")
            .expect("a debug tracing example");
        trimmed(&text[at..])
    };
    assert_eq!(tail(&ours), tail(&canonical), "the help text tail differs");
}

/// The canonical help text, read out of the template literal in
/// `ts/src/jsonic-cli.ts`.
fn canonical_help() -> String {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("rs/ has a parent")
            .join("ts")
            .join("src")
            .join("jsonic-cli.ts"),
    )
    .expect("the canonical source is readable");
    let open = "let s = `";
    let at = source.find(open).expect("the help template literal") + open.len();
    let rest = &source[at..];
    let end = rest
        .find("`\n\n  console.log(s)")
        .expect("the end of the help template literal");
    // The only escape the template carries is an escaped backtick.
    rest[..end].replace("\\`", "`")
}

/// Every line with its trailing whitespace removed.
fn trimmed(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .collect::<Vec<&str>>()
        .join("\n")
}

/// An `-o` naming an option the engine types as a FUNCTION reference is
/// refused here and accepted, silently, in the canonical command.
///
/// `ts/src/jsonic-cli.ts` hands the option bag to `Jsonic.make(options)`,
/// which deep-merges it into the defaults and validates nothing: JavaScript
/// stores `true` where a function belongs and only trips over it if the
/// grammar ever calls it. `map.merge` is read at one place only
/// (`jsonic/ts/src/grammar.ts`: `ctx.cfg.map.merge ? ctx.cfg.map.merge(...)
/// : deep(...)`), and only for a REPEATED key, so `jsonic -o map.merge=true
/// a:1` prints `{"a":1}` there.
///
/// The Rust engine validates the option document as it installs it, so the
/// same argument is refused before anything is parsed. That is not a
/// repairable difference in this crate: a `-o` value is text, and no text
/// names a Rust function. The alternative, swallowing the rejection, would
/// hide every real mistake in an option document.
///
/// Go accepts these too, because its option bag is `map[string]any` and
/// nothing checks it either.
///
/// Not expressible as a register row: the register reads a failed run as
/// `ERROR:<code>`, and this failure is the command reporting a grammar
/// build error, which carries no engine error code.
#[test]
fn a_function_typed_option_is_refused_rather_than_ignored() {
    for option in [
        "map.merge=true",
        "text.modify=true",
        "parse.prepare.x=true",
        "parse.budget.onCheck=true",
        "parser.start=true",
        "config.modify.x=true",
    ] {
        let out = run(&["-o", option, "a:1"], "");
        assert_eq!(out.code, 1, "{option} was accepted: {:?}", out.lines);
        assert!(
            out.stderr.starts_with("Grammar: options."),
            "{option}: {}",
            out.stderr
        );
    }

    // An option the engine types as DATA still goes through, so the
    // rejection is about function references and not about `-o` itself.
    for option in ["map.extend=false", "number.hex=false", "value.def.q.val=1"] {
        let out = run(&["-o", option, "a:1"], "");
        assert_eq!(out.code, 0, "{option}: {}", out.stderr);
        assert_eq!(out.first(), r#"{"a":1}"#, "{option}");
    }
}
