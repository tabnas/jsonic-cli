/* Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
contributors, MIT License */

//! The command itself: argument parsing, option, meta and plugin wiring,
//! source merging and serialization. The port of `ts/src/jsonic-cli.ts`
//! and of `go/cli/run.go`.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use indexmap::IndexMap;
use serde_json::{json, Value as Json};
use tabnas::{Plugin, Tabnas, Value};

use crate::args::{engine_options, handle_props, parse_args, sub_object};
use crate::help::HELP;
use crate::merge::deep;
use crate::registry::{lookup, plugins};
use crate::stringify::{json_option, parse_replacer, parse_space, stringify};

/// Run the command over `argv`, whose first entry is the program name, as
/// [`std::env::args`] yields it. Standard input is read only when it is
/// needed, and every line the command prints goes to `stdout`; a failure
/// is reported on `stderr`. The return value is the process exit code.
///
/// The streams are parameters so the suite can drive the command in
/// process, which is the seam `run(argv, console)` gives the TypeScript
/// CLI and `Run(argv, stdin, out, extra)` gives the Go one.
///
/// ```
/// let argv: Vec<String> = ["jsonic", "a:1"].iter().map(|s| s.to_string()).collect();
/// let mut out: Vec<u8> = Vec::new();
/// let code = tabnas_jsonic_cli::run(
///     &argv,
///     &mut std::io::empty(),
///     &mut out,
///     &mut std::io::sink(),
/// );
/// assert_eq!(code, 0);
/// assert_eq!(String::from_utf8(out).unwrap(), "{\"a\":1}\n");
/// ```
pub fn run(
    argv: &[String],
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    run_with_plugins(argv, stdin, stdout, stderr, &BTreeMap::new())
}

/// [`run`], with `extra` plugins available to `-p` / `--plugin` for this
/// run alone. A same-named entry wins over the registry, which is how the
/// suite injects fixture plugins without touching the process-wide
/// registry.
pub fn run_with_plugins(
    argv: &[String],
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    extra: &BTreeMap<String, Plugin>,
) -> i32 {
    let mut registry = plugins();
    for (name, plugin) in extra {
        registry.insert(name.clone(), plugin.clone());
    }
    let mut sink = WriteSink { out: stdout };
    match run_inner(argv, stdin, &mut sink, stderr, &registry) {
        Ok(code) => code,
        Err(error) => report_io(stderr, &error),
    }
}

/// An input or output failure that reached the top: say what it was, then
/// exit nonzero.
///
/// Discarding it left a nonzero exit code with an EMPTY standard error,
/// which is the one shape this command never uses for a failure, so a
/// caller had no way to tell a failed read from a run that printed
/// nothing. The canonical command reports the same thing and in the same
/// shape: a standard input that errors rejects `run()`, and
/// `ts/bin/jsonic` prints `e.message` alone, on one line.
fn report_io(stderr: &mut dyn Write, error: &std::io::Error) -> i32 {
    // A failed write to standard output cannot be reported on a stream
    // that may be just as broken, so this last write is allowed to fail.
    let _ = writeln!(stderr, "{error}");
    1
}

/// What one captured run produced. The counterpart of the `lines` a Go
/// `logger` records and of `cn.d.log` in the TypeScript suite: one entry
/// per printed call, so an entry that spans several physical lines (the
/// help text, the grammar description, indented JSON) stays one entry.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Captured {
    /// The process exit code.
    pub code: i32,
    /// What went to standard output, one entry per printed call.
    pub lines: Vec<String>,
    /// What went to standard error.
    pub stderr: String,
}

impl Captured {
    /// The first printed entry, or the empty string when nothing was
    /// printed.
    pub fn first(&self) -> &str {
        self.lines.first().map_or("", String::as_str)
    }

    /// The last printed entry: the JSON result, after any debug trace.
    pub fn last(&self) -> &str {
        self.lines.last().map_or("", String::as_str)
    }
}

/// Run the command and collect what it printed instead of writing it to a
/// stream, so a caller can inspect the individual printed entries.
///
/// ```
/// let argv: Vec<String> = ["jsonic", "-n", "a:1"].iter().map(|s| s.to_string()).collect();
/// let out = tabnas_jsonic_cli::capture(&argv, &mut std::io::empty(), &Default::default());
/// assert_eq!(out.code, 0);
/// assert_eq!(out.lines, vec!["{\n  \"a\": 1\n}".to_string()]);
/// ```
pub fn capture(
    argv: &[String],
    stdin: &mut dyn Read,
    extra: &BTreeMap<String, Plugin>,
) -> Captured {
    let mut registry = plugins();
    for (name, plugin) in extra {
        registry.insert(name.clone(), plugin.clone());
    }
    let mut sink = CaptureSink::default();
    let mut stderr: Vec<u8> = Vec::new();
    let code = match run_inner(argv, stdin, &mut sink, &mut stderr, &registry) {
        Ok(code) => code,
        Err(error) => report_io(&mut stderr, &error),
    };
    Captured {
        code,
        lines: sink.lines,
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    }
}

