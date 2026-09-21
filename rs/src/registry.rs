/* Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
contributors, MIT License */

//! The compiled-in plugin registry behind `-p` / `--plugin`. The port of
//! `go/cli/registry.go`.
//!
//! The TypeScript CLI `require`s a plugin module at run time. Rust, like
//! Go, has no way to load one by name, so a reference resolves against
//! this registry instead. The plugins already in the crate's dependency
//! graph are registered at first use, and a custom binary adds its own
//! with [`register_plugin`] before calling
//! [`run_with_plugins`](crate::run_with_plugins).

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use tabnas::{Plugin, PluginError};

/// The plugins a stock binary resolves, plus whatever a caller has added.
fn registry() -> &'static Mutex<BTreeMap<String, Plugin>> {
    static REGISTRY: OnceLock<Mutex<BTreeMap<String, Plugin>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(builtins()))
}

/// The plugins already present in this crate's dependency graph, so a
/// stock binary supports them out of the box:
///
/// - `debug`, tracing and grammar description from `tabnas-debug`;
/// - `jsonic`, the relaxed-JSON grammar itself, a no-op on the CLI's own
///   instance, which already carries it;
/// - `json`, which restricts the parser to standard JSON.
///
/// The other tabnas grammars (csv, toml, directive, multisource and the
/// rest) live in crates that depend on jsonic. Requiring them here would
/// pull the whole family into every build of the command, so they are
/// left to custom binaries.
fn builtins() -> BTreeMap<String, Plugin> {
    let mut out = BTreeMap::new();
    out.insert(
        "debug".to_string(),
        tabnas_debug::plugin(tabnas_debug::DebugOptions::quiet()),
    );
    out.insert("jsonic".to_string(), tabnas_jsonic::plugin());
    out.insert(
        "json".to_string(),
        Plugin::new("json", |parser, _options| {
            tabnas_json::json(parser).map_err(|error| PluginError(error.0))
        }),
    );
    out
}

/// Make `plugin` available to `-p` / `--plugin` under `name`.
///
/// Register the plain name (`csv`); the lookup also accepts the
/// `@tabnas/csv` form and a path-like reference ending in that name.
/// Registering a name again replaces it. Call this before
/// [`run`](crate::run), typically from a custom binary's `main`:
///
/// ```
/// # fn nothing(parser: &mut tabnas::Tabnas, _options: &tabnas::Value)
/// #     -> Result<(), tabnas::PluginError> { let _ = parser; Ok(()) }
/// tabnas_jsonic_cli::register_plugin("noop", tabnas::Plugin::new("noop", nothing));
/// ```
pub fn register_plugin(name: impl Into<String>, plugin: Plugin) {
    let mut registry = registry()
        .lock()
        .expect("the plugin registry is not poisoned");
    registry.insert(name.into(), plugin);
}

/// A snapshot of every registered plugin, keyed by reference name.
pub fn plugins() -> BTreeMap<String, Plugin> {
    registry()
        .lock()
        .expect("the plugin registry is not poisoned")
        .clone()
}

/// Resolve a `-p` reference against `registry`, with the fallbacks that
/// stand in for the TypeScript `require(name)` then
/// `require('@tabnas/' + name)` chain: the bare name, the base name of a
/// path-like reference with any `.js` suffix removed, and the tail of an
/// `@tabnas/<name>` reference.
pub(crate) fn lookup<'a>(registry: &'a BTreeMap<String, Plugin>, name: &str) -> Option<&'a Plugin> {
    if let Some(plugin) = registry.get(name) {
        return Some(plugin);
    }
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let base = base.strip_suffix(".js").unwrap_or(base);
    if let Some(plugin) = registry.get(base) {
        return Some(plugin);
    }
    name.strip_prefix("@tabnas/")
        .and_then(|tail| registry.get(tail))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stock_binary_carries_three_plugins() {
        let registry = builtins();
        for name in ["debug", "jsonic", "json"] {
            assert!(registry.contains_key(name), "missing {name}");
        }
    }

    #[test]
    fn a_reference_resolves_through_the_require_fallbacks() {
        let registry = builtins();
        assert!(lookup(&registry, "json").is_some());
        assert!(lookup(&registry, "@tabnas/json").is_some());
        assert!(lookup(&registry, "../test/json.js").is_some());
        assert!(lookup(&registry, "no-such-plugin").is_none());
    }
}
