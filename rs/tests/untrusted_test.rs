/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! Standard input is untrusted. Deep nesting, very long input, an
//! unterminated construct, no input at all, control characters, odd
//! Unicode and bytes that are not UTF-8 at all must be reported or
//! parsed, never panic, hang, overflow the stack or take super-linear
//! time.
//!
//! The nesting bound is the engine's: `tabnas-jsonic` refuses past 127
//! containers with the code `cancel`, which is what keeps the recursive
//! walk this command performs (the merge, and the serializer) off the
//! stack limit. These tests pin that boundary from the command's side,
//! because a command that segfaults on a piped file is a different defect
//! from a grammar that does.

mod common;

use std::time::Instant;

use common::run;

/// Nesting far past any real document is refused, and the process
/// survives to say so.
#[test]
fn deep_nesting_is_refused_rather_than_crashing() {
    for depth in [200usize, 5_000, 50_000] {
        let source = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let out = run(&[], &source);
        assert_eq!(out.code, 1, "depth {depth} was not refused");
        assert!(
            out.stderr.contains("cancel"),
            "depth {depth}: {}",
            &out.stderr[..out.stderr.len().min(200)]
        );
    }
}

/// Nesting just inside the bound still parses and still serializes, so
/// the limit is a limit and not an outage.
#[test]
fn nesting_just_inside_the_bound_parses() {
    let depth = 120;
    let source = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
    let out = run(&[], &source);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.first(), source);
}

/// A dive through keys nests implicit maps, and is bounded the same way.
#[test]
fn a_deep_key_dive_is_bounded() {
    let source = (0..5_000)
        .map(|_| "a")
        .collect::<Vec<_>>()
        .join(":")
        .to_string()
        + ":1";
    let out = run(&[], &source);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("cancel"), "{}", out.stderr);
}

/// An unterminated construct is reported, not a hang. jsonic closes an
/// unterminated CONTAINER for you, which is the relaxation it exists for,
/// so those parse; an unterminated string or comment has no sane closure
/// and is an error.
#[test]
fn an_unterminated_construct_is_reported() {
    for (source, want) in [
        ("{a:1", r#"{"a":1}"#),
        ("[1,2", "[1,2]"),
        ("{a:[1,{b:", r#"{"a":[1,{"b":null}]}"#),
    ] {
        let out = run(&[], source);
        assert_eq!(out.code, 0, "{source:?}: {}", out.stderr);
        assert_eq!(out.first(), want, "{source:?}");
    }

    for source in ["\"abc", "'abc", "`abc", "/* a"] {
        let out = run(&[], source);
        assert_eq!(out.code, 1, "{source:?} did not fail: {:?}", out.lines);
        assert!(
            out.stderr.contains("unterminated"),
            "{source:?}: {}",
            out.stderr
        );
    }
}

/// Nothing piped in prints the seed value, and so does an empty source
/// argument. Neither reads a stream that is not there.
#[test]
fn empty_input_prints_the_seed() {
    assert_eq!(run(&[], "").first(), "null");
    assert_eq!(run(&[""], "").first(), "null");
    assert_eq!(run(&["-"], "").first(), "null");
}

/// Bytes that are not UTF-8 fold to U+FFFD rather than failing the read.
#[test]
fn invalid_utf8_folds_rather_than_failing() {
    let mut stdout: Vec<u8> = Vec::new();
    let mut stderr: Vec<u8> = Vec::new();
    let argv = common::argv(&[]);
    let code = tabnas_jsonic_cli::run(&argv, &mut &b"a:'\xff\xfe'"[..], &mut stdout, &mut stderr);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert_eq!(
        String::from_utf8(stdout).expect("utf-8 stdout"),
        "{\"a\":\"\u{fffd}\u{fffd}\"}\n"
    );
}

/// Control characters and odd Unicode are handled without panicking:
/// either parsed or reported, never a crash.
#[test]
fn control_characters_and_odd_unicode_are_handled() {
    for source in [
        "a:\u{0}",
        "a:'x\u{7f}y'",
        "a:'\u{2028}\u{2029}'",
        "a:'\u{1f600}'",
        "a:'\\ud800'",
        "\u{feff}a:1",
        "a:'\u{202e}'",
    ] {
        let out = run(&[], source);
        assert!(
            out.code == 0 || out.code == 1,
            "{source:?} gave {}",
            out.code
        );
        if out.code == 0 {
            assert!(!out.first().is_empty(), "{source:?} printed nothing");
        }
    }
}

/// Long input stays linear. The check is deliberately loose: it is there
/// to catch quadratic behaviour, which would put four times the work into
/// the larger of these two, not to pin a speed.
#[test]
fn long_input_stays_linear() {
    let small = elapsed_ms(20_000);
    let large = elapsed_ms(80_000);
    let budget = (small.max(1.0) * 16.0).max(2_000.0);
    assert!(
        large < budget,
        "four times the input took {large}ms against {small}ms for a quarter of it"
    );
}

/// Milliseconds spent parsing and printing a flat list of `count` bytes.
fn elapsed_ms(count: usize) -> f64 {
    let source = format!("[{}]", "1,".repeat(count / 2));
    let started = Instant::now();
    let out = run(&[], &source);
    assert_eq!(out.code, 0, "{}", out.stderr);
    started.elapsed().as_secs_f64() * 1000.0
}
