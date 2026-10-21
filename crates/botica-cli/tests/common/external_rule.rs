//! The catalog `acme` of a project, with the external rule `acme/complexity`, which complexipy
//! enforces through a hook of prek.

use std::path::Path;

use super::write;

pub const SELECTS_COMPLEXITY: &str = "preview = true\nextend-select = [\"acme/complexity\"]\n";

pub const WITHOUT_THE_HOOK: &str =
    "[[repos]]\nrepo = \"local\"\n\n[[repos.hooks]]\nid = \"deptry\"\n";

pub const WITH_THE_HOOK: &str =
    "[[repos]]\nrepo = \"local\"\n\n[[repos.hooks]]\nid = \"complexipy\"\n\
entry = \"uvx complexipy --max-complexity-allowed 15\"\n";

pub const LACKS: &str = "prek.toml lacks the configuration of complexipy that `acme/complexity` \
needs: `botica explain acme/complexity` shows it";

pub const DOCS: &str = "https://github.com/rohaquinlop/complexipy";

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Tool\n\nt\n\n## Configuration\n\nc\n\n\
## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

/// Writes `.botica/rules/acme` into `dir`.
pub fn complexity_rule(dir: &Path) {
    let acme = ".botica/rules/acme";
    write(dir, &format!("{acme}/catalog.toml"), "code = \"ACME\"\n");
    write(
        dir,
        &format!("{acme}/checks/family.toml"),
        "code = \"CHK\"\ndescription = \"The checks that another tool runs\"\n",
    );
    let rule = format!("{acme}/checks/complexity");
    write(
        dir,
        &format!("{rule}/rule.toml"),
        &format!(
            "id = \"acme/complexity\"\ncode = \"ACMECHK001\"\n\
             summary = \"A Python function has a cognitive complexity of at most 15\"\n\
             status = \"preview\"\ndefault-enabled = false\ndefault-severity = \"error\"\n\n\
             [[implementations]]\nkind = \"external\"\nlanguages = [\"python\"]\n\
             tool = \"complexipy\"\ndocs = \"{DOCS}\"\nconfig = \"config/prek.toml\"\n"
        ),
    );
    write(dir, &format!("{rule}/README.md"), README);
    write(
        dir,
        &format!("{rule}/config/prek.toml"),
        "[[repos]]\n[[repos.hooks]]\nid = \"complexipy\"\n",
    );
}
