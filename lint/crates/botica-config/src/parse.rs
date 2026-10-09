use serde::Deserialize;

use crate::issue::ConfigError;
use crate::model::Config;

#[derive(Deserialize)]
struct PyProject {
    tool: Option<Tool>,
}

#[derive(Deserialize)]
struct Tool {
    botica: Option<Config>,
}

fn syntax_error(error: &toml::de::Error) -> ConfigError {
    ConfigError::single("", error.to_string().trim_end())
}

impl Config {
    pub fn from_botica_toml(text: &str) -> Result<Self, ConfigError> {
        toml::from_str(text).map_err(|error| syntax_error(&error))
    }

    pub fn from_pyproject(text: &str) -> Result<Option<Self>, ConfigError> {
        let parsed: PyProject = toml::from_str(text).map_err(|error| syntax_error(&error))?;
        Ok(parsed.tool.and_then(|tool| tool.botica))
    }
}

/// The settings of a `botica.toml` as written, to lay `--config` settings over them.
pub(crate) fn botica_table(text: &str) -> Result<toml::Table, ConfigError> {
    toml::from_str(text).map_err(|error| syntax_error(&error))
}

/// The `[tool.botica]` table of a `pyproject.toml` as written, empty when there is none.
pub(crate) fn pyproject_table(text: &str) -> Result<toml::Table, ConfigError> {
    let all: toml::Table = toml::from_str(text).map_err(|error| syntax_error(&error))?;
    Ok(all
        .get("tool")
        .and_then(toml::Value::as_table)
        .and_then(|tool| tool.get("botica"))
        .and_then(toml::Value::as_table)
        .cloned()
        .unwrap_or_default())
}

pub fn json_schema() -> String {
    let schema = schemars::schema_for!(Config);
    let mut text = serde_json::to_string_pretty(&schema).expect("a schema serializes to JSON");
    text.push('\n');
    text
}
