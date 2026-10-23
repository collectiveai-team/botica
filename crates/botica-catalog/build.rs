use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map(|read| read.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    entries.sort();
    entries
}

fn push(table: &mut String, rules_dir: &Path, path: &Path) {
    let relative = path
        .strip_prefix(rules_dir)
        .expect("path is under the rules directory")
        .to_string_lossy()
        .replace('\\', "/");
    let absolute = path
        .canonicalize()
        .expect("rules file can be canonicalized");
    writeln!(
        table,
        "    ({relative:?}, include_str!({:?})),",
        absolute.to_string_lossy()
    )
    .expect("writing to a String cannot fail");
}

/// Every file under `dir`, at any depth, in the order of their paths.
fn push_tree(table: &mut String, root: &Path, dir: &Path) {
    for entry in sorted_entries(dir) {
        if entry.is_dir() {
            push_tree(table, root, &entry);
        } else if entry.is_file() {
            push(table, root, &entry);
        }
    }
}

/// The code of a topic: every file under `dir` but `__pycache__` and the hidden files and folders,
/// as `is_code` of `src/dir.rs` reads them.
fn push_code(table: &mut String, root: &Path, dir: &Path) {
    for entry in sorted_entries(dir) {
        let name = entry
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if name.starts_with('.') || name == "__pycache__" {
            continue;
        }
        if entry.is_dir() {
            push_code(table, root, &entry);
        } else if entry.is_file() {
            push(table, root, &entry);
        }
    }
}

fn is_file_with_extension(path: &Path, extension: &str) -> bool {
    path.is_file() && path.extension().is_some_and(|e| e == extension)
}

/// Each of `names` in `dir` that is a file, in the order of `names`.
fn push_named(table: &mut String, root: &Path, dir: &Path, names: [&str; 2]) {
    for name in names {
        let file = dir.join(name);
        if file.is_file() {
            push(table, root, &file);
        }
    }
}

/// Each file directly in `dir` with the extension `extension`.
fn push_with_extension(table: &mut String, root: &Path, dir: &Path, extension: &str) {
    for file in sorted_entries(dir) {
        if is_file_with_extension(&file, extension) {
            push(table, root, &file);
        }
    }
}

fn push_rule(table: &mut String, rules_dir: &Path, rule: &Path) {
    push_named(table, rules_dir, rule, ["rule.toml", "README.md"]);
    // The YAML of the implementations (`file` in `rule.toml`).
    push_with_extension(table, rules_dir, rule, "yml");
    // The fragments of configuration of the external implementations (`config` in `rule.toml`).
    // A fragment can sit in a hidden folder, as `config/.cargo/config.toml`, which `push_code` skips.
    push_tree(table, rules_dir, &rule.join("config"));
}

fn push_family(table: &mut String, rules_dir: &Path, family: &Path) {
    let family_toml = family.join("family.toml");
    if family_toml.is_file() {
        push(table, rules_dir, &family_toml);
    }
    for rule in sorted_entries(family).into_iter().filter(|p| p.is_dir()) {
        push_rule(table, rules_dir, &rule);
    }
}

fn push_rules(table: &mut String, manifest: &Path) {
    let rules_dir = manifest.join("../../catalog/rules");
    println!("cargo:rerun-if-changed={}", rules_dir.display());
    table.push_str("pub(crate) const EMBEDDED_FILES: &[(&str, &str)] = &[\n");
    let history = rules_dir.join("history.toml");
    if history.is_file() {
        push(table, &rules_dir, &history);
    }
    for family in sorted_entries(&rules_dir)
        .into_iter()
        .filter(|p| p.is_dir())
    {
        push_family(table, &rules_dir, &family);
    }
    table.push_str("];\n");
}

fn push_references(table: &mut String, manifest: &Path) {
    let references_dir = manifest.join("../../catalog/references");
    println!("cargo:rerun-if-changed={}", references_dir.display());
    table.push_str("pub(crate) const EMBEDDED_REFERENCES: &[(&str, &str)] = &[\n");
    for topic in sorted_entries(&references_dir)
        .into_iter()
        .filter(|p| p.is_dir())
    {
        push_named(table, &references_dir, &topic, ["topic.toml", "README.md"]);
        push_code(table, &references_dir, &topic.join("code"));
    }
    table.push_str("];\n");
}

fn push_recipes(table: &mut String, manifest: &Path) {
    let recipes_dir = manifest.join("../../catalog/recipes");
    println!("cargo:rerun-if-changed={}", recipes_dir.display());
    table.push_str("pub(crate) const EMBEDDED_RECIPES: &[(&str, &str)] = &[\n");
    push_with_extension(table, &recipes_dir, &recipes_dir, "toml");
    // The templates of the steps of the recipes.
    for folder in ["files", "sections"] {
        push_tree(table, &recipes_dir, &recipes_dir.join(folder));
    }
    table.push_str("];\n");
}

fn main() {
    let manifest =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let mut table = String::new();
    push_rules(&mut table, &manifest);
    push_references(&mut table, &manifest);
    push_recipes(&mut table, &manifest);

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"))
        .join("embedded_files.rs");
    fs::write(out, table).expect("the generated table can be written");
}
