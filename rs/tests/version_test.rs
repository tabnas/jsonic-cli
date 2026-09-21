/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! The Rust crate's version is one of the release sites that must agree.
//! A bump that updates `ts/package.json` and forgets these fails here
//! rather than shipping a crate whose version disagrees with the package
//! it is a port of. Mirrors `ts/test/version.test.js` and
//! `go/cmd/jsonic/version_test.go`.
//!
//! The constant HAS drifted in practice: this package shipped 0.4.1 and
//! 0.4.2 while the Go const sat at 0.4.0, and nothing noticed until
//! someone read the file.

use std::fs;
use std::path::Path;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rs/ has a parent")
}

#[test]
fn version_looks_like_a_semver() {
    let parts: Vec<&str> = tabnas_jsonic_cli::VERSION.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "VERSION is not x.y.z: {}",
        tabnas_jsonic_cli::VERSION
    );
    for part in parts {
        assert!(
            !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()),
            "VERSION segment is not numeric: {}",
            tabnas_jsonic_cli::VERSION
        );
    }
}

#[test]
fn version_matches_cargo_toml() {
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("the manifest is readable");
    let declared = manifest
        .lines()
        .find_map(|line| line.strip_prefix("version = \""))
        .and_then(|rest| rest.split('"').next())
        .expect("the manifest declares a version");
    assert_eq!(
        declared,
        tabnas_jsonic_cli::VERSION,
        "Cargo.toml disagrees with VERSION"
    );
}

#[test]
fn version_matches_package_json() {
    let package = fs::read_to_string(repo_root().join("ts").join("package.json"))
        .expect("ts/package.json is readable");
    let declared: serde_json::Value =
        serde_json::from_str(&package).expect("ts/package.json is JSON");
    assert_eq!(
        declared["version"]
            .as_str()
            .expect("package.json has a version"),
        tabnas_jsonic_cli::VERSION,
        "ts/package.json disagrees with VERSION"
    );
}

/// The lockfile records this crate at the manifest's version. `ci/rust/run.sh`
/// asserts the same thing before it runs cargo, because cargo silently
/// fixes the lock up afterwards; having it here too means a plain
/// `cargo test` catches the stale lock a version bump leaves behind.
#[test]
fn version_matches_cargo_lock() {
    let lock = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock"))
        .expect("Cargo.lock is committed and readable");
    let mut lines = lock.lines();
    let declared = loop {
        let Some(line) = lines.next() else {
            panic!("Cargo.lock has no entry for tabnas-jsonic-cli");
        };
        if line.trim() == "name = \"tabnas-jsonic-cli\"" {
            break lines
                .find_map(|line| line.trim().strip_prefix("version = \""))
                .and_then(|rest| rest.split('"').next())
                .expect("the lock entry states a version")
                .to_string();
        }
    };
    assert_eq!(
        declared,
        tabnas_jsonic_cli::VERSION,
        "rs/Cargo.lock disagrees with VERSION"
    );
}
