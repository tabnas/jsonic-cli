/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! Shared test scaffolding.
//!
//! Cargo compiles this module into EVERY integration test binary, so a
//! helper only one of them uses reads as dead code in the others. The
//! allow is about that compilation model, not about unused code.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tabnas::{Plugin, Value, ValueDef};
use tabnas_jsonic_cli::Captured;

/// The repository root: `rs/`'s parent.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rs/ has a parent")
        .to_path_buf()
}

/// `argv` as the command sees it, program name first.
pub fn argv(list: &[&str]) -> Vec<String> {
    std::iter::once("jsonic")
        .chain(list.iter().copied())
        .map(String::from)
        .collect()
}

/// Drive the command in process, with no extra plugins.
///
/// The entries the run printed are compared, not the physical lines:
/// one entry is one `console.log` in the canonical command, which is the
/// unit the shared fixtures pin and what `cn.d.log[0][0]` reads in the
/// TypeScript suite.
pub fn run(list: &[&str], stdin: &str) -> Captured {
    run_with(list, stdin, &BTreeMap::new())
}

/// Drive the command in process with `extra` plugins resolvable by `-p`.
pub fn run_with(list: &[&str], stdin: &str, extra: &BTreeMap<String, Plugin>) -> Captured {
    tabnas_jsonic_cli::capture(&argv(list), &mut stdin.as_bytes(), extra)
}

/// Drive the command through the streaming entry point, which is what the
/// binary calls, and return the exit code with the raw streams.
pub fn run_streams(list: &[&str], stdin: &str) -> (i32, String, String) {
    let mut input = stdin.as_bytes();
    let mut stdout: Vec<u8> = Vec::new();
    let mut stderr: Vec<u8> = Vec::new();
    let code = tabnas_jsonic_cli::run(&argv(list), &mut input, &mut stdout, &mut stderr);
    (
        code,
        String::from_utf8(stdout).expect("utf-8 stdout"),
        String::from_utf8(stderr).expect("utf-8 stderr"),
    )
}

/// A plugin that registers one value definition whose key defaults to
/// `def_key` (overridable with the plugin's `s` option) and whose value is
/// the plugin's `val_opt` option, under the plugin name `name`.
///
/// The NAME matters: the engine merges `options.plugin[<plugin name>]`
/// into the bag a plugin is installed with, so `-o plugin.p0.x=0`
/// reaches the plugin called `p0` whatever `-p` reference named it.
///
/// This is the Rust form of the `ts/test/p0.js`, `p1.js`, `p2.js` and
/// `pa-qa.js` fixtures, which differ only in default key and option name.
/// The four JavaScript files exist to cover the four module export shapes
/// `handle_plugins` accepts; Rust has no module loading and so no export
/// shapes, and what is left to test is the option plumbing, which is what
/// these carry. Go's `valueDefPlugin` makes the same adaptation.
pub fn value_def_plugin(name: &str, def_key: &str, val_opt: &str) -> Plugin {
    let def_key = def_key.to_string();
    let val_opt = val_opt.to_string();
    Plugin::new(name, move |parser, options| {
        let key = match option(options, "s") {
            Some(Value::String(text)) if !text.is_empty() => text.clone(),
            _ => def_key.clone(),
        };
        let val = option(options, &val_opt).cloned();
        parser.set_options(|engine| {
            engine.value.definitions.insert(
                key,
                ValueDef {
                    val,
                    matcher: None,
                    transform: None,
                    consume: false,
                },
            );
        })?;
        Ok(())
    })
}

/// One entry of a plugin's option bag.
fn option<'a>(options: &'a Value, name: &str) -> Option<&'a Value> {
    match options {
        Value::Object(entries) => entries.get(name),
        Value::MapRef(map) => map.value.get(name),
        _ => None,
    }
}

/// The four fixture plugins, under the reference names the TypeScript
/// tests use.
pub fn test_plugins() -> BTreeMap<String, Plugin> {
    let mut out = BTreeMap::new();
    out.insert("p0".to_string(), value_def_plugin("p0", "X", "x"));
    out.insert("p1".to_string(), value_def_plugin("p1", "Y", "y"));
    out.insert("p2".to_string(), value_def_plugin("p2", "Z", "z"));
    out.insert("paqa".to_string(), value_def_plugin("paqa", "Q", "q"));
    out
}
