/* Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
contributors, MIT License */

//! Argument parsing and the dotted-path property bags behind `-o` and
//! `-m`. The port of `go/cli/args.go` and of the argument loop at the top
//! of `ts/src/jsonic-cli.ts`.

use serde_json::{Map, Value as Json};

/// The parsed command line, matching the TypeScript `args` object.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct CliArgs {
    pub help: bool,
    pub stdin: bool,
    pub debug: bool,
    pub sources: Vec<String>,
    pub files: Vec<String>,
    pub options: Vec<String>,
    pub meta: Vec<String>,
    pub plugins: Vec<String>,
}

/// Parse `argv` (the program name first, as `std::env::args` yields it).
///
/// This replicates the TypeScript loop exactly. A bare `-` means standard
/// input, `--` stops flag parsing, the value-taking flags consume the
/// following argument, and every other token, `-` prefixed or not, becomes
/// a source. Only the exact spellings are flags: `--option=JSON.space=2`
/// is not `--option`, so it falls through to the sources and is parsed as
/// relaxed JSON, which is what the canonical command does with it.
///
/// A value-taking flag in the LAST position consumes nothing rather than
/// taking an absent value, mirroring the TypeScript `aI + 1 < argv.length`
/// guard and Go's `i+1 < len(argv)`.
pub(crate) fn parse_args(argv: &[String]) -> CliArgs {
    let mut args = CliArgs::default();
    let mut accept_args = true;
    let mut i = 1;

    while i < argv.len() {
        let arg = argv[i].as_str();

        // The value of a value-taking flag, when one follows it.
        let value = |i: &mut usize| -> Option<String> {
            if *i + 1 < argv.len() {
                *i += 1;
                Some(argv[*i].clone())
            } else {
                None
            }
        };

        if accept_args && arg.starts_with('-') {
            match arg {
                "-" => args.stdin = true,
                "--" => accept_args = false,
                "--file" | "-f" => args.files.extend(value(&mut i)),
                "--option" | "-o" => args.options.extend(value(&mut i)),
                "--meta" | "-m" => args.meta.extend(value(&mut i)),
                "--debug" | "-d" => {
                    args.debug = true;
                    args.meta.push("log=-1".to_string());
                }
                "--help" | "-h" => args.help = true,
                "--plugin" | "-p" => args.plugins.extend(value(&mut i)),
                "--nice" | "-n" => args.options.push("JSON.space=2".to_string()),
                _ => args.sources.push(arg.to_string()),
            }
        } else {
            args.sources.push(arg.to_string());
        }

        i += 1;
    }

    args
}

/// Split a `name=value` argument the way the TypeScript
/// `propval.split(/=/)` does: `pv[0]` is the text before the first `=`,
/// and `pv[1]` the text between the first and the second. Everything
/// after that is discarded. `None` when there is no `=` at all, which
/// leaves `pv[1]` undefined and the property unset.
fn split_propval(propval: &str) -> Option<(&str, &str)> {
    let (name, rest) = propval.split_once('=')?;
    let value = rest.split_once('=').map_or(rest, |(head, _)| head);
    Some((name, value))
}

/// Build a dotted-path property bag from `name=value` arguments, with each
/// leaf value parsed by vanilla jsonic. The port of `handle_props`.
///
/// An entry with an empty name or an empty value is a no-op, which is what
/// makes `-o ""`, `-o "="` and `-o "bad="` harmless. A leaf value that
/// does not parse is an error, as it is in TypeScript, where `Jsonic(...)`
/// throws out of `run()`.
pub(crate) fn handle_props(propvals: &[String]) -> Result<Json, String> {
    let mut out = Map::new();

    for propval in propvals {
        let Some((name, value)) = split_propval(propval) else {
            continue;
        };
        if name.is_empty() || value.is_empty() {
            continue;
        }
        let parsed = tabnas_jsonic::parse(value).map_err(|error| error.to_string())?;
        set_prop(&mut out, name, to_json(&parsed));
    }

    Ok(Json::Object(out))
}

/// The engine's value as `serde_json`, with whole numbers narrowed to
/// integers.
///
/// [`tabnas::Value::to_json`] keeps every number a float, because that is
/// what the engine holds. An option document reader that wants a count
/// asks for an integer (`options.debug.maxlen` does), and a float-backed
/// `serde_json::Number` answers `as_u64` with `None` however round it is,
/// so `-o debug.maxlen=11` would be rejected as "not a non-negative
/// integer". Narrowing here is what makes the numeric options settable.
pub(crate) fn to_json(value: &tabnas::Value) -> Json {
    match value {
        tabnas::Value::Number(number) => number_to_json(*number),
        tabnas::Value::Array(items) => Json::Array(items.iter().map(to_json).collect()),
        tabnas::Value::ListRef(list) => Json::Array(list.value.iter().map(to_json).collect()),
        tabnas::Value::Object(entries) => Json::Object(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), to_json(value)))
                .collect(),
        ),
        tabnas::Value::MapRef(map) => Json::Object(
            map.value
                .iter()
                .map(|(key, value)| (key.clone(), to_json(value)))
                .collect(),
        ),
        other => other.to_json(),
    }
}

