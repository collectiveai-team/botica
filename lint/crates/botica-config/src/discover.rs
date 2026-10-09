use std::fs;
use std::path::{Path, PathBuf};

use crate::issue::ConfigError;
use crate::model::Config;

const BOTICA_TOML: &str = "botica.toml";
const PYPROJECT: &str = "pyproject.toml";

#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub root: PathBuf,
    pub source: Option<PathBuf>,
    pub config: Config,
}

fn read(path: &Path) -> Result<String, ConfigError> {
    fs::read_to_string(path).map_err(|error| {
        ConfigError::single("", format!("cannot read the file: {error}")).in_file(path)
    })
}

fn botica_toml(path: &Path) -> Result<Config, ConfigError> {
    Config::from_botica_toml(&read(path)?).map_err(|error| error.in_file(path))
}

fn pyproject(path: &Path) -> Result<Option<Config>, ConfigError> {
    Config::from_pyproject(&read(path)?).map_err(|error| error.in_file(path))
}

fn config_in(dir: &Path) -> Result<Option<(PathBuf, Config)>, ConfigError> {
    let botica_path = dir.join(BOTICA_TOML);
    let pyproject_path = dir.join(PYPROJECT);
    let from_botica = if botica_path.is_file() {
        Some(botica_toml(&botica_path)?)
    } else {
        None
    };
    let from_pyproject = if pyproject_path.is_file() {
        pyproject(&pyproject_path)?
    } else {
        None
    };
    match (from_botica, from_pyproject) {
        (Some(_), Some(_)) => Err(ConfigError::single(
            "",
            format!("both {BOTICA_TOML} and the [tool.botica] table of {PYPROJECT} configure botica in this directory: keep only one"),
        )
        .in_file(dir)),
        (Some(config), None) => Ok(Some((botica_path, config))),
        (None, Some(config)) => Ok(Some((pyproject_path, config))),
        (None, None) => Ok(None),
    }
}

fn git_root(start: &Path) -> Option<&Path> {
    start.ancestors().find(|dir| dir.join(".git").exists())
}

pub fn discover(start: &Path) -> Result<Loaded, ConfigError> {
    for dir in start.ancestors() {
        if let Some((source, config)) = config_in(dir)? {
            return Ok(Loaded {
                root: dir.to_path_buf(),
                source: Some(source),
                config,
            });
        }
    }
    Ok(Loaded {
        root: git_root(start).unwrap_or(start).to_path_buf(),
        source: None,
        config: Config::default(),
    })
}

pub fn load_file(path: &Path) -> Result<Loaded, ConfigError> {
    let is_pyproject = path.file_name().is_some_and(|name| name == PYPROJECT);
    let config = if is_pyproject {
        pyproject(path)?.ok_or_else(|| {
            ConfigError::single("", "the file has no [tool.botica] table").in_file(path)
        })?
    } else {
        botica_toml(path)?
    };
    let root = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    Ok(Loaded {
        root: root.to_path_buf(),
        source: Some(path.to_path_buf()),
        config,
    })
}
