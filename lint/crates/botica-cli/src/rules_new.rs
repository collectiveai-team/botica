use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use botica_catalog::{Catalog, Family, Rule, SourceFile, CONFIG_DIR};
use clap::{Args, ValueEnum};

use crate::project::PROJECT_RULES;
use crate::rules_new_extension::{self, NewRuntime};
use crate::rules_new_target::Target;

#[derive(Args)]
pub struct NewArgs {
    /// The folder of the rule in the catalog, `<family>/<slug>`.
    rule: String,
    /// How the rule is implemented. An `extension` goes into a catalog of a project only.
    #[arg(long, value_enum)]
    kind: NewKind,
    /// The language the rule reads, as ast-grep names it (`python`, `typescript`…).
    #[arg(long)]
    language: String,
    /// What runs an extension: its script with `python` or `node`, or a command.
    #[arg(long, value_enum)]
    runtime: Option<NewRuntime>,
    /// The standard the rule implements, e.g. `CES-63`.
    #[arg(long)]
    standard: Option<String>,
    /// The tool that enforces an external rule, e.g. `complexipy`.
    #[arg(long, required_if_eq("kind", "external"))]
    tool: Option<String>,
    /// The address of the documentation of the tool of an external rule.
    #[arg(long, required_if_eq("kind", "external"))]
    docs: Option<String>,
    /// The file of the project that the configuration of the tool goes into, e.g. `prek.toml`:
    /// the rule gets the fragment `config/<file>` to fill in.
    #[arg(long)]
    config: Option<String>,
    /// The directory of the catalog: `lint/rules` for the built-in one, or `.botica/rules/<name>`
    /// for a catalog of a project (the one with a `catalog.toml`).
    #[arg(long, default_value = "lint/rules")]
    catalog: PathBuf,
    /// A judgment rule of the catalog that the new deterministic rule starts from: the new rule
    /// takes its summary, its standard and the text of its README, and joins its `covered-by`.
    #[arg(long)]
    from: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum NewKind {
    AstGrep,
    Judgment,
    External,
    Extension,
    Native,
}

const README: &str = "## Directive\n\n\
TODO: what the rule asks, and the severity it reports by default.\n\n\
## Why\n\nTODO: what goes wrong without the rule.\n\n\
## Do\n\nTODO: code that follows the rule.\n\n\
## Don't\n\nTODO: code that breaks the rule.\n\n\
## Suppression\n\nTODO: the comment or the configuration that silences a finding.\n";

const JUDGMENT_README: &str = "## Directive\n\n\
TODO: what the judge asks of the code.\n\n\
## Why\n\nTODO: what goes wrong without the rule.\n\n\
## Flag\n\nTODO: what the judge reports.\n\n\
## Do not flag\n\nTODO: what the judge leaves alone in every repository.\n\n\
## Confirm\n\nTODO: what the judge checks before it reports.\n\n\
## Do\n\nTODO: code that follows the rule.\n\n\
## Don't\n\nTODO: code that breaks the rule.\n\n\
## Suppression\n\nTODO: the `ignore-file` comment, with its reason, or the configuration that \
silences the rule.\n";

const EXTERNAL_README: &str = "## Directive\n\n\
TODO: what the rule asks, and the tool that measures it.\n\n\
## Why\n\nTODO: what goes wrong without the rule.\n\n\
## Tool\n\nTODO: the tool, its documentation, and the configuration that enforces the rule.\n\n\
## Configuration\n\nTODO: the configuration the tool needs, as a repository writes it.\n\n\
## Do\n\nTODO: code that follows the rule.\n\n\
## Don't\n\nTODO: code that breaks the rule.\n\n\
## Suppression\n\nTODO: how the tool silences a finding.\n";

const WHY: &str =
    "TODO: why no check of the code or of the configuration of a repository decides it";

const INSTRUCTIONS: &str = "TODO: the question the judge answers about the code";

const EXPECTED: &str = "{\n  \"diagnostics\": []\n}\n";

/// What `rules new` writes: the path of each file under the catalog and its text.
struct Draft {
    folder: String,
    code: String,
    files: Vec<SourceFile>,
}

pub fn run(args: &NewArgs) -> Result<()> {
    check_kind(args)?;
    let (family, slug) = args
        .rule
        .split_once('/')
        .filter(|(family, slug)| !family.is_empty() && !slug.contains('/'))
        .with_context(|| format!("`{}` must be `<family>/<slug>`", args.rule))?;
    let target = Target::load(&args.catalog)?;
    check_extension(args, &target)?;
    let family = find_family(&target, family)?;
    if args.catalog.join(&family.name).join(slug).exists() {
        bail!("`{}/{slug}` exists in the catalog", family.name);
    }

    let source = judgment_source(args, &target)?;
    let draft = draft(args, &target, family, slug, source);
    let catalog_files: Vec<SourceFile> = draft
        .files
        .iter()
        .filter(|file| {
            !file.path.contains("/fixtures/")
                && !file
                    .path
                    .ends_with(&format!("/{}", rules_new_extension::COMMAND_SCRIPT))
        })
        .cloned()
        .collect();
    target
        .loads_with(&catalog_files)
        .with_context(|| format!("the catalog would not load with `{}`", draft.folder))?;
    write(&args.catalog, &draft.files)?;
    println!("{} ({}):", draft.folder, draft.code);
    for file in &draft.files {
        println!("  {}", file.path);
    }
    if let Some(source) = source {
        let id = target.id(slug);
        let changed = cover(&args.catalog, &target, source, &id)?;
        println!("{changed}: `covered-by` names `{id}`");
    }
    Ok(())
}

/// The judgment rule that `--from` names, in the catalog.
fn judgment_source<'a>(args: &NewArgs, target: &'a Target) -> Result<Option<&'a Rule>> {
    let Some(name) = &args.from else {
        return Ok(None);
    };
    if args.kind == NewKind::Judgment {
        bail!(
            "--from writes a deterministic rule that covers a judgment rule: pick another --kind"
        );
    }
    let rule = target
        .catalog
        .resolve(name)
        .with_context(|| format!("`{name}` is not a rule of the catalog"))?;
    if !rule.is_judgment() {
        bail!("`{}` is not a judgment rule", rule.id);
    }
    Ok(Some(rule))
}

