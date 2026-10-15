use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use botica_catalog::{read_files, Catalog, Family, SourceFile};
use clap::{Args, ValueEnum};

#[derive(Args)]
pub struct NewArgs {
    /// The folder of the rule in the catalog, `<family>/<slug>`.
    rule: String,
    /// How the rule is implemented.
    #[arg(long, value_enum)]
    kind: NewKind,
    /// The language the rule reads, as ast-grep names it (`python`, `typescript`…).
    #[arg(long)]
    language: String,
    /// What runs an extension.
    #[arg(long, value_enum, required_if_eq("kind", "extension"))]
    runtime: Option<NewRuntime>,
    /// The standard the rule implements, e.g. `CES-63`.
    #[arg(long)]
    standard: Option<String>,
    /// The directory of the catalog.
    #[arg(long, default_value = "lint/rules")]
    catalog: PathBuf,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum NewKind {
    AstGrep,
    Extension,
    Native,
}

#[derive(Clone, Copy, ValueEnum)]
enum NewRuntime {
    Python,
    Node,
    Command,
}

impl NewRuntime {
    fn word(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Node => "node",
            Self::Command => "command",
        }
    }
}

const README: &str = "## Directive\n\n\
TODO: what the rule asks, and the severity it reports by default.\n\n\
## Why\n\nTODO: what goes wrong without the rule.\n\n\
## Do\n\nTODO: code that follows the rule.\n\n\
## Don't\n\nTODO: code that breaks the rule.\n\n\
## Suppression\n\nTODO: the comment or the configuration that silences a finding.\n";

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
    let existing = read_files(&args.catalog)
        .with_context(|| format!("cannot read the catalog `{}`", args.catalog.display()))?;
    let catalog = Catalog::from_files(&existing)?;
    let family = find_family(&catalog, family)?;
    if args.catalog.join(&family.name).join(slug).exists() {
        bail!("`{}/{slug}` exists in the catalog", family.name);
    }

    let draft = draft(args, &catalog, family, slug);
    let catalog_files: Vec<SourceFile> = draft
        .files
        .iter()
        .filter(|file| !file.path.contains("/fixtures/"))
        .cloned()
        .collect();
    Catalog::from_files(&[existing, catalog_files].concat())
        .with_context(|| format!("the catalog would not load with `{}`", draft.folder))?;
    write(&args.catalog, &draft.files)?;
    println!("{} ({}):", draft.folder, draft.code);
    for file in &draft.files {
        println!("  {}", file.path);
    }
    Ok(())
}

fn check_kind(args: &NewArgs) -> Result<()> {
    if args.kind == NewKind::Native {
        bail!(
            "a native rule has its code in botica-core, which `rules new` does not write: \
             add its folder by hand with that code"
        );
    }
    if args.kind != NewKind::Extension && args.runtime.is_some() {
        bail!("--runtime applies to an extension only");
    }
    Ok(())
}

fn find_family<'a>(catalog: &'a Catalog, name: &str) -> Result<&'a Family> {
    catalog
        .families()
        .iter()
        .find(|family| family.name == name)
        .with_context(|| {
            let names: Vec<&str> = catalog
                .families()
                .iter()
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
    let prefix = format!("BT{}", family.code);
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

fn rule_toml(args: &NewArgs, slug: &str, code: &str) -> String {
    let mut text = format!("id = \"{slug}\"\ncode = \"{code}\"\n");
    if let Some(standard) = &args.standard {
        text.push_str(&format!("standard = \"{standard}\"\n"));
    }
    let needs = if args.kind == NewKind::AstGrep {
        "syntax"
    } else {
        "bytes"
    };
    text.push_str(&format!(
        "summary = \"TODO: what the rule asks, in one line\"\n\
         status = \"preview\"\n\
         scope = \"file\"\n\
         needs = \"{needs}\"\n\
         run-on = [\"cli\", \"save\", \"type\"]\n\
         default-enabled = false\n\
         default-severity = \"warning\"\n\
         \n\
         [[implementations]]\n"
    ));
    let language = &args.language;
    match args.runtime {
        Some(runtime) => text.push_str(&format!(
            "kind = \"extension\"\nruntime = \"{}\"\nlanguages = [\"{language}\"]\n",
            runtime.word()
        )),
        None => text.push_str(&format!(
            "kind = \"ast-grep\"\nlanguages = [\"{language}\"]\nfile = \"{language}.yml\"\n"
        )),
    }
    text
}

fn ast_grep_yaml(args: &NewArgs, slug: &str) -> String {
    let label = match &args.standard {
        Some(standard) => format!("{standard} ({slug})"),
        None => slug.to_owned(),
    };
    format!(
        "id: {slug}\nlanguage: {}\nseverity: warning\n\
         message: \"{label}: TODO: the rule and its fix\"\nrule:\n  pattern: TODO\n",
        args.language
    )
}

fn draft(args: &NewArgs, catalog: &Catalog, family: &Family, slug: &str) -> Draft {
    let folder = format!("{}/{slug}", family.name);
    let code = next_code(catalog, family);
    let file = |name: &str, text: String| SourceFile {
        path: format!("{folder}/{name}"),
        text,
    };
    let mut files = vec![
        file("rule.toml", rule_toml(args, slug, &code)),
        file("README.md", README.to_owned()),
    ];
    if args.kind == NewKind::AstGrep {
        files.push(file(
            &format!("{}.yml", args.language),
            ast_grep_yaml(args, slug),
        ));
    }
    let (extension, comment) = (file_extension(&args.language), comment(&args.language));
    files.extend([
        file("fixtures/expected.json", EXPECTED.to_owned()),
        file(
            &format!("fixtures/valid/example.{extension}"),
            format!("{comment} TODO: code that follows the rule\n"),
        ),
        file(
            &format!("fixtures/invalid/example.{extension}"),
            format!("{comment} TODO: code that breaks the rule, with its findings in ../expected.json\n"),
        ),
    ]);
    Draft {
        folder,
        code,
        files,
    }
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