/// Where the command's printed entries go. One call is one entry, which
/// is the unit the shared fixtures compare and the unit a `console.log`
/// is in the canonical command.
trait Sink {
    fn line(&mut self, text: &str) -> std::io::Result<()>;
}

/// A sink that writes each entry to a stream, newline terminated.
struct WriteSink<'a> {
    out: &'a mut dyn Write,
}

impl Sink for WriteSink<'_> {
    fn line(&mut self, text: &str) -> std::io::Result<()> {
        writeln!(self.out, "{text}")
    }
}

/// A sink that keeps each entry.
#[derive(Default)]
struct CaptureSink {
    lines: Vec<String>,
}

impl Sink for CaptureSink {
    fn line(&mut self, text: &str) -> std::io::Result<()> {
        self.lines.push(text.to_string());
        Ok(())
    }
}

fn run_inner(
    argv: &[String],
    stdin: &mut dyn Read,
    log: &mut dyn Sink,
    stderr: &mut dyn Write,
    registry: &BTreeMap<String, Plugin>,
) -> std::io::Result<i32> {
    let args = parse_args(argv);

    if args.help {
        log.line(HELP)?;
        return Ok(0);
    }

    let option_bag = match handle_props(&args.options) {
        Ok(bag) => bag,
        Err(message) => return fail(stderr, &message),
    };
    let meta_bag = match handle_props(&args.meta) {
        Ok(bag) => bag,
        Err(message) => return fail(stderr, &message),
    };

    // Resolve every `-p` reference before building anything, so an
    // unknown one fails without side effects. `-d` adds the debug plugin
    // ahead of them, as the TypeScript CLI seeds `plugins.debug` before
    // merging `handle_plugins`.
    //
    // The collection is KEYED BY REFERENCE, which is what makes a repeated
    // `-p` reference install once. TypeScript keeps its plugins in an
    // object (`out[name] = require(name)`), so `-p x -p x` leaves one
    // entry and `jsonic.use` runs once; a list would install the plugin
    // twice, which duplicates whatever it registers. Measured under Node
    // against `ts/src/jsonic-cli.ts`: a counting plugin passed twice
    // reports one install, and `-p a -p b -p a` installs `a` before `b`,
    // so the position kept is the FIRST occurrence's.
    //
    // The first occurrence's PLUGIN is kept with it. TypeScript's later
    // assignment overwrites the value, but the module cache answers the
    // same reference with the same module, so which one survives is not
    // observable there. It is observable here for `-d -p debug`, where
    // `-d`'s tracing plugin would be replaced by the registry's quiet
    // one, and the canonical command still traces (`-d` also pushes
    // `log=-1`, which no `-p` undoes).
    let mut wanted: IndexMap<String, Plugin> = IndexMap::new();
    if args.debug {
        wanted.insert("debug".to_string(), debug_plugin());
    }
    for name in &args.plugins {
        let Some(plugin) = lookup(registry, name) else {
            return fail(stderr, &format!("Plugin not found: {name}"));
        };
        wanted.entry(name.clone()).or_insert_with(|| plugin.clone());
    }

    let mut parser = match build_parser(&engine_options(&option_bag)) {
        Ok(parser) => parser,
        Err(message) => return fail(stderr, &message),
    };

    // Trace and configuration output goes through the engine's debug
    // sink, the Rust counterpart of the TypeScript CLI's
    // `options.debug.get_console = () => console`. It is collected rather
    // than written straight out, because the sink is a shared callback
    // and cannot hold the caller's `&mut` stream; flushing it after each
    // parse keeps the order the canonical command prints in.
    let traced: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&traced);
    parser.options.debug.output = Some(Arc::new(move |line: &str| {
        if let Ok(mut lines) = sink.lock() {
            lines.push(line.to_string());
        }
    }));

    // The plugin's options are NOT passed at the call site. The engine
    // merges `options.plugin[<plugin name>]` into the bag a plugin is
    // installed with, and the `-o plugin.<name>...` settings are already
    // in the option document, so a plugin receives them under its own
    // name. That is the TypeScript behaviour: `jsonic.use` keys the
    // option sub-bag by the plugin function's name, which is how
    // `-p ../test/pa-qa.js -o plugin.paqa.q=3` finds its option.
    //
    // The describe header is keyed by the REFERENCE, not by the plugin's
    // name, because that is what TypeScript keys it by: `handle_plugins`
    // stores each loaded plugin under the `-p` string it was named with,
    // and `null != plugins.debug` is then true for `-p debug` and false
    // for `-p @tabnas/debug`. Go keys it the same way.
    let mut has_debug = false;
    for (name, plugin) in wanted {
        has_debug |= name == "debug";
        if let Err(error) = parser.use_plugin(plugin, None) {
            return fail(stderr, &error.0);
        }
    }

    if has_debug {
        log.line(&format!(
            "{}\n=== PARSE ===",
            tabnas_debug::describe(&parser)
        ))?;
    }

    let meta = Value::from_json(&meta_bag);

    // Sources merge last-wins by precedence: `--file` results first, then
    // standard input, then the positional sources. Each is deep-merged
    // into the seed, and a source that parses to nothing leaves the seed
    // alone, which is the TypeScript `util.deep(data, {val: undefined})`
    // no-op.
    let mut data = Value::Null;

    for path in &args.files {
        // An empty `--file` is skipped rather than read.
        if path.is_empty() {
            continue;
        }
        let source = match std::fs::read(path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(error) => return fail(stderr, &format!("{path}: {error}")),
        };
        match parse_source(&parser, &source, &meta, &traced, log)? {
            Ok(value) => data = deep(data, value),
            Err(message) => return fail(stderr, &message),
        }
    }

    // Standard input is read whenever there is no positional source, and
    // whenever `-` was given, exactly as the canonical command does.
    if args.sources.is_empty() || args.stdin {
        // A pipe or device that fails mid-read is reported, not
        // discarded: the exit code alone, with nothing on standard error,
        // leaves the caller unable to tell a failed read from a source
        // that parsed to nothing. The canonical command prints the
        // error's message and nothing else.
        let source = match read_stdin(stdin) {
            Ok(text) => text,
            Err(error) => return fail(stderr, &error.to_string()),
        };
        match parse_source(&parser, &source, &meta, &traced, log)? {
            Ok(value) => data = deep(data, value),
            Err(message) => return fail(stderr, &message),
        }
    }

    for source in &args.sources {
        match parse_source(&parser, source, &meta, &traced, log)? {
            Ok(value) => data = deep(data, value),
            Err(message) => return fail(stderr, &message),
        }
    }

    // `replacer = Jsonic(options.JSON.replacer)` and
    // `space = Jsonic(options.JSON.space)`, run here rather than earlier
    // because that is where the canonical command runs them: a string
    // that does not parse fails AFTER the sources have been parsed and
    // any debug trace printed.
    let json_bag = sub_object(&option_bag, "JSON");
    let replacer = match json_option(json_bag, "replacer") {
        Ok(value) => parse_replacer(value.as_ref()),
        Err(message) => return fail(stderr, &message),
    };
    let space = match json_option(json_bag, "space") {
        Ok(value) => parse_space(value.as_ref()),
        Err(message) => return fail(stderr, &message),
    };

    log.line(&stringify(&data, replacer.as_deref(), &space))?;
    Ok(0)
}