/// Adds `id` to `covered-by` of the judgment rule `source`, in its `rule.toml`, and says which
/// file changed.
fn cover(catalog: &Path, target: &Target, source: &Rule, id: &str) -> Result<String> {
    let slug = match &target.project {
        Some(name) => source
            .id
            .strip_prefix(name.as_str())
            .and_then(|rest| rest.strip_prefix('/'))
            .unwrap_or(&source.id),
        None => &source.id,
    };
    let relative = format!(
        "{}/{slug}/rule.toml",
        source.family.as_deref().unwrap_or_default()
    );
    let path = catalog.join(&relative);
    let text =
        fs::read_to_string(&path).with_context(|| format!("cannot read `{}`", path.display()))?;
    let changed = with_cover(&text, id).with_context(|| {
        format!("add `{id}` to `covered-by` of `{relative}` by hand: botica cannot place it")
    })?;
    fs::write(&path, changed).with_context(|| format!("cannot write `{}`", path.display()))?;
    Ok(relative)
}

/// The text of a `rule.toml` with `id` in the `covered-by` of its judgment implementation: added
/// to a list on one line, or as a new line after `kind = "judgment"`. `None` when it is written
/// some other way.
fn with_cover(text: &str, id: &str) -> Option<String> {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    if let Some(line) = lines
        .iter_mut()
        .find(|l| l.trim_start().starts_with("covered-by"))
    {
        let inside = line.split_once('[')?.1.strip_suffix(']')?.trim().to_owned();
        *line = if inside.is_empty() {
            format!("covered-by = [\"{id}\"]")
        } else {
            format!("covered-by = [{inside}, \"{id}\"]")
        };
    } else {
        let at = lines
            .iter()
            .position(|l| l.trim() == "kind = \"judgment\"")?;
        lines.insert(at + 1, format!("covered-by = [\"{id}\"]"));
    }
    Some(lines.join("\n") + "\n")
}

fn check_kind(args: &NewArgs) -> Result<()> {
    if args.kind == NewKind::Native {
        bail!(
            "a native rule has its code in botica-core, which `rules new` does not write: \
             add its folder by hand with that code"
        );
    }
    let tool_flags = args.tool.is_some() || args.docs.is_some() || args.config.is_some();
    if args.kind != NewKind::External && tool_flags {
        bail!("--tool, --docs and --config apply to an external rule only");
    }
    if args.kind != NewKind::Extension && args.runtime.is_some() {
        bail!("--runtime applies to an extension only");
    }
    Ok(())
}

