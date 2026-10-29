//! The version of `botica` that a Python extension script runs with: a script that declares
//! `botica` with no version gets the version of the running botica through `uv run --with`.

/// The version of the running botica, which is the version of its wheel on PyPI.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The requirement that `uv run --with` adds for the script whose text is `text`:
/// `botica[sdk]==<version of botica>`, with the extras that the script names. `None` when the
/// script declares no `botica`, when it gives `botica` a version, a URL or a marker of its own, or
/// when `[tool.uv.sources]` of its metadata names `botica`: that script chose its version.
pub fn sdk_requirement(text: &str) -> Option<String> {
    let metadata: toml::Value = toml::from_str(&script_metadata(text)?).ok()?;
    let sourced = metadata
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .and_then(|uv| uv.get("sources"))
        .and_then(toml::Value::as_table)
        .is_some_and(|sources| sources.keys().any(|name| is_botica(name)));
    if sourced {
        return None;
    }
    metadata
        .get("dependencies")?
        .as_array()?
        .iter()
        .filter_map(toml::Value::as_str)
        .find_map(unversioned_botica)
        .map(|named| format!("{named}=={VERSION}"))
}

/// The TOML of the block `# /// script` of a script (PEP 723), without the `#` of its lines.
fn script_metadata(text: &str) -> Option<String> {
    let mut lines = text
        .lines()
        .skip_while(|line| line.trim_end() != "# /// script");
    lines.next()?;
    let mut toml = String::new();
    for line in lines {
        let line = line.trim_end_matches('\r');
        if line == "# ///" {
            return Some(toml);
        }
        toml.push_str(line.strip_prefix("# ").or_else(|| line.strip_prefix('#'))?);
        toml.push('\n');
    }
    None
}

/// `botica` or `botica[sdk]`, as the requirement writes it, when the requirement names `botica`
/// and nothing after it.
fn unversioned_botica(requirement: &str) -> Option<&str> {
    let requirement = requirement.trim();
    let name_end = requirement
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        .unwrap_or(requirement.len());
    if !is_botica(&requirement[..name_end]) {
        return None;
    }
    let rest = requirement[name_end..].trim_start();
    let named_end = if rest.starts_with('[') {
        requirement.len() - rest.len() + rest.find(']')? + 1
    } else {
        name_end
    };
    requirement[named_end..]
        .trim()
        .is_empty()
        .then(|| &requirement[..named_end])
}

/// Whether a name of a package is `botica` once normalized (PEP 503).
fn is_botica(name: &str) -> bool {
    name.eq_ignore_ascii_case("botica")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(dependencies: &str, tool: &str) -> String {
        format!(
            "#!/usr/bin/env -S uv run --script\n# /// script\n# requires-python = \">=3.10\"\n\
             # dependencies = [{dependencies}]\n{tool}# ///\nprint(1)\n"
        )
    }

    #[test]
    fn a_script_that_names_botica_with_no_version_gets_the_version_of_botica() {
        for (dependencies, expected) in [
            ("\"botica[sdk]\"", "botica[sdk]"),
            ("\"ruff\", \"botica\"", "botica"),
            ("\"Botica [sdk]\"", "Botica [sdk]"),
        ] {
            assert_eq!(
                sdk_requirement(&script(dependencies, "")),
                Some(format!("{expected}=={VERSION}")),
                "{dependencies}"
            );
        }
    }

    #[test]
    fn a_script_that_chose_its_version_of_botica_keeps_it() {
        for dependencies in [
            "\"botica[sdk]==0.17.1\"",
            "\"botica>=0.17\"",
            "\"botica[sdk] @ file:///src/botica\"",
            "\"botica[sdk]; python_version >= '3.12'\"",
            "\"botica-extra\"",
            "\"ruff\"",
        ] {
            assert_eq!(
                sdk_requirement(&script(dependencies, "")),
                None,
                "{dependencies}"
            );
        }
        let sourced = script(
            "\"botica[sdk]\"",
            "# [tool.uv.sources]\n# botica = { path = \"../botica\" }\n",
        );
        assert_eq!(sdk_requirement(&sourced), None);
    }

    #[test]
    fn a_script_with_no_metadata_or_a_broken_block_gets_nothing() {
        assert_eq!(sdk_requirement("import botica\n"), None);
        assert_eq!(
            sdk_requirement("# /// script\n# dependencies = [\"botica\"]\n"),
            None
        );
        assert_eq!(
            sdk_requirement("# /// script\n# dependencies = [\"botica\"\n# ///\n"),
            None
        );
    }

    #[test]
    fn the_settings_of_uv_in_the_block_leave_the_requirement_as_it_is() {
        let text = script(
            "\"botica[sdk]\"",
            "# [tool.uv]\n# exclude-newer-package = { botica = false }\n",
        );
        assert_eq!(
            sdk_requirement(&text),
            Some(format!("botica[sdk]=={VERSION}"))
        );
    }
}
