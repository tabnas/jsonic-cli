/* Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
contributors, MIT License */

//! The `jsonic` command. The implementation is `tabnas_jsonic_cli::run`,
//! which takes its streams as arguments so the tests can drive it in
//! process; this is the launcher, the port of `ts/bin/jsonic` and
//! `go/cmd/jsonic/main.go`.

use std::io::IsTerminal;

fn main() {
    let argv: Vec<String> = std::env::args().collect();

    // Nothing is piped in at a terminal, and the command must not sit
    // waiting for a keyboard: `process.stdin.isTTY` makes the TypeScript
    // `read_stdin` answer with the empty string, and Go's `ReadStdin`
    // checks the same thing with `os.ModeCharDevice`.
    let mut stdin = std::io::stdin();
    let mut nothing = std::io::empty();
    let input: &mut dyn std::io::Read = if stdin.is_terminal() {
        &mut nothing
    } else {
        &mut stdin
    };

    let code = tabnas_jsonic_cli::run(
        &argv,
        input,
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    );
    std::process::exit(code);
}
