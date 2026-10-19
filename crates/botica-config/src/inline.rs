use crate::discover::Loaded;
use crate::issue::ConfigError;
use crate::model::Config;

fn refuse(message: impl Into<String>) -> ConfigError {
    ConfigError::single("--config", message)
}

/// A `--config` argument that is a TOML `key = value` pair (dotted keys reach inside tables, as in
/// `rules.file-size-guard.max = 900`), checked as far as it can be on its own: it must be TOML and
/// every key must be a setting of botica with the right type.
pub fn inline_setting(text: &str) -> Result<toml::Table, ConfigError> {
    let table: toml::Table =
        toml::from_str(text).map_err(|error| refuse(error.to_string().trim_end()))?;
    if table.is_empty() {
        return Err(refuse("empty setting: write a `key = value` pair"));
    }
    table
        .clone()
        .try_into::<Config>()
        .map_err(|error| refuse(error.to_string().trim_end()))?;
    Ok(table)
}

/// Tables are merged key by key; anything else, arrays included, is replaced.
fn merge(base: &mut toml::Table, over: &toml::Table) {
    for (key, value) in over {
        match (base.get_mut(key), value) {
            (Some(toml::Value::Table(base)), toml::Value::Table(over)) => merge(base, over),
            _ => {
                base.insert(key.clone(), value.clone());
            }
        }
    }
}

impl Loaded {
    /// Lays the settings of `--config key = value` over the ones that were read, in order: when a
    /// key comes twice, the last wins.
    pub fn with_inline(mut self, settings: &[toml::Table]) -> Result<Self, ConfigError> {
        if settings.is_empty() {
            return Ok(self);
        }
        for setting in settings {
            merge(&mut self.table, setting);
        }
        self.config = self
            .table
            .clone()
            .try_into()
            .map_err(|error: toml::de::Error| refuse(error.to_string().trim_end()))?;
        Ok(self)
    }
}
