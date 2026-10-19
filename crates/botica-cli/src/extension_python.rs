//! The Python that `uv` runs the scripts of the extension rules with: `[extensions] python` of the
//! configuration, by default the version that the project pins.

use std::path::Path;

/// The Python of a project that pins none.
pub const DEFAULT_PYTHON: &str = "3.13";

/// The arguments of `uv run` and `uv sync` that choose the Python, for the setting `python` of
/// `[extensions]` (`project` when absent) in the project at `root`. A free-threaded Python is an
/// error.
pub fn uv_python(setting: Option<&str>, root: &Path) -> Result<Vec<String>, String> {
    let (version, source) = match setting.unwrap_or("project") {
        "system" => {
            return Ok(vec![
                "--python-preference".to_owned(),
                "only-system".to_owned(),
            ])
        }
        "project" => project_python(root),
        version => (version.to_owned(), "`[extensions] python`".to_owned()),
    };
    if is_free_threaded(&version) {
        return Err(format!(
            "{source} asks for the free-threaded Python `{version}`: the extensions of botica \
             run on a Python with the GIL, name one such as `3.13`"
        ));
    }
    Ok(vec!["--python".to_owned(), version])
}

/// The version that the project pins and where it is written: `.python-version`, the
/// `pyvenv.cfg` of `.venv`, `requires-python` of `pyproject.toml`, else [`DEFAULT_PYTHON`].
fn project_python(root: &Path) -> (String, String) {
    let read = |name: &str| std::fs::read_to_string(root.join(name)).ok();
    if let Some(version) = read(".python-version").as_deref().and_then(pinned) {
        return (version, "`.python-version`".to_owned());
    }
    if let Some(version) = read(".venv/pyvenv.cfg").as_deref().and_then(venv_version) {
        return (version, "`.venv/pyvenv.cfg`".to_owned());
    }
    if let Some(version) = read("pyproject.toml").as_deref().and_then(requires_python) {
        return (version, "`requires-python` of `pyproject.toml`".to_owned());
    }
    (DEFAULT_PYTHON.to_owned(), "botica".to_owned())
}

/// The first line of `.python-version` that is not empty or a comment.
fn pinned(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
}

/// The major and minor version of `version_info` (uv) or `version` (venv) of `pyvenv.cfg`.
fn venv_version(text: &str) -> Option<String> {
    let value = text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        matches!(key.trim(), "version_info" | "version").then(|| value.trim().to_owned())
    })?;
    let mut parts = value.split('.');
    Some(format!("{}.{}", parts.next()?, parts.next()?))
}

fn requires_python(text: &str) -> Option<String> {
    let document: toml::Table = text.parse().ok()?;
    let project = document.get("project")?.as_table()?;
    Some(project.get("requires-python")?.as_str()?.trim().to_owned())
}

/// `3.13t`, `3.14+freethreaded`, `cpython-3.14t`: a request for a Python without the GIL.
fn is_free_threaded(version: &str) -> bool {
    version.contains("freethreaded")
        || version
            .split(['+', '-', '@'])
            .any(|part| part.ends_with('t') && part.starts_with(|c: char| c.is_ascii_digit()))
}
