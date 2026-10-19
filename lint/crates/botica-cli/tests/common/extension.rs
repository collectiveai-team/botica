//! What the tests of the extension rules share: a project with the catalog `acme` and rules whose
//! scripts are shell scripts that speak the protocol.

use std::path::{Path, PathBuf};
use std::process::Output;

use super::{command, project_in, stdout, write};

pub const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

/// A shell script that speaks the protocol and declares `rule`. It notes where it started in
/// `starts.log`, and answers every `lint/files` with one finding of `rule` and `severity` for each
/// file it was sent.
pub fn script(rule: &str, severity: &str) -> String {
    script_for(rule, rule, severity)
}

/// As [`script`], declaring `declares` and answering with findings of `rule`.
pub fn script_for(declares: &str, rule: &str, severity: &str) -> String {
    format!(
        r#"echo "{rule} $(pwd)" >> starts.log
while IFS= read -r line; do
  case "$line" in
    *'"initialize"'*) printf '%s\n' '{{"jsonrpc":"2.0","id":1,"result":{{"rules":[{{"id":"{declares}","languages":["python"]}}]}}}}' ;;
    *'"lint/files"'*)
      items=""
      for path in $(printf '%s' "$line" | grep -o '"path":"[^"]*"' | sed 's/"path":"//; s/"$//'); do
        one='{{"path":"'"$path"'","rule":"{rule}","severity":"{severity}","start":0,"end":1,"message":"seen"}}'
        items="${{items:+$items,}}$one"
      done
      printf '%s\n' '{{"jsonrpc":"2.0","id":2,"result":{{"diagnostics":['"$items"']}}}}' ;;
    *'"shutdown"'*) printf '%s\n' '{{"jsonrpc":"2.0","id":3,"result":null}}'; exit 0 ;;
  esac
done
"#
    )
}

pub fn rule_toml(slug: &str, code: &str, extra: &str, implementation: &str) -> String {
    format!(
        "id = \"acme/{slug}\"\ncode = \"{code}\"\nsummary = \"s\"\nstatus = \"stable\"\n\
         scope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]\ndefault-enabled = true\n\
         default-severity = \"warning\"\n{extra}\n\n{implementation}"
    )
}

pub fn implementation(runtime: &str, languages: &str, how: &str) -> String {
    format!(
        "[[implementations]]\nkind = \"extension\"\nruntime = \"{runtime}\"\n\
         languages = [{languages}]\n{how}\n"
    )
}

pub fn folder(slug: &str) -> String {
    format!(".botica/rules/acme/structure/{slug}")
}

/// Adds the rule `acme/<slug>` whose script answers with findings of `reports` as rule. It runs
/// as the shell command `sh <folder>/ext.sh`.
pub fn add_rule(dir: &Path, slug: &str, code: &str, extra: &str, reports: &str) {
    add_rule_with(dir, slug, code, extra, reports, "error");
}

pub fn add_rule_with(
    dir: &Path,
    slug: &str,
    code: &str,
    extra: &str,
    reports: &str,
    severity: &str,
) {
    let root = folder(slug);
    let how = format!("command = [\"sh\", \"{root}/ext.sh\"]");
    write(
        dir,
        &format!("{root}/rule.toml"),
        &rule_toml(
            slug,
            code,
            extra,
            &implementation("command", "\"python\"", &how),
        ),
    );
    write(dir, &format!("{root}/README.md"), README);
    write(
        dir,
        &format!("{root}/ext.sh"),
        &script_for(&format!("acme/{slug}"), reports, severity),
    );
}

pub fn project(name: &str, config: &str) -> PathBuf {
    let dir = project_in("extension-rules", name, Some(config));
    let dir = std::fs::canonicalize(dir).expect("the project exists");
    let root = ".botica/rules/acme";
    write(&dir, &format!("{root}/catalog.toml"), "code = \"ACME\"\n");
    write(
        &dir,
        &format!("{root}/structure/family.toml"),
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    write(&dir, "app.py", "x = 1\n");
    dir
}

pub fn lint(dir: &Path, args: &[&str]) -> Output {
    let mut all = vec!["lint", "."];
    all.extend(args);
    command(dir, &all)
}

/// The lines of `starts.log`: each process of a rule writes one when it starts.
pub fn starts(dir: &Path) -> Vec<String> {
    let mut lines: Vec<String> = std::fs::read_to_string(dir.join("starts.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    lines.sort();
    lines
}

/// The findings, as `path rule` of each line of the output.
pub fn findings(output: &Output) -> Vec<String> {
    stdout(output)
        .lines()
        .filter_map(|line| {
            let (position, rest) = line.split_once(": ")?;
            let rule = rest.split_once('[')?.1.split_once(']')?.0;
            Some(format!("{} {rule}", position.split(':').next()?))
        })
        .collect()
}
