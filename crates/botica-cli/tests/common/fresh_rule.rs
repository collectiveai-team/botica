//! A rule that no release has added: a preview rule of the catalog `acme` of a project, which has no
//! history.

use std::path::Path;

use super::write;

pub const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

/// `acme/no-eval`, code `ACMESTRUC001`, in preview and off by default, in `dir`.
pub fn lay_out(dir: &Path) {
    let root = ".botica/rules/acme";
    write(dir, &format!("{root}/catalog.toml"), "code = \"ACME\"\n");
    write(
        dir,
        &format!("{root}/structure/family.toml"),
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    let rule = format!("{root}/structure/no-eval");
    write(
        dir,
        &format!("{rule}/rule.toml"),
        "id = \"acme/no-eval\"\ncode = \"ACMESTRUC001\"\nsummary = \"Do not call eval\"\n\
         status = \"preview\"\nscope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]\n\
         default-enabled = false\ndefault-severity = \"warning\"\n\n[[implementations]]\n\
         kind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n",
    );
    write(dir, &format!("{rule}/README.md"), README);
    write(
        dir,
        &format!("{rule}/python.yml"),
        "id: acme/no-eval\nlanguage: python\nseverity: warning\n\
         message: \"ACMESTRUC001 (acme/no-eval): do not call eval\"\n\
         rule:\n  pattern: eval($$$ARGS)\n",
    );
}