fn check_extension(args: &NewArgs, target: &Target) -> Result<()> {
    if args.kind != NewKind::Extension {
        return Ok(());
    }
    if target.project.is_none() {
        bail!(
            "an extension is a program of a project, which the built-in catalog does not take: \
             write it in a catalog of the project with `--catalog .botica/rules/<name>`"
        );
    }
    if args.runtime.is_none() {
        bail!("an extension needs --runtime: python, node or command");
    }
    Ok(())
}

fn find_family<'a>(target: &'a Target, name: &str) -> Result<&'a Family> {
    target
        .families()
        .find(|family| family.name == name)
        .with_context(|| {
            let names: Vec<&str> = target
                .families()
                .map(|family| family.name.as_str())
                .collect();
            format!(
                "`{name}` is not a family of the catalog: {}",
                names.join(", ")
            )
        })
}

/// The code after the highest one of the family: a code is never reused.
fn next_code(catalog: &Catalog, family: &Family) -> String {
    let prefix = family.prefix();
    let highest = catalog
        .rules()
        .iter()
        .filter_map(|rule| rule.code.strip_prefix(prefix.as_str()))
        .filter_map(|digits| digits.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("{prefix}{:03}", highest + 1)
}

fn file_extension(language: &str) -> &str {
    match language {
        "python" => "py",
        "typescript" => "ts",
        "javascript" => "js",
        "rust" => "rs",
        "kotlin" => "kt",
        other => other,
    }
}

fn comment(language: &str) -> &'static str {
    if language == "python" {
        "#"
    } else {
        "//"
    }
}

/// How a new rule is named: the folder, the id, and the folder from the root of the project.
struct Names<'a> {
    slug: &'a str,
    id: String,
    root: String,
}

fn rule_toml(args: &NewArgs, names: &Names, code: &str, source: Option<&Rule>) -> String {
    let mut text = format!("id = \"{}\"\ncode = \"{code}\"\n", names.id);
    let standard = args
        .standard
        .as_deref()
        .or_else(|| source.and_then(|rule| rule.standard.as_deref()));
    if let Some(standard) = standard {
        text.push_str(&format!("standard = \"{standard}\"\n"));
    }
    let summary = source.map_or("TODO: what the rule asks, in one line", |rule| {
        rule.summary.as_str()
    });
    text.push_str(&format!(
        "summary = {}\nstatus = \"preview\"\n",
        toml::Value::String(summary.to_owned())
    ));
    let scope = match args.kind {
        NewKind::AstGrep => Some("\"cli\", \"save\", \"type\""),
        NewKind::Extension => Some("\"cli\""),
        NewKind::Judgment | NewKind::External | NewKind::Native => None,
    };
    if let Some(run_on) = scope {
        text.push_str(&format!(
            "scope = \"file\"\nneeds = \"syntax\"\nrun-on = [{run_on}]\n"
        ));
    }
    text.push_str(
        "default-enabled = false\ndefault-severity = \"warning\"\n\n[[implementations]]\n",
    );
    let language = &args.language;
    text.push_str(&match args.kind {
        NewKind::Judgment => format!(
            "kind = \"judgment\"\nlanguages = [\"{language}\"]\nwhen = \"edit\"\n\
             why-not-deterministic = \"{WHY}\"\n\n[implementations.question]\n\
             type = \"boolean\"\ninstructions = \"{INSTRUCTIONS}\"\n"
        ),
        NewKind::External => external_implementation(args),
        NewKind::Extension => format!(
            "kind = \"extension\"\n{}",
            rules_new_extension::implementation(
                args.runtime.unwrap_or(NewRuntime::Command),
                language,
                names.slug,
                &names.root
            )
        ),
        _ => format!(
            "kind = \"ast-grep\"\nlanguages = [\"{language}\"]\nfile = \"{language}.yml\"\n"
        ),
    });
    text
}

fn external_implementation(args: &NewArgs) -> String {
    let mut text = format!(
        "kind = \"external\"\nlanguages = [\"{}\"]\ntool = \"{}\"\ndocs = \"{}\"\ncodes = []\n",
        args.language,
        args.tool.as_deref().unwrap_or_default(),
        args.docs.as_deref().unwrap_or_default()
    );
    if let Some(target) = &args.config {
        text.push_str(&format!("config = \"{CONFIG_DIR}{target}\"\n"));
    }
    text
}

