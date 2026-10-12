use crate::{extension, project, rules, timings};
use anyhow::Result;
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

mod paths;

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
    /// Leave out the paths named on the command line too when `extend-exclude` excludes them
    /// (by default only what the walk finds is left out).
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
    /// Where the configuration was found (the working directory when there is none).
    project_root: PathBuf,
}

impl Setup {
    fn build(args: &LintArgs) -> Result<Self> {
        let project = project::load(&args.project)?;
        let project_root = project.loaded.root.clone();
        let with_astgrep = !args.no_astgrep && project.embedded_rules;
        let io_error_active = project.is_active(IO_ERROR);
        let prefix = project.working_prefix();
        let config_key = project.fingerprint();
        let natives = project::natives(&project);
        let resolution = Arc::new(project.resolution);
        let policy = project::suppression_policy(&resolution);
        let planner = project::planner(Arc::clone(&resolution), natives.index, prefix.clone());
        let exclusion = project::exclusion(Arc::clone(&resolution), prefix.clone());
        let inclusion = project::inclusion(Arc::clone(&resolution), prefix.clone());
        let engine = rules::build_engine(
            &rules::EngineOptions {
                astgrep: with_astgrep,
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
        Ok(Self {
            engine,
            resolution,
            prefix,
            io_error_active,
            config_key,
            with_astgrep,
            project_root,
        })
    }
}

pub fn run(args: LintArgs) -> Result<ExitCode> {
    let format = Format::from(args.output_format);
    check_statistics_format(&args, format)?;
    let total = Instant::now();

    let setup = Setup::build(&args)?;
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
    diagnostics.extend(io_errors);
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
