//! The parameter step of `botica recipe install`: a value of `--set` goes into
//! `[recipes.params]` of the configuration, so the next `install` and `check` use it too.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::recipes_install_selection::{existing, params_table, BOTICA_TOML};
use crate::recipes_install_steps::Outcome;

/// One outcome for each value of `set`. Without a configuration of botica, `botica.toml` is
/// written with them (`added`). A configuration whose `[recipes.params]` has the value is `ok`,
/// and one that lacks it or has another is `pending`: `install` does not edit it.
pub fn add(root: &Path, set: &BTreeMap<String, String>) -> Result<Vec<(String, Outcome)>> {
    if set.is_empty() {
        return Ok(Vec::new());
    }
    let Some((file, table)) = existing(root) else {
        let path = root.join(BOTICA_TOML);
        fs::write(&path, params_table(set).trim_start())
            .with_context(|| format!("cannot write `{}`", path.display()))?;
        return Ok(set
            .keys()
            .map(|name| (name.clone(), Outcome::Added(None)))
            .collect());
    };
    let written = table
        .as_ref()
        .and_then(|table| table.get("recipes"))
        .and_then(|recipes| recipes.get("params"))
        .and_then(toml::Value::as_table);
    Ok(set
        .iter()
        .map(|(name, value)| {
            let current = written.and_then(|params| params.get(name)).map(|current| {
                current
                    .as_str()
                    .map_or_else(|| current.to_string(), str::to_owned)
            });
            let outcome = if current.as_deref() == Some(value.as_str()) {
                Outcome::Present
            } else {
                Outcome::Pending(format!(
                    "the configuration in `{file}` lacks `[recipes.params] {name} = {}`; this run \
                     used the value",
                    toml::Value::String(value.clone())
                ))
            };
            (name.clone(), outcome)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect()
    }

    fn root(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("botica-params-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create the folder");
        dir
    }

    #[test]
    fn without_a_configuration_the_values_are_written_to_a_new_botica_toml() {
        let dir = root("new");

        let outcomes = add(&dir, &set(&[("cooldown-days", "14")])).expect("adds");

        assert!(matches!(outcomes[0].1, Outcome::Added(None)));
        let text = fs::read_to_string(dir.join(BOTICA_TOML)).expect("written");
        assert_eq!(text, "[recipes.params]\ncooldown-days = \"14\"\n");
    }

    #[test]
    fn a_configuration_with_the_value_is_ok_and_one_without_it_is_pending() {
        let dir = root("existing");
        fs::write(
            dir.join(BOTICA_TOML),
            "select = []\n\n[recipes.params]\ncooldown-days = 14\n",
        )
        .expect("written");

        let outcomes = add(
            &dir,
            &set(&[("cooldown-days", "14"), ("go-preference", "managed")]),
        )
        .expect("adds");

        assert!(matches!(outcomes[0].1, Outcome::Present));
        assert!(
            matches!(&outcomes[1].1, Outcome::Pending(text) if text.contains("go-preference = \"managed\""))
        );
    }
}
