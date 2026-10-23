//! The formats of the files of configuration that an external rule names: TOML, YAML and INI. Each
//! is read into a TOML table, so that a fragment and the file of a project compare the same way.

use toml::{Table, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Toml,
    Yaml,
    /// `key = value` lines under `[section]` headers, as `pytest.ini` and `setup.cfg` write them.
    /// Every value is text, and a `#` after a value is part of it, as pytest reads it.
    Ini,
    /// INI as npm reads `.npmrc`: a value that is not quoted ends before its first `#` or `;`
    /// that a `\` does not escape.
    Npmrc,
}

impl Format {
    /// The format of the file at `path`, from its name: `.toml`, `.yaml` or `.yml`, `.ini`, `.cfg`
    /// or `.npmrc`. `.cargo/config`, the name Cargo read before `.cargo/config.toml`, is TOML.
    pub fn of(path: &str) -> Option<Self> {
        if path == ".cargo/config" || path.ends_with("/.cargo/config") {
            return Some(Self::Toml);
        }
        let name = path.rsplit('/').next().unwrap_or(path);
        match name.rsplit_once('.')?.1 {
            "toml" => Some(Self::Toml),
            "yaml" | "yml" => Some(Self::Yaml),
            "ini" | "cfg" => Some(Self::Ini),
            "npmrc" => Some(Self::Npmrc),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Toml => "TOML",
            Self::Yaml => "YAML",
            Self::Ini | Self::Npmrc => "INI",
        }
    }

    /// The table that `text` holds. A YAML file must hold a mapping; its `null` values are left
    /// out, as TOML has none.
    pub fn parse(self, text: &str) -> Result<Table, String> {
        match self {
            Self::Toml => text
                .parse::<Table>()
                .map_err(|error| error.message().to_owned()),
            Self::Yaml => parse_yaml(text),
            Self::Ini => parse_ini(text, false),
            Self::Npmrc => parse_ini(text, true),
        }
    }
}

fn parse_yaml(text: &str) -> Result<Table, String> {
    let value: serde_json::Value =
        serde_saphyr::from_str(text).map_err(|error| error.to_string())?;
    match value {
        serde_json::Value::Null => Ok(Table::new()),
        serde_json::Value::Object(map) => Ok(table_of(map)),
        _ => Err("the file does not hold a mapping".to_owned()),
    }
}

fn table_of(map: serde_json::Map<String, serde_json::Value>) -> Table {
    map.into_iter()
        .filter_map(|(key, value)| Some((key, toml_of(value)?)))
        .collect()
}

fn toml_of(value: serde_json::Value) -> Option<Value> {
    Some(match value {
        serde_json::Value::Null => return None,
        serde_json::Value::Bool(flag) => Value::Boolean(flag),
        serde_json::Value::Number(number) => match number.as_i64() {
            Some(integer) => Value::Integer(integer),
            None => Value::Float(number.as_f64()?),
        },
        serde_json::Value::String(text) => Value::String(text),
        serde_json::Value::Array(items) => {
            Value::Array(items.into_iter().filter_map(toml_of).collect())
        }
        serde_json::Value::Object(map) => Value::Table(table_of(map)),
    })
}

/// A key of the root or of a `[section]`, with `=` or `:` between the key and the value. A line
/// that starts with white space continues the value above it, on a line of its own, as pytest
/// writes `testpaths` and `markers`. `#` and `;` start a comment line, and with `npm` also a
/// comment after a value.
fn parse_ini(text: &str, npm: bool) -> Result<Table, String> {
    let mut root = Table::new();
    let mut section: Option<String> = None;
    let mut last: Option<String> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(['#', ';']) {
            continue;
        }
        let table = match &section {
            Some(name) => match root.get_mut(name) {
                Some(Value::Table(table)) => table,
                _ => unreachable!("a section is added to the root before its keys"),
            },
            None => &mut root,
        };
        if line.starts_with([' ', '\t']) && continues(table, last.as_deref(), trimmed) {
            continue;
        }
        if let Some(name) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            let name = name.trim().to_owned();
            root.entry(name.clone())
                .or_insert_with(|| Value::Table(Table::new()));
            section = Some(name);
            last = None;
            continue;
        }
        let (key, value) = key_value(trimmed, npm)
            .ok_or_else(|| format!("line {}: expected `key = value`", index + 1))?;
        table.insert(key.clone(), Value::String(value));
        last = Some(key);
    }
    Ok(root)
}

/// Append an indented line to the value of `last`; false when there is no such value.
fn continues(table: &mut Table, last: Option<&str>, trimmed: &str) -> bool {
    let Some(Value::String(value)) = last.and_then(|key| table.get_mut(key)) else {
        return false;
    };
    value.push('\n');
    value.push_str(trimmed);
    true
}

/// The key and the value of a line, without the quotes around the value; with `npm`, as npm reads
/// the value.
fn key_value(trimmed: &str, npm: bool) -> Option<(String, String)> {
    // A key of `.npmrc` may hold a `:` (`//registry.npmjs.org/:_authToken`), so `=` comes first.
    let (key, value) = trimmed
        .split_once('=')
        .or_else(|| trimmed.split_once(':'))?;
    if npm {
        return Some((key.trim().to_owned(), npm_value(value)));
    }
    let value = value.trim();
    let value = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(value);
    Some((key.trim().to_owned(), value.trim().to_owned()))
}

/// A value of `.npmrc` as npm reads it: a quoted value without its quotes, or the text before the
/// first `#` or `;` that a `\` does not escape.
fn npm_value(raw: &str) -> String {
    let value = raw.trim();
    for quote in ['"', '\''] {
        if value.len() > 1 && value.starts_with(quote) && value.ends_with(quote) {
            return value[1..value.len() - 1].to_owned();
        }
    }
    let mut kept = String::new();
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(escaped @ ('\\' | ';' | '#')) => kept.push(escaped),
                Some(other) => {
                    kept.push('\\');
                    kept.push(other);
                }
                None => kept.push('\\'),
            },
            ';' | '#' => break,
            _ => kept.push(c),
        }
    }
    kept.trim().to_owned()
}
