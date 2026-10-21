use crate::extension_pool::{ExtensionPool, Launcher};
use crate::{extension, project, rules, timings};
use anyhow::{Context, Result};
use botica_astgrep::Tuning;
use botica_config::Resolution;
use botica_core::registry::IO_ERROR;
use botica_core::report::{self, ErrorKind, Format, Report, RunError};
use botica_core::walk::{SourceFile, Unreadable};
use botica_core::{postprocess, Diagnostic, Engine, Severity};
use clap::{Args, ValueEnum};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

mod diff;
mod editor;
mod extensions;
mod external;
mod git;
mod import_layers;
mod judged;
mod lockfiles;
mod messages;
mod paths;
mod project_files;
mod settings_schema;

use diff::Diff;
pub use editor::{when_of, Editor};
use paths::{lint_paths, Linted};

#[derive(Args)]
pub struct LintArgs {
    /// Directory to walk, or explicit files (as prek passes them).
    paths: Vec<PathBuf>,
    #[command(flatten)]
    project: project::ProjectArgs,
    /// How to print the findings.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    output_format: OutputFormat,
    /// Leave out the paths named on the command line too when `extend-exclude` excludes them, or
    /// when they are inside a directory of a tool (`.venv`, `.claude`…) that `extend-include` does
    /// not ask for (by default only what the walk finds is left out).
    #[arg(long)]
    force_exclude: bool,
    /// Print how many findings each rule has instead of the findings (text and json formats).
    #[arg(long)]
    statistics: bool,
    /// Exit with 0 even when there are findings of severity error. Failures of the run still exit 2.
    #[arg(long)]
    exit_zero: bool,
    /// The lowest severity of a finding that exits with 1.
    #[arg(long, value_enum, default_value_t = FailOn::Error)]
    fail_on: FailOn,
    /// Extension command, e.g. "uv run --script ext.py".
    #[arg(long)]
    ext: Option<String>,
    /// Report the findings of a judge, read from this file (SARIF or the report of
    /// `abide check --json`), instead of linting: the ones of the active judgment rules, filtered
    /// by the configuration and the `ignore-file` comments.
    #[arg(long, conflicts_with_all = ["paths", "ext", "statistics"])]
    judged: Option<PathBuf>,
    /// Lint the files that changed since the merge base of this revision and `HEAD`, the changes
    /// not committed and the files git does not track included, and keep only the findings on the
    /// lines the change adds. The result cache is off.
    #[arg(long, value_name = "REV", group = "change")]
    diff: Option<String>,
    /// Like `--diff`, for the changes staged against `HEAD`: the files are read as they are staged.
    #[arg(long, group = "change")]
    staged: bool,
    /// Like `--diff`, for the changes of a unified diff, as `git diff` writes it, with paths
    /// relative to the working directory.
    #[arg(long, value_name = "PATCH", group = "change")]
    diff_file: Option<PathBuf>,
    /// Check the commit messages in these files, as git gives one to a `commit-msg` hook, with the
    /// rules of scope `commit` instead of linting files.
    #[arg(long, value_name = "FILE", num_args = 1.., group = "message",
          conflicts_with_all = ["paths", "judged", "change", "ext"])]
    commit_msg: Vec<PathBuf>,
    /// Like `--commit-msg`, for the message of each commit of `git log <RANGE>`.
    #[arg(long, value_name = "RANGE", group = "message",
          conflicts_with_all = ["paths", "judged", "change", "ext"])]
    commits: Option<String>,
    /// Like `--commit-msg`, for the title of a pull request: `--pr-title="$PR_TITLE"`.
    #[arg(long, value_name = "TITLE", group = "message",
          conflicts_with_all = ["paths", "judged", "change", "ext"])]
    pr_title: Option<String>,
    /// Files per `lint/files` request.
    #[arg(long, default_value_t = 256)]
    batch: usize,
    /// Skip the embedded ast-grep rules.
    #[arg(long)]
    no_astgrep: bool,
    /// Parse every file, even those the rules' derived prefilters rule out.
    #[arg(long)]
    no_prefilter: bool,
    /// Directory for the per-file result cache (off when omitted). One that botica creates ignores
    /// itself in git; add one that already exists to your `.gitignore`.
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    /// Keep the result cache in `.botica/cache` under the root of the project.
    #[arg(long, conflicts_with = "cache_dir")]
    cache: bool,
    /// Print to stderr how long each rule and each extension took, and what the prefilter saved.
    #[arg(long)]
    timings: bool,
    /// Extra ast-grep rules (`*.yml` in this directory), for the parity checks on other languages.
    #[arg(long, hide = true)]
    extra_rules: Option<PathBuf>,
    /// Parse whole files instead of only the top-level statements holding a candidate.
    #[arg(long, hide = true)]
    no_regions: bool,
    /// Match every node instead of skipping the subtrees without candidates.
    #[arg(long, hide = true)]
    no_subtree_skip: bool,
}

