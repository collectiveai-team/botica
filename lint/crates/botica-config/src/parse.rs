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

pub fn json_schema() -> String {
    let schema = schemars::schema_for!(Config);
    let mut text = serde_json::to_string_pretty(&schema).expect("a schema serializes to JSON");
    text.push('\n');
    text
}
