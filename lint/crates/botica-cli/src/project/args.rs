use std::path::{Path, PathBuf};

use botica_config::{inline_setting, CommandLine};
use clap::{Args, ValueEnum};

/// The options that decide which configuration applies and which rules it selects.
#[derive(Args, Clone)]
pub struct ProjectArgs {
    /// A configuration file to use instead of looking for one (at most one), or a TOML
    /// `key = value` setting that goes over the file (as many as you like; for the same key the
    /// last wins), e.g. `--config 'rules.file-size-guard.max = 900'`.
    #[arg(long, value_parser = config_option)]
    pub(super) config: Vec<ConfigOption>,
    /// Run only these rules (slug, code, family, prefix or `tag:<t>`; comma-separated or repeated).
    /// Starts from scratch: the `select`, `extend-select` and `ignore` of the file do not apply.
    #[arg(long, value_delimiter = ',')]
    pub(super) select: Vec<String>,
    /// Run these rules too, on top of the selection (the file's, or the one `--select` gave).
    #[arg(long, value_delimiter = ',')]
    pub(super) extend_select: Vec<String>,
    /// Switch these rules off, on top of what the file ignores.
    #[arg(long, value_delimiter = ',')]
    pub(super) ignore: Vec<String>,
    /// Also run the rules whose status is `preview`.
    #[arg(long, overrides_with = "no_preview")]
    pub(super) preview: bool,
    /// Do not run the rules whose status is `preview`, even if the file says to. The last of
    /// `--preview` and `--no-preview` wins.
    #[arg(long, overrides_with = "preview")]
    pub(super) no_preview: bool,
    /// Which rules run: those that look at one file, those that need the whole project, or both.
    #[arg(long, value_enum, default_value_t = ScopeArg::All)]
    pub(super) scope: ScopeArg,
}

impl ProjectArgs {
    /// The same configuration with `rule` alone selected, preview or not, and on whatever the
    /// configuration says of it.
    pub fn only(&self, rule: &str) -> Self {
        let mut on = toml::Table::new();
        on.insert("enabled".to_owned(), toml::Value::Boolean(true));
        let mut by_rule = toml::Table::new();
        by_rule.insert(rule.to_owned(), toml::Value::Table(on));
        let mut setting = toml::Table::new();
        setting.insert("rules".to_owned(), toml::Value::Table(by_rule));
        let mut config = self.config.clone();
        config.push(ConfigOption::Setting(setting));
        Self {
            config,
            select: vec![rule.to_owned()],
            extend_select: Vec::new(),
            ignore: Vec::new(),
            preview: true,
            no_preview: false,
            scope: ScopeArg::All,
        }
    }
}

/// One `--config`: a file, or a setting that goes over the configuration.
#[derive(Clone)]
pub(super) enum ConfigOption {
    File(PathBuf),
    Setting(toml::Table),
}

const CONFIG_HINT: &str = "tip: a `--config` flag must either be the path to a `.toml` \
                           configuration file or a TOML `key = value` pair that overrides one setting";

fn config_option(text: &str) -> Result<ConfigOption, String> {
    match inline_setting(text) {
        Ok(setting) => Ok(ConfigOption::Setting(setting)),
        Err(_) if Path::new(text).is_file() => Ok(ConfigOption::File(PathBuf::from(text))),
        Err(error) if text.contains('=') => Err(format!("{error}\n\n  {CONFIG_HINT}")),
        Err(_) => Err(format!(
            "`{text}` is not a file that exists nor a `key = value` setting\n\n  {CONFIG_HINT}"
        )),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum ScopeArg {
    File,
    Project,
    All,
}

impl ScopeArg {
    pub(super) fn only(self) -> Option<botica_catalog::Scope> {
        match self {
            Self::File => Some(botica_catalog::Scope::File),
            Self::Project => Some(botica_catalog::Scope::Project),
            Self::All => None,
        }
    }
}

impl ProjectArgs {
    pub(super) fn command_line(&self) -> CommandLine {
        CommandLine {
            select: self.select.clone(),
            extend_select: self.extend_select.clone(),
            ignore: self.ignore.clone(),
            preview: match (self.preview, self.no_preview) {
                (_, true) => Some(false),
                (true, false) => Some(true),
                (false, false) => None,
            },
        }
    }
}