impl LintArgs {
    /// The arguments of a run over `paths` that reports and does not fail on the findings, for
    /// `botica rules eval`.
    pub fn for_eval(paths: Vec<PathBuf>, project: project::ProjectArgs) -> Self {
        Self {
            paths,
            exit_zero: true,
            ..Self::for_editor(project)
        }
    }

    /// The arguments of a run that lints no path: what an editor session builds its engine from.
    fn for_editor(project: project::ProjectArgs) -> Self {
        Self {
            paths: Vec::new(),
            project,
            output_format: OutputFormat::Text,
            force_exclude: false,
            statistics: false,
            exit_zero: false,
            fail_on: FailOn::Error,
            ext: None,
            judged: None,
            diff: None,
            staged: false,
            diff_file: None,
            commit_msg: Vec::new(),
            commits: None,
            pr_title: None,
            batch: 256,
            no_astgrep: false,
            no_prefilter: false,
            cache_dir: None,
            cache: false,
            timings: false,
            extra_rules: None,
            no_regions: false,
            no_subtree_skip: false,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Sarif,
    Github,
    Markdown,
}

#[derive(Clone, Copy, ValueEnum)]
enum FailOn {
    Error,
    Warning,
    Info,
    Hint,
}

impl From<FailOn> for Severity {
    fn from(arg: FailOn) -> Self {
        match arg {
            FailOn::Error => Severity::Error,
            FailOn::Warning => Severity::Warning,
            FailOn::Info => Severity::Info,
            FailOn::Hint => Severity::Hint,
        }
    }
}

impl From<OutputFormat> for Format {
    fn from(arg: OutputFormat) -> Self {
        match arg {
            OutputFormat::Text => Format::Text,
            OutputFormat::Json => Format::Json,
            OutputFormat::Sarif => Format::Sarif,
            OutputFormat::Github => Format::Github,
            OutputFormat::Markdown => Format::Markdown,
        }
    }
}

/// Everything the run needs that comes from the project, built once before any file is read.
struct Setup {
    engine: Engine,
    resolution: Arc<Resolution>,
    prefix: Option<String>,
    io_error_active: bool,
    config_key: String,
    with_astgrep: bool,
    /// Whether an active rule is an extension, so the run keeps the texts of the files to send.
    has_extensions: bool,
    /// Where the configuration was found (the working directory when there is none).
    project_root: PathBuf,
    /// The extension rules of the catalogs of the project.
    pool: ExtensionPool,
}

impl Setup {
    fn build(args: &LintArgs) -> Result<Self> {
        let project = project::load(&args.project)?;
        let project_root = project.loaded.root.clone();
        let launcher = Launcher::of(&project);
        let with_astgrep = !args.no_astgrep;
        let io_error_active = project.is_active(IO_ERROR);
        let prefix = project.working_prefix();
        let config_key = project.fingerprint();
        let natives = project::natives(&project);
        let resolution = Arc::new(project.resolution);
        let pool = ExtensionPool::new(
            Arc::clone(&resolution),
            prefix.clone(),
            launcher,
            project_root.clone(),
        );
        let has_extensions = resolution
            .entries()
            .iter()
            .any(|entry| entry.active && entry.rule.has_extension());
        let policy = project::suppression_policy(&resolution);
        let planner = project::planner(Arc::clone(&resolution), natives.index, prefix.clone());
        let exclusion = project::exclusion(Arc::clone(&resolution), prefix.clone());
        let inclusion = project::inclusion(Arc::clone(&resolution), prefix.clone());
        let engine = rules::build_engine(
            &rules::EngineOptions {
                astgrep: with_astgrep,
                resolution: &resolution,
                tuning: Tuning {
                    prefilter: !args.no_prefilter,
                    regions: !args.no_regions,
                    skip_subtrees: !args.no_subtree_skip,
                },
                extra_rules: args.extra_rules.as_deref(),
                timings: args.timings,
            },
            natives.rules,
            planner,
            policy,
            exclusion,
        )?;
        let engine = match inclusion {
            Some(inclusion) => engine.with_inclusion(inclusion),
            None => engine,
        };
        let engine = if has_extensions {
            engine.with_extension_paths(project::extension_paths(
                Arc::clone(&resolution),
                prefix.clone(),
            ))
        } else {
            engine
        };
        Ok(Self {
            engine,
            resolution,
            prefix,
            io_error_active,
            config_key,
            with_astgrep,
            has_extensions,
            project_root,
            pool,
        })
    }
}

pub fn run(args: LintArgs) -> Result<ExitCode> {
    let format = Format::from(args.output_format);
    check_statistics_format(&args, format)?;
    let total = Instant::now();

    let mut setup = Setup::build(&args)?;
    if messages::asked(&args) {
        let diagnostics = messages::findings(&args, &setup.resolution)?;
        write_report(&args, format, &setup, &diagnostics, &[])?;
        return Ok(exit_code(&[], &diagnostics, &args));
    }
    let diff = Diff::read(&args)?;
    if let Some(sarif) = &args.judged {
        return run_judged(&args, format, &setup, sarif, diff.as_ref());
    }
    let root = std::env::current_dir()?;
    let Linted {
        sources,
        mut diagnostics,
        io_errors,
        unreadable,
        files_read,
    } = lint_paths(&args, &mut setup, &root, diff.as_ref())?;
    if args.timings {
        print_prefilter_stats(&setup.engine);
    }

    let mut errors = read_errors(unreadable);
    if let Some(command) = &args.ext {
        errors.extend(run_extension(&args, command, &sources, &mut diagnostics));
    }
    if setup.has_extensions {
        let ran = extensions::run(&args, &setup, &sources);
        diagnostics.extend(ran.found);
        errors.extend(ran.errors);
    }
    diagnostics.extend(io_errors);
    diagnostics.extend(project_files::findings(
        &setup.resolution,
        &setup.project_root,
        setup.prefix.as_deref(),
    ));
    let mut diagnostics = postprocess::apply(
        diagnostics,
        project::verdicts(&setup.resolution, setup.prefix.as_deref()),
    );
    if let Some(diff) = &diff {
        diagnostics.retain(|diagnostic| diff.changes.keeps(diagnostic));
    }
    write_report(&args, format, &setup, &diagnostics, &errors)?;
    if args.timings {
        timings::print_rules(setup.engine.rule_times());
        eprintln!(
            "[timing] total {:.2} ms, {} files, {} diagnostics",
            total.elapsed().as_secs_f64() * 1e3,
            files_read.max(sources.len()),
            diagnostics.len()
        );
    }
    Ok(exit_code(&errors, &diagnostics, &args))
}

fn run_judged(
    args: &LintArgs,
    format: Format,
    setup: &Setup,
    sarif: &std::path::Path,
    diff: Option<&Diff>,
) -> Result<ExitCode> {
    let text = std::fs::read_to_string(sarif)
        .with_context(|| format!("cannot read `{}`", sarif.display()))?;
    let mut judged = judged::filter(&setup.resolution, setup.prefix.as_deref(), &text, |path| {
        std::fs::read_to_string(path).ok()
    })?;
    if let Some(diff) = diff {
        judged
            .diagnostics
            .retain(|diagnostic| diff.changes.keeps(diagnostic));
    }
    for rule in &judged.left_out {
        eprintln!(
            "botica: `{rule}` is not a judgment rule of the catalog: its finding is left out"
        );
    }
    write_report(args, format, setup, &judged.diagnostics, &[])?;
    Ok(exit_code(&[], &judged.diagnostics, args))
}

fn check_statistics_format(args: &LintArgs, format: Format) -> Result<()> {
    if args.statistics && !report::statistics::supports(format) {
        let name = args
            .output_format
            .to_possible_value()
            .map_or_else(String::new, |value| value.get_name().to_owned());
        anyhow::bail!("--statistics works with the text and json output formats, not {name}");
    }
    Ok(())
}

/// A file the walk found and could not read is a failure of the run, not a finding.
fn read_errors(unreadable: Vec<Unreadable>) -> Vec<RunError> {
    unreadable
        .into_iter()
        .map(|failure| RunError {
            kind: ErrorKind::Read,
            extension: None,
            path: Some(failure.path),
            message: failure.message,
        })
        .collect()
}

fn print_prefilter_stats(engine: &Engine) {
    let stats = engine.astgrep().stats();
    eprintln!(
        "[timing] prefilter: parsed {} files, skipped {}",
        stats.parsed, stats.skipped
    );
    eprintln!(
        "[timing] regions: {} files, {} fallbacks, {} suspect, parsed {:.1} of {:.1} MB; \
         {} files matched skipping subtrees",
        stats.region_files,
        stats.region_fallbacks,
        stats.region_suspects,
        stats.parsed_bytes as f64 / 1e6,
        stats.file_bytes as f64 / 1e6,
        stats.anchored_files
    );
}

/// Runs the extension over the texts, adds what it found to `diagnostics` and returns the
/// failures of the run, if any.
fn run_extension(
    args: &LintArgs,
    command: &str,
    sources: &[SourceFile],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<RunError> {
    let outcome = extension::run(command, sources, args.batch);
    if args.timings {
        timings::print_extension(command, &outcome.time);
    }
    diagnostics.extend(outcome.found);
    outcome
        .failure
        .map(|error| RunError {
            kind: ErrorKind::Extension,
            extension: Some(command.to_owned()),
            path: None,
            message: format!("{error:#}"),
        })
        .into_iter()
        .collect()
}

fn write_report(
    args: &LintArgs,
    format: Format,
    setup: &Setup,
    diagnostics: &[Diagnostic],
    errors: &[RunError],
) -> Result<()> {
    let rules = project::rule_docs(&setup.resolution);
    let report = Report {
        diagnostics,
        errors,
        rules: &rules,
        tool_version: env!("CARGO_PKG_VERSION"),
    };
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    if args.statistics {
        report::statistics::write(&report, format, &mut out)?;
    } else {
        report::write(&report, format, &mut out)?;
    }
    out.flush()?;
    if matches!(args.output_format, OutputFormat::Text) {
        for error in errors {
            eprintln!("botica: {error}");
        }
    }
    Ok(())
}

/// 2 when the run itself failed (it wins over everything), 1 when a finding has the severity of
/// `--fail-on` or a higher one, else 0. `--exit-zero` only forgives the findings.
fn exit_code(errors: &[RunError], diagnostics: &[Diagnostic], args: &LintArgs) -> ExitCode {
    let lowest = Severity::from(args.fail_on);
    if !errors.is_empty() {
        ExitCode::from(2)
    } else if !args.exit_zero && diagnostics.iter().any(|d| d.severity >= lowest) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
