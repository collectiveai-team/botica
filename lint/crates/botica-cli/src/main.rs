mod project;
mod rules;
mod rules_cmd;

use anyhow::Result;
use botica_core::cache::{lint_files_cached, Cache};
use botica_core::report::{self, ErrorKind, Format, Report, RunError};
use botica_core::walk::Schedule;
use botica_core::{postprocess, walk, Diagnostic, ExtensionRun, Severity};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "botica", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Lint(LintArgs),
    /// Show which rules are active and why.
    Rules(rules_cmd::RulesArgs),
}

#[derive(Args)]
struct LintArgs {
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
    /// Directory for the per-file result cache (off when omitted).
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    /// Print phase timings to stderr.
    #[arg(long)]
    timings: bool,
    /// Measurement aid: extra ast-grep rules (`*.yml` in this directory).
    #[arg(long, hide = true)]
    extra_rules: Option<PathBuf>,
    /// Measurement aid: print each rule's effective prefilter to stderr.
    #[arg(long, hide = true)]
    describe_prefilter: bool,
    /// Measurement aid: how files are handed to the worker threads.
    #[arg(long, hide = true, value_enum, default_value_t = ScheduleArg::InOrder)]
    schedule: ScheduleArg,
    /// Parse whole files instead of only the top-level statements holding a candidate.
    #[arg(long, hide = true)]
    no_regions: bool,
    /// Match every node instead of skipping the subtrees without candidates.
    #[arg(long, hide = true)]
    no_subtree_skip: bool,
    /// Measurement aid: finish the walk before reading and linting any file.
    #[arg(long, hide = true)]
    separate_walk: bool,
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

#[derive(Clone, Copy, ValueEnum)]
enum ScheduleArg {
    InOrder,
    LargestFirst,
    /// One `[file]` timing row per file on stderr.
    Timed,
}

impl From<ScheduleArg> for Schedule {
    fn from(arg: ScheduleArg) -> Self {
        match arg {
            ScheduleArg::InOrder => Schedule::InOrder,
            ScheduleArg::LargestFirst => Schedule::LargestFirst,
            ScheduleArg::Timed => Schedule::Timed,
        }
    }
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Lint(args) => lint(args),
        Command::Rules(args) => rules_cmd::run(&args).map(|()| ExitCode::SUCCESS),
    };
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("botica: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn lint(args: LintArgs) -> Result<ExitCode> {
    let timings = args.timings;
    let total = Instant::now();
    let mut lap = Instant::now();
    let mut mark = |label: &str| {
        if timings {
            eprintln!(
                "[timing] {label:<18} {:>8.2} ms",
                lap.elapsed().as_secs_f64() * 1e3
            );
        }
        lap = Instant::now();
    };

    let with_astgrep = !args.no_astgrep;
    let project = project::load(&args.project)?;
    let io_error_active = project.is_active("io-error");
    let prefix = project.working_prefix();
    let config_key = project.fingerprint();
    let natives = project::natives(&project.resolution);
    let resolution = Arc::new(project.resolution);
    let policy = project::suppression_policy(&resolution);
    let planner = project::planner(Arc::clone(&resolution), natives.index, prefix.clone());
    let exclusion = project::exclusion(Arc::clone(&resolution), prefix.clone());
    let engine = rules::build_engine(
        &rules::EngineOptions {
            astgrep: with_astgrep,
            prefilter: !args.no_prefilter,
            extra_rules: args.extra_rules.as_deref(),
            describe_prefilter: args.describe_prefilter,
            regions: !args.no_regions,
            subtree_skip: !args.no_subtree_skip,
        },
        natives.rules,
        planner,
        policy,
        exclusion,
    )?;
    mark("compile rules");

    let root = std::env::current_dir()?;
    let paths = if args.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.paths
    };
    let fused = args.cache_dir.is_none() && matches!(args.schedule, ScheduleArg::InOrder);
    let streaming = fused && args.ext.is_none();
    let overlapped = streaming && !args.separate_walk;
    let mut files = Vec::new();
    let mut files_read = 0;
    let mut overlapped_found = Vec::new();
    let mut io_errors = Vec::new();
    for path in &paths {
        if args.force_exclude && named_and_excluded(&engine, &root, path) {
            continue;
        }
        if !path.is_dir() {
            match std::fs::File::open(path) {
                Ok(_) => files.push(path.clone()),
                Err(error) if io_error_active => io_errors.push(io_error(path, &error)),
                Err(_) => {}
            }
        } else if overlapped {
            let (read, found) = walk::walk_and_lint(&engine, &root, path);
            files_read += read;
            overlapped_found.extend(found);
        } else {
            files.extend(walk::discover(path, &root, &engine));
        }
    }
    mark(if overlapped { "walk+read+lint" } else { "walk" });
    let (sources, mut diagnostics) = if streaming {
        let (read, mut found) = walk::lint_streaming(&engine, &root, &files);
        files_read += read;
        found.append(&mut overlapped_found);
        found.sort_by(|a, b| (&a.path, a.start, &a.rule).cmp(&(&b.path, b.start, &b.rule)));
        (Vec::new(), found)
    } else if fused {
        walk::read_and_lint(&engine, &root, &files)
    } else {
        let sources = walk::read_all(&root, &files);
        mark("read");
        let found = if let Some(dir) = &args.cache_dir {
            let mut cache = Cache::load(
                dir,
                &rules::fingerprint(with_astgrep, args.extra_rules.as_deref(), &config_key)?,
            );
            let (found, stats) = lint_files_cached(&engine, &sources, &mut cache);
            cache.save()?;
            if timings {
                eprintln!(
                    "[timing] cache: {} hits, {} misses",
                    stats.hits, stats.misses
                );
            }
            found
        } else {
            walk::lint_files(&engine, &sources, args.schedule.into())
        };
        (sources, found)
    };
    mark(if fused {
        "read+lint"
    } else {
        "native+ast-grep"
    });
    if timings {
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
        let [pre, regions, whole, partial, matching, drop, lex, members, positions] =
            stats.phase_nanos.map(|n| n as f64 / 1e6);
        eprintln!(
            "[timing] cpu: prefilter {pre:.0} ms, regions {:.0} ms (positions {positions:.0}, \
             lex {lex:.0}, members {members:.0}), parse {:.0} ms (whole {whole:.0}, regions \
             {partial:.0}), match {matching:.0} ms, free tree {drop:.0} ms",
            positions + regions - partial,
            whole + partial
        );
    }

