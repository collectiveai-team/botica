use crate::extension_pool::ExtensionPool;
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

mod extensions;
mod external;
mod judged;
mod paths;
mod settings_schema;

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
    /// Extension command, e.g. "uv run --script ext.py".
    #[arg(long)]
    ext: Option<String>,
    /// Report the findings of a judge, read from this SARIF file, instead of linting: the ones of
    /// the active judgment rules, filtered by the configuration and the `ignore-file` comments.
    #[arg(long, conflicts_with_all = ["paths", "ext", "statistics"])]
    judged: Option<PathBuf>,
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
    /// The arguments of a run that lints no path: what an editor session builds its engine from.
    fn for_editor(project: project::ProjectArgs) -> Self {
        Self {
            paths: Vec::new(),
            project,
            output_format: OutputFormat::Text,
            force_exclude: false,
            statistics: false,
            exit_zero: false,
            ext: None,
            judged: None,
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

/// What lints the buffers of an editor: the engine of a run, built once. `lint` runs the rules of
/// the engine (native and ast-grep); the extension rules are the processes of `extensions`.
pub struct Editor {
    setup: Setup,
}

impl Editor {
    pub fn build(project: project::ProjectArgs) -> Result<Self> {
        Ok(Self {
            setup: Setup::build(&LintArgs::for_editor(project))?,
        })
    }

    /// The findings in `text`, the content of the file `path`, given from the working directory.
    pub fn lint(&self, path: &str, text: &str) -> Vec<Diagnostic> {
        self.setup.engine.lint_source(path, text)
    }

    /// The extension rules of the project.
    pub const fn extensions(&self) -> &ExtensionPool {
        &self.setup.pool
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Sarif,
    Github,
}

impl From<OutputFormat> for Format {
    fn from(arg: OutputFormat) -> Self {
        match arg {
            OutputFormat::Text => Format::Text,
            OutputFormat::Json => Format::Json,
            OutputFormat::Sarif => Format::Sarif,
            OutputFormat::Github => Format::Github,
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
    /// The extension rules of the project.
    pool: ExtensionPool,
}

impl Setup {
    fn build(args: &LintArgs) -> Result<Self> {
        let project = project::load(&args.project)?;
        let project_root = project.loaded.root.clone();
        let catalog_dirs = project.catalog_dirs.clone();
        let with_astgrep = !args.no_astgrep;
        let io_error_active = project.is_active(IO_ERROR);
        let prefix = project.working_prefix();
        let config_key = project.fingerprint();
        let natives = project::natives(&project);
        let resolution = Arc::new(project.resolution);
        let pool = ExtensionPool::new(
            Arc::clone(&resolution),
            prefix.clone(),
            catalog_dirs,
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

    let setup = Setup::build(&args)?;
    if let Some(sarif) = &args.judged {
        return run_judged(&args, format, &setup, sarif);
    }
    let root = std::env::current_dir()?;
    let Linted {
        sources,
        mut diagnostics,
        io_errors,
        unreadable,
        files_read,
    } = lint_paths(&args, &setup, &root)?;
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
    diagnostics.extend(external::findings(
        &setup.resolution,
        &setup.project_root,
        setup.prefix.as_deref(),
    ));
    diagnostics.extend(settings_schema::findings(
        &setup.resolution,
        &setup.project_root,
        setup.prefix.as_deref(),
    ));
    let diagnostics = postprocess::apply(
        diagnostics,
        project::verdicts(&setup.resolution, setup.prefix.as_deref()),
    );
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
    Ok(exit_code(&errors, &diagnostics, args.exit_zero))
}

fn run_judged(
    args: &LintArgs,
    format: Format,
    setup: &Setup,
    sarif: &std::path::Path,
) -> Result<ExitCode> {
    let text = std::fs::read_to_string(sarif)
        .with_context(|| format!("cannot read `{}`", sarif.display()))?;
    let judged = judged::filter(&setup.resolution, setup.prefix.as_deref(), &text, |path| {
        std::fs::read_to_string(path).ok()
    })?;
    for rule in &judged.left_out {
        eprintln!(
            "botica: `{rule}` is not a judgment rule of the catalog: its finding is left out"
        );
    }
    write_report(args, format, setup, &judged.diagnostics, &[])?;
    Ok(exit_code(&[], &judged.diagnostics, args.exit_zero))
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

/// 2 when the run itself failed (it wins over everything), 1 when a finding is an error, else 0.
/// `--exit-zero` only forgives the findings.
fn exit_code(errors: &[RunError], diagnostics: &[Diagnostic], exit_zero: bool) -> ExitCode {
    if !errors.is_empty() {
        ExitCode::from(2)
    } else if !exit_zero && diagnostics.iter().any(|d| d.severity == Severity::Error) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
