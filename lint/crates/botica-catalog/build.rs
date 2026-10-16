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

fn main() {
    let manifest =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let rules_dir = manifest.join("../../rules");
    println!("cargo:rerun-if-changed={}", rules_dir.display());

    let mut table = String::from("pub(crate) const EMBEDDED_FILES: &[(&str, &str)] = &[\n");
    let history = rules_dir.join("history.toml");
    if history.is_file() {
        push(&mut table, &rules_dir, &history);
    }
    for family in sorted_entries(&rules_dir)
        .into_iter()
        .filter(|p| p.is_dir())
    {
        let family_toml = family.join("family.toml");
        if family_toml.is_file() {
            push(&mut table, &rules_dir, &family_toml);
        }
        for rule in sorted_entries(&family).into_iter().filter(|p| p.is_dir()) {
            for name in ["rule.toml", "README.md"] {
                let file = rule.join(name);
                if file.is_file() {
                    push(&mut table, &rules_dir, &file);
                }
            }
            // The YAML of the implementations (`file` in `rule.toml`).
            for file in sorted_entries(&rule) {
                if file.is_file() && file.extension().is_some_and(|e| e == "yml") {
                    push(&mut table, &rules_dir, &file);
                }
            }
            // The fragments of configuration of the external implementations (`config`).
            for file in sorted_entries(&rule.join("config")) {
                if file.is_file() && file.extension().is_some_and(|e| e == "toml") {
                    push(&mut table, &rules_dir, &file);
                }
            }
        }
    }
    table.push_str("];\n");

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"))
        .join("embedded_files.rs");
    fs::write(out, table).expect("the generated table can be written");
}
