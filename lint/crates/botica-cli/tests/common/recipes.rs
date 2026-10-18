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
