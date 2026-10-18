use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::load::{SourceFile, CATALOG_FILE, HISTORY_FILE};
use crate::recipes::TEMPLATE_DIRS;
use crate::references::TOPIC_FILE;
use crate::sources::is_script;

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
                let name = file.file_name().map(|name| name.to_string_lossy());
                let wanted = name
                    .as_deref()
                    .is_some_and(|name| name.ends_with(".yml") || is_script(name));
                if file.is_file() && wanted {
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

/// The files of the topics under `dir`, the `references/` of a repository: the `topic.toml` and
/// the `README.md` of each folder, with paths from `dir`. A `dir` that does not exist holds none.
pub fn read_references(dir: &Path) -> io::Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }
    for topic in sorted(dir)?.into_iter().filter(|path| path.is_dir()) {
        for name in ["README.md", TOPIC_FILE] {
            if topic.join(name).is_file() {
                push(&mut files, dir, &topic.join(name))?;
            }
        }
    }
    Ok(files)
}

/// The files of the recipes under `dir`, the `recipes/` of a repository: each `<recipe>.toml`
/// that is directly in it and every file under its `files/` and `sections/`, with paths from
/// `dir`. A `dir` that does not exist holds none.
pub fn read_recipes(dir: &Path) -> io::Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }
    for file in sorted(dir)? {
        if file.is_file() && file.extension().is_some_and(|ext| ext == "toml") {
            push(&mut files, dir, &file)?;
        }
    }
    for folder in TEMPLATE_DIRS {
        push_tree(&mut files, dir, &dir.join(folder))?;
    }
    Ok(files)
}

/// Every file under `folder`, at any depth, in the order of their paths.
fn push_tree(files: &mut Vec<SourceFile>, root: &Path, folder: &Path) -> io::Result<()> {
    if !folder.is_dir() {
        return Ok(());
    }
    for entry in sorted(folder)? {
        if entry.is_dir() {
            push_tree(files, root, &entry)?;
        } else if entry.is_file() {
            push(files, root, &entry)?;
        }
    }
    Ok(())
}

/// The files of the catalogs of a project under `dir`, its `.botica/rules`, with paths from `dir`:
/// each folder gives its `catalog.toml` and what `read_files` reads of it. A `dir` that does not
/// exist holds no catalog.
pub fn read_project_files(dir: &Path) -> io::Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    for (name, catalog) in project_catalog_dirs(dir)? {
        files.extend(read_catalog_files(&name, &catalog)?);
    }
    Ok(files)
}

/// The folders of `dir` that are catalogs, as `(name, folder)`: each folder is a catalog and its
/// name is the name of the folder. A `dir` that does not exist holds none.
pub fn project_catalog_dirs(dir: &Path) -> io::Result<Vec<(String, PathBuf)>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    Ok(sorted(dir)?
        .into_iter()
        .filter(|path| path.is_dir())
        .map(|catalog| {
            let name = catalog
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            (name, catalog)
        })
        .collect())
}

/// The files of the catalog `name` that is in the folder `catalog`: its `catalog.toml` and what
/// `read_files` reads of it, with paths from the folder of the catalogs, `<name>/…`.
pub fn read_catalog_files(name: &str, catalog: &Path) -> io::Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    let catalog_toml = catalog.join(CATALOG_FILE);
    if catalog_toml.is_file() {
        files.push(SourceFile {
            path: format!("{name}/{CATALOG_FILE}"),
            text: fs::read_to_string(&catalog_toml)?,
        });
    }
    for file in read_files(catalog)? {
        files.push(SourceFile {
            path: format!("{name}/{}", file.path),
            text: file.text,
        });
    }
    Ok(files)
}
