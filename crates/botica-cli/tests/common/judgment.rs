//! A catalog `acme` with the judgment rule `acme/judge-it`, which the deterministic rule
//! `acme/no-eval` covers, both stable and on by default.

use std::path::{Path, PathBuf};

use super::{project_in, write};

pub const JUDGMENT_README: &str = "## Directive\n\nJudge it well. More text.\n\n## Why\n\nw\n\n\
## Flag\n\nf\n\n## Do not flag\n\nn\n\n## Confirm\n\nc\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n\
## Suppression\n\ns\n";

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

pub const ROOT: &str = ".botica/rules/acme";

/// A project in `<scratch>/<group>/<name>` with the catalog, `config` as `botica.toml`, an
/// `AGENTS.md` with the markers of the block, and `app.py`, which calls `eval`.
pub fn project(group: &str, name: &str, config: &str) -> PathBuf {
    let dir = project_in(group, name, Some(config));
    let dir = std::fs::canonicalize(dir).expect("the project exists");
    write(&dir, &format!("{ROOT}/catalog.toml"), "code = \"ACME\"\n");
    write(
        &dir,
        &format!("{ROOT}/quality/family.toml"),
        "code = \"QUAL\"\ndescription = \"d\"\n",
    );
    let judge = format!("{ROOT}/quality/judge-it");
    write(
        &dir,
        &format!("{judge}/rule.toml"),
        "id = \"acme/judge-it\"\ncode = \"ACMEQUAL001\"\nsummary = \"s\"\nstatus = \"stable\"\n\
         default-enabled = true\ndefault-severity = \"warning\"\n\n[[implementations]]\n\
         kind = \"judgment\"\nlanguages = [\"python\"]\nwhen = \"edit\"\n\
         covered-by = [\"acme/no-eval\"]\nwhy-not-deterministic = \"w\"\n\n\
         [implementations.question]\ntype = \"boolean\"\ninstructions = \"Is it judged?\"\n",
    );
    write(&dir, &format!("{judge}/README.md"), JUDGMENT_README);
    let eval = format!("{ROOT}/quality/no-eval");
    write(
        &dir,
        &format!("{eval}/rule.toml"),
        "id = \"acme/no-eval\"\ncode = \"ACMEQUAL002\"\nsummary = \"Do not call eval\"\n\
         status = \"stable\"\nscope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]\n\
         default-enabled = true\ndefault-severity = \"warning\"\n\n[[implementations]]\n\
         kind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n",
    );
    write(&dir, &format!("{eval}/README.md"), README);
    write(
        &dir,
        &format!("{eval}/python.yml"),
        "id: acme/no-eval\nlanguage: python\nseverity: warning\n\
         message: \"ACMEQUAL002 (acme/no-eval): do not call eval\"\n\
         rule:\n  pattern: eval($$$ARGS)\n",
    );
    write(
        &dir,
        "AGENTS.md",
        "# Notes\n\n<!-- botica:rules -->\n<!-- /botica:rules -->\n",
    );
    write(&dir, "app.py", "x = eval(\"1\")\n");
    dir
}

pub fn read(dir: &Path, relative: &str) -> String {
    std::fs::read_to_string(dir.join(relative)).expect("read the file")
}
