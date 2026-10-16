use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::load::{SourceFile, CATALOG_FILE, HISTORY_FILE};

fn sorted(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<Vec<_>>>()?;
    entries.sort();
    Ok(entries)
}

fn push(files: &mut Vec<SourceFile>, root: &Path, path: &Path) -> io::Result<()> {
    let relative = path
        .strip_prefix(root)
        .expect("a file of the catalog is under its directory")
        .to_string_lossy()
        .replace('\\', "/");
    files.push(SourceFile {
        path: relative,
        text: fs::read_to_string(path)?,
    });
    Ok(())
}

/// The files of the catalog under `dir` that `build.rs` embeds in the binary: `history.toml`, the
/// `family.toml` of each family, and the `rule.toml`, `README.md`, YAML and `config/*.toml` of
/// each rule.
pub fn read_files(dir: &Path) -> io::Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    let history = dir.join(HISTORY_FILE);
    if history.is_file() {
        push(&mut files, dir, &history)?;
    }
    for family in sorted(dir)?.into_iter().filter(|path| path.is_dir()) {
        let family_toml = family.join("family.toml");
        if family_toml.is_file() {
            push(&mut files, dir, &family_toml)?;
        }
        for rule in sorted(&family)?.into_iter().filter(|path| path.is_dir()) {
            for name in ["rule.toml", "README.md"] {
                if rule.join(name).is_file() {
                    push(&mut files, dir, &rule.join(name))?;
                }
            }
            for file in sorted(&rule)? {
                if file.is_file() && file.extension().is_some_and(|ext| ext == "yml") {
                    push(&mut files, dir, &file)?;
                }
            }
            let config = rule.join("config");
            if config.is_dir() {
                for file in sorted(&config)? {
                    if file.is_file() && file.extension().is_some_and(|ext| ext == "toml") {
                        push(&mut files, dir, &file)?;
                    }
                }
            }
        }
    }
    Ok(files)
}

/// The files of the catalogs of a project under `dir`, its `.botica/rules`, with paths from `dir`:
/// each folder gives its `catalog.toml` and what `read_files` reads of it. A `dir` that does not
/// exist holds no catalog.
pub fn read_project_files(dir: &Path) -> io::Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }
    for catalog in sorted(dir)?.into_iter().filter(|path| path.is_dir()) {
        let name = catalog
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let catalog_toml = catalog.join(CATALOG_FILE);
        if catalog_toml.is_file() {
            push(&mut files, dir, &catalog_toml)?;
        }
        for file in read_files(&catalog)? {
            files.push(SourceFile {
                path: format!("{name}/{}", file.path),
                text: file.text,
            });
        }
    }
    Ok(files)
}
