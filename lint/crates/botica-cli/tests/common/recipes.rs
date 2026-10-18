//! What the tests of `botica recipe` share: a project with recipes of its own, and the commands.

use std::path::{Path, PathBuf};
use std::process::Output;

use super::{command, project_in, write};

/// The three rules that the built-in recipe `base` lists.
pub const BASE_RULES: [&str; 3] = ["file-size-guard", "no-notebooks", "no-artifacts-in-package"];

/// The selection that makes every rule of `generic` active, as the arguments of a command.
pub const SELECT_BASE: &str = "file-size-guard,no-notebooks,no-artifacts-in-package";

pub fn project(name: &str, config: Option<&str>) -> PathBuf {
    project_in("recipes", name, config)
}

/// Writes `recipes/<id>.toml`, and a file for each template that it names.
pub fn add_recipe(dir: &Path, id: &str, toml: &str) {
    write(dir, &format!("recipes/{id}.toml"), toml);
    for line in toml.lines() {
        let named = line
            .strip_prefix("template = \"")
            .map(|rest| rest.trim_end_matches('"'));
        if let Some(path) =
            named.filter(|path| path.starts_with("files/") || path.starts_with("sections/"))
        {
            write(dir, &format!("recipes/{path}"), "template\n");
        }
    }
}

/// A `npx` that records its arguments in `.npx-calls` of the working directory and installs each
/// skill that follows `--skill`, as the CLI of skills does.
pub const NPX_INSTALLS: &str = "echo \"$*\" >> \"$PWD/.npx-calls\"\nseen=0\nfor a in \"$@\"; do\n  \
    if [ \"$seen\" = 1 ]; then\n    mkdir -p \".agents/skills/$a\"\n    \
    echo skill > \".agents/skills/$a/SKILL.md\"\n  fi\n  if [ \"$a\" = \"--skill\" ]; then seen=1; fi\ndone";

/// `botica recipe <args>` with a `npx` that runs `script` first on the `PATH`.
#[cfg(unix)]
pub fn with_npx(dir: &Path, script: &str, args: &[&str]) -> Output {
    use std::os::unix::fs::PermissionsExt;

    let name = dir.file_name().expect("a name").to_string_lossy();
    let bin = dir.with_file_name(format!("{name}-bin"));
    std::fs::create_dir_all(&bin).expect("create the folder");
    let fake = bin.join("npx");
    std::fs::write(&fake, format!("#!/bin/sh\n{script}\n")).expect("write the script");
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    run_with_path(dir, &format!("{}:/usr/bin:/bin", bin.display()), args)
}

/// `botica recipe <args>` where no `npx` is on the `PATH`.
pub fn without_npx(dir: &Path, args: &[&str]) -> Output {
    let empty = dir.join("no-bin");
    std::fs::create_dir_all(&empty).expect("create the folder");
    run_with_path(dir, &empty.display().to_string(), args)
}

fn run_with_path(dir: &Path, path: &str, args: &[&str]) -> Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_botica"))
        .arg("recipe")
        .args(args)
        .env("PATH", path)
        .current_dir(dir)
        .output()
        .expect("run botica")
}

/// The lines that the fake `npx` recorded, one for each call.
pub fn npx_calls(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join(".npx-calls"))
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// Sets the text of a template of the recipes of the project, such as `files/prek.toml`.
pub fn template(dir: &Path, path: &str, text: &str) {
    write(dir, &format!("recipes/{path}"), text);
}

/// Writes `skills-lock.json` with an entry for each name, in the shape the CLI of skills writes.
pub fn lock_skills(dir: &Path, names: &[&str]) {
    let entries: Vec<String> = names
        .iter()
        .map(|name| format!("\"{name}\": {{\"source\": \"acme/skills\"}}"))
        .collect();
    let text = format!("{{\"version\": 1, \"skills\": {{{}}}}}", entries.join(", "));
    write(dir, "skills-lock.json", &text);
}

/// Writes `.agents/skills/<name>/SKILL.md`, as an installed skill has it.
pub fn install_skill(dir: &Path, name: &str) {
    write(dir, &format!(".agents/skills/{name}/SKILL.md"), "skill\n");
}

pub fn recipe(dir: &Path, args: &[&str]) -> Output {
    let mut all = vec!["recipe"];
    all.extend_from_slice(args);
    command(dir, &all)
}

const JUDGMENT_README: &str = "## Directive\n\nd\n\n## Why\n\nw\n\n## Flag\n\nf\n\n\
## Do not flag\n\nn\n\n## Confirm\n\nc\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

/// The catalog `acme` of the project, with the judgment rule `acme/judge-it`, which runs only with
/// `--preview`.
pub fn judgment_rule(dir: &Path) {
    let acme = ".botica/rules/acme";
    write(dir, &format!("{acme}/catalog.toml"), "code = \"ACME\"\n");
    write(
        dir,
        &format!("{acme}/structure/family.toml"),
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    let rule = format!("{acme}/structure/judge-it");
    write(
        dir,
        &format!("{rule}/rule.toml"),
        "id = \"acme/judge-it\"\ncode = \"ACMESTRUC001\"\nsummary = \"s\"\nstatus = \"preview\"\n\
         default-enabled = false\ndefault-severity = \"warning\"\n\n[[implementations]]\n\
         kind = \"judgment\"\nlanguages = [\"python\"]\nwhen = \"edit\"\nwhy-not-deterministic = \"w\"\n\n\
         [implementations.question]\ntype = \"boolean\"\ninstructions = \"Is it judged?\"\n",
    );
    write(dir, &format!("{rule}/README.md"), JUDGMENT_README);
}
