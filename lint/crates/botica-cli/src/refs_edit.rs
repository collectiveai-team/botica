//! Edits of a `topic.toml` that keep the rest of the file as it is.

/// `text` as a TOML string.
pub fn quoted(text: &str) -> String {
    toml::Value::String(text.to_owned()).to_string()
}

/// `toml` with the `rev` and, when given, the `lines` of the `index`th `[[references]]` table
/// replaced. It fails when the file does not have the table written that way: the caller leaves
/// the file as it is.
pub fn set_fields(
    toml: &str,
    index: usize,
    rev: &str,
    lines: Option<&str>,
) -> Result<String, String> {
    let mut out = String::new();
    let mut table = 0;
    let mut inside = false;
    for line in toml.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == "[[references]]" && table == index;
            table += usize::from(trimmed == "[[references]]");
        } else if inside {
            if let Some(replacement) = replacement(line, rev, lines) {
                out.push_str(&replacement);
                continue;
            }
        }
        out.push_str(line);
    }
    if holds(&out, index, rev, lines) {
        Ok(out)
    } else {
        Err(format!(
            "cannot rewrite references[{index}]: change its `rev` and `lines` by hand"
        ))
    }
}

fn replacement(line: &str, rev: &str, lines: Option<&str>) -> Option<String> {
    let (key, _) = line.split_once('=')?;
    match key.trim() {
        "rev" => Some(format!("rev = {}\n", quoted(rev))),
        "lines" => lines.map(|lines| format!("lines = {}\n", quoted(lines))),
        _ => None,
    }
}

fn holds(toml: &str, index: usize, rev: &str, lines: Option<&str>) -> bool {
    let Ok(parsed) = toml::from_str::<toml::Value>(toml) else {
        return false;
    };
    let Some(table) = parsed.get("references").and_then(|all| all.get(index)) else {
        return false;
    };
    let field = |name: &str| table.get(name).and_then(toml::Value::as_str);
    field("rev") == Some(rev) && lines.is_none_or(|lines| field("lines") == Some(lines))
}