    let mut errors = Vec::new();
    if let Some(command) = &args.ext {
        let (found, failure) = run_extension(command, &sources, args.batch, &mut mark);
        diagnostics.extend(found);
        errors.extend(failure.map(|error| RunError {
            kind: ErrorKind::Extension,
            extension: Some(command.clone()),
            message: format!("{error:#}"),
        }));
    }

    diagnostics.extend(io_errors);
    let diagnostics = postprocess::apply(
        diagnostics,
        project::verdicts(&resolution, prefix.as_deref()),
    );
    let rules = project::rule_docs(&resolution);
    let report = Report {
        diagnostics: &diagnostics,
        errors: &errors,
        rules: &rules,
        tool_version: env!("CARGO_PKG_VERSION"),
    };
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    report::write(&report, args.output_format.into(), &mut out)?;
    out.flush()?;
    if matches!(args.output_format, OutputFormat::Text) {
        for error in &errors {
            eprintln!("botica: {error}");
        }
    }
    if timings {
        eprintln!(
            "[timing] total {:.2} ms, {} files, {} diagnostics",
            total.elapsed().as_secs_f64() * 1e3,
            files_read.max(sources.len()),
            diagnostics.len()
        );
    }
    Ok(exit_code(&errors, &diagnostics, args.exit_zero))
}

/// Whether `extend-exclude` leaves out a path the user named. `.` and the like are never excluded.
fn named_and_excluded(engine: &botica_core::Engine, root: &Path, path: &Path) -> bool {
    let named = walk::relative_path(root, path);
    !named.is_empty() && engine.skips_path(&named)
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

/// What the extension found, and why it stopped if it did not finish cleanly. A failure never
/// takes down what was already found.
fn run_extension(
    command: &str,
    sources: &[walk::SourceFile],
    batch: usize,
    mark: &mut dyn FnMut(&str),
) -> (Vec<Diagnostic>, Option<anyhow::Error>) {
    let argv: Vec<String> = command.split_whitespace().map(str::to_owned).collect();
    let mut run = match ExtensionRun::start(&argv) {
        Ok(run) => run,
        Err(error) => return (Vec::new(), Some(error)),
    };
    mark("ext start");
    let files = sources.iter().map(|f| (f.path.as_str(), f.text.as_str()));
    let found = match run.lint(files, batch) {
        Ok(found) => found,
        Err(error) => return (Vec::new(), Some(error)),
    };
    mark("ext lint");
    let failure = run.finish().err();
    mark("ext shutdown");
    (found, failure)
}

fn io_error(path: &Path, error: &std::io::Error) -> Diagnostic {
    Diagnostic {
        path: path.display().to_string(),
        rule: "io-error".to_owned(),
        severity: Severity::Error,
        start: 0,
        end: 0,
        line: 1,
        column: 1,
        end_line: 1,
        end_column: 1,
        file_level: true,
        message: error.to_string(),
    }
}
