/* Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
contributors, MIT License */

//! The `jsonic` command as a library: read relaxed JSON from arguments,
//! files or standard input, parse it with
//! [`tabnas-jsonic`](https://github.com/tabnas/jsonic), merge the results
//! and print standard JSON.
//!
//! This is the Rust port of the canonical TypeScript implementation in
//! `ts/src/jsonic-cli.ts`; the TypeScript version is authoritative and
//! this crate tracks it. The Go port is in `go/`.
//!
//! The crate is a library plus a thin `src/bin/jsonic.rs` launcher, so
//! the command can be driven in process:
//!
//! ```
//! let argv: Vec<String> = ["jsonic", "a:b:1", "a:c:2"]
//!     .iter()
//!     .map(|s| s.to_string())
//!     .collect();
//! let mut out: Vec<u8> = Vec::new();
//! let code = tabnas_jsonic_cli::run(
//!     &argv,
//!     &mut std::io::empty(),
//!     &mut out,
//!     &mut std::io::sink(),
//! );
//! assert_eq!(code, 0);
//! assert_eq!(String::from_utf8(out).unwrap(), "{\"a\":{\"b\":1,\"c\":2}}\n");
//! ```

mod args;
pub mod cli;
mod help;
mod merge;
pub mod registry;
mod stringify;

pub use cli::{capture, run, run_with_plugins, Captured};
pub use registry::{plugins, register_plugin};

/// This crate's version. It MUST equal `ts/package.json` "version" and
/// the `version` in `Cargo.toml`: the release orchestrator rewrites all
/// of them, and `tests/version_test.rs` fails the build if they drift.
/// Mirrors `VERSION` in `ts/src/jsonic-cli.ts` and `const VERSION` in
/// `go/cmd/jsonic/main.go`.
pub const VERSION: &str = "0.5.7";

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_examples {}