/// A finite whole number as an integer, anything else as it stands.
fn number_to_json(number: f64) -> Json {
    if number.fract() == 0.0 && number.is_finite() {
        if number >= 0.0 && number <= u64::MAX as f64 {
            return Json::Number((number as u64).into());
        }
        if number >= i64::MIN as f64 && number < 0.0 {
            return Json::Number((number as i64).into());
        }
    }
    serde_json::Number::from_f64(number).map_or(Json::Null, Json::Number)
}

/// Set a dotted path in a nested object bag, building the intermediate
/// objects. The port of the engine's `util.prop`: an intermediate that is
/// not an object is replaced by one.
fn set_prop(bag: &mut Map<String, Json>, path: &str, value: Json) {
    let parts: Vec<&str> = path.split('.').collect();
    let (last, parents) = parts
        .split_last()
        .expect("split never yields an empty list");

    let mut cursor = bag;
    for part in parents {
        let next = cursor
            .entry(part.to_string())
            .or_insert_with(|| Json::Object(Map::new()));
        if !next.is_object() {
            *next = Json::Object(Map::new());
        }
        cursor = next.as_object_mut().expect("just made it an object");
    }
    cursor.insert(last.to_string(), value);
}

/// The sub-object at `key`, or `None` when the bag has no object there.
pub(crate) fn sub_object<'a>(bag: &'a Json, key: &str) -> Option<&'a Map<String, Json>> {
    bag.get(key).and_then(Json::as_object)
}

/// The bag with the CLI-only `JSON` key removed: what is left configures
/// the engine.
///
/// `plugin` STAYS. It is an engine option, and leaving it in the document
/// is what routes `-o plugin.<name>.<option>=<value>` to the plugin,
/// exactly as it does in TypeScript: the engine merges
/// `options.plugin[<plugin name>]` into the bag a plugin is installed
/// with, keyed by the PLUGIN'S OWN name rather than by the `-p`
/// reference. That is why `-p ../test/pa-qa.js -o plugin.paqa.q=3`
/// reaches the plugin at all.
pub(crate) fn engine_options(bag: &Json) -> Json {
    let mut out = Map::new();
    if let Some(map) = bag.as_object() {
        for (key, value) in map {
            if key == "JSON" {
                continue;
            }
            out.insert(key.clone(), value.clone());
        }
    }
    Json::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(list: &[&str]) -> Vec<String> {
        std::iter::once("jsonic")
            .chain(list.iter().copied())
            .map(String::from)
            .collect()
    }

    #[test]
    fn a_trailing_value_flag_consumes_nothing() {
        for flag in ["-o", "-m", "-f", "-p"] {
            let args = parse_args(&argv(&["a:1", flag]));
            assert_eq!(args.sources, vec!["a:1".to_string()]);
            assert!(args.options.is_empty());
            assert!(args.files.is_empty());
            assert!(args.plugins.is_empty());
        }
    }

    #[test]
    fn end_of_flags_turns_the_rest_into_sources() {
        let args = parse_args(&argv(&["--", "-h", "-n"]));
        assert!(!args.help);
        assert_eq!(args.sources, vec!["-h".to_string(), "-n".to_string()]);
    }

    #[test]
    fn an_equals_form_is_not_a_flag() {
        let args = parse_args(&argv(&["--option=JSON.space=2"]));
        assert!(args.options.is_empty());
        assert_eq!(args.sources, vec!["--option=JSON.space=2".to_string()]);
    }

    #[test]
    fn a_propval_reads_only_the_first_two_segments() {
        assert_eq!(split_propval("a=b=c"), Some(("a", "b")));
        assert_eq!(split_propval("a=b"), Some(("a", "b")));
        assert_eq!(split_propval("a="), Some(("a", "")));
        assert_eq!(split_propval("a"), None);
    }

    #[test]
    fn a_dotted_path_builds_intermediate_objects() {
        let bag = handle_props(&["a.b.c=1".to_string()]).expect("parses");
        assert_eq!(bag.to_string(), r#"{"a":{"b":{"c":1}}}"#);
    }

    #[test]
    fn a_whole_option_value_narrows_to_an_integer() {
        let bag = handle_props(&["debug.maxlen=11".to_string()]).expect("parses");
        assert_eq!(bag["debug"]["maxlen"].as_u64(), Some(11));
        let bag = handle_props(&["a=1.5".to_string()]).expect("parses");
        assert_eq!(bag["a"].as_f64(), Some(1.5));
    }
}