fn ast_grep_yaml(args: &NewArgs, id: &str) -> String {
    let label = match &args.standard {
        Some(standard) => format!("{standard} ({id})"),
        None => id.to_owned(),
    };
    format!(
        "id: {id}\nlanguage: {}\nseverity: warning\n\
         message: \"{label}: TODO: the rule and its fix\"\nrule:\n  pattern: TODO\n",
        args.language
    )
}

/// The README of a deterministic rule from the README of the judgment rule it covers: its
/// Directive, Why, Do and Don't, and its Do not flag as Exceptions.
fn readme_from(source: &Rule) -> String {
    let sections = source.sections();
    let text = |title: &str| {
        sections
            .iter()
            .find(|section| section.title == title)
            .map(|section| section.text.trim().to_owned())
    };
    let mut readme = String::new();
    for (title, from) in [
        ("Directive", "Directive"),
        ("Why", "Why"),
        ("Exceptions", "Do not flag"),
        ("Do", "Do"),
        ("Don't", "Don't"),
    ] {
        if let Some(body) = text(from).filter(|body| !body.is_empty()) {
            readme.push_str(&format!("## {title}\n\n{body}\n\n"));
        }
    }
    readme.push_str(
        "## Suppression\n\nTODO: the comment or the configuration that silences a finding.\n",
    );
    readme
}

fn draft(
    args: &NewArgs,
    target: &Target,
    family: &Family,
    slug: &str,
    source: Option<&Rule>,
) -> Draft {
    let folder = format!("{}/{slug}", family.name);
    let code = next_code(&target.catalog, family);
    let names = Names {
        slug,
        id: target.id(slug),
        root: match &target.project {
            Some(project) => format!("{PROJECT_RULES}/{project}/{folder}"),
            None => folder.clone(),
        },
    };
    let file = |name: &str, text: String| SourceFile {
        path: format!("{folder}/{name}"),
        text,
    };
    let mut files = vec![
        file("rule.toml", rule_toml(args, &names, &code, source)),
        file(
            "README.md",
            source.map_or_else(
                || {
                    match args.kind {
                        NewKind::Judgment => JUDGMENT_README,
                        NewKind::External => EXTERNAL_README,
                        _ => README,
                    }
                    .to_owned()
                },
                readme_from,
            ),
        ),
    ];
    if args.kind == NewKind::External {
        if let Some(target) = &args.config {
            files.push(file(
                &format!("{CONFIG_DIR}{target}"),
                format!(
                    "# TODO: the configuration of {} in {target} that enforces the rule\n",
                    args.tool.as_deref().unwrap_or_default()
                ),
            ));
        }
        return Draft {
            folder,
            code,
            files,
        };
    }
    if args.kind == NewKind::AstGrep {
        files.push(file(
            &format!("{}.yml", args.language),
            ast_grep_yaml(args, &names.id),
        ));
    }
    if let (NewKind::Extension, Some(runtime)) = (args.kind, args.runtime) {
        let script = rules_new_extension::script(runtime, &names.id, &args.language, slug);
        files.push(file(&script.name, script.text));
    }
    let (extension, comment) = (file_extension(&args.language), comment(&args.language));
    files.extend([
        file("fixtures/expected.json", expected(args.kind, extension)),
        file(
            &format!("fixtures/valid/example.{extension}"),
            format!("{comment} TODO: code that follows the rule\n"),
        ),
        file(
            &format!("fixtures/invalid/example.{extension}"),
            format!(
                "{comment} TODO: code that breaks the rule, with its {} in ../expected.json\n",
                if args.kind == NewKind::Judgment {
                    "verdict"
                } else {
                    "findings"
                }
            ),
        ),
    ]);
    Draft {
        folder,
        code,
        files,
    }
}

/// The `expected.json` of a new rule: no finding, or for a judgment rule the band that abide must
/// give each fixture.
fn expected(kind: NewKind, extension: &str) -> String {
    if kind != NewKind::Judgment {
        return EXPECTED.to_owned();
    }
    format!(
        "{{\n  \"verdicts\": [\n    {{ \"file\": \"valid/example.{extension}\", \"band\": \"clear\" }},\n    \
         {{ \"file\": \"invalid/example.{extension}\", \"band\": \"act\" }}\n  ]\n}}\n"
    )
}

fn write(catalog: &Path, files: &[SourceFile]) -> Result<()> {
    for file in files {
        let path = catalog.join(&file.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create `{}`", parent.display()))?;
        }
        fs::write(&path, &file.text)
            .with_context(|| format!("cannot write `{}`", path.display()))?;
    }
    Ok(())
}