/// Parse one source, flushing whatever the trace wrote while it ran.
fn parse_source(
    parser: &Tabnas,
    source: &str,
    meta: &Value,
    traced: &Arc<Mutex<Vec<String>>>,
    log: &mut dyn Sink,
) -> std::io::Result<Result<Value, String>> {
    let outcome = parser.parse_with_meta(source, meta.clone());
    if let Ok(mut lines) = traced.lock() {
        for line in lines.drain(..) {
            log.line(&line)?;
        }
    }
    Ok(outcome.map_err(|error| error.to_string()))
}

/// Report `message` on `stderr` and exit nonzero.
fn fail(stderr: &mut dyn Write, message: &str) -> std::io::Result<i32> {
    writeln!(stderr, "{message}")?;
    Ok(1)
}

/// Build the parser for `engine_options`, a `serde_json` object of the
/// `-o` settings that are not CLI-only.
///
/// The order matters and is the canonical one: jsonic's own option
/// defaults, then the caller's options, and only then the grammar, which
/// is built against the finished option set. That is what makes an option
/// deciding which alternates exist (`map.child`, `list.child`,
/// `list.pair`) take effect, as `Jsonic.make(options)` does in
/// TypeScript. The options are resolved on a throwaway instance first
/// because the engine applies an option DOCUMENT to a parser, while
/// [`tabnas_jsonic::make_with`] wants the finished typed options.
fn build_parser(engine_options: &Json) -> Result<Tabnas, String> {
    let empty = engine_options
        .as_object()
        .is_none_or(serde_json::Map::is_empty);
    if empty {
        return Ok(tabnas_jsonic::make());
    }

    let document = json!({ "options": engine_options });
    let mut probe = tabnas_jsonic::make();
    probe
        .grammar_json(&document.to_string())
        .map_err(|error| error.0)?;
    let resolved = probe.config();
    Ok(tabnas_jsonic::make_with(move |options| *options = resolved))
}

/// The debug plugin `-d` installs: tracing on, and no `USE:` dump, since
/// the command prints the description itself.
///
/// Tracing is unconditional, as it is in TypeScript, where `-d` pushes
/// `log=-1` onto the metadata bag and the engine log is what traces.
/// Nothing a `-o` can say turns that off there, so nothing does here. A
/// `-p debug` with no `-d` gets the registry's entry instead, which is
/// quiet, and that is the TypeScript behaviour too: without `log=-1` the
/// plugin describes the grammar and traces nothing.
fn debug_plugin() -> Plugin {
    tabnas_debug::plugin(
        tabnas_debug::DebugOptions::quiet().with_trace(tabnas_debug::TraceKinds::all()),
    )
}

/// Read standard input as text.
///
/// The bytes are untrusted, so an invalid sequence folds to U+FFFD rather
/// than failing: the canonical command reads the stream as UTF-8 and the
/// engine replaces what it cannot decode.
fn read_stdin(stdin: &mut dyn Read) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    stdin.read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
