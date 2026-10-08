mod rules;

use anyhow::Result;
use botica_core::cache::{lint_files_cached, Cache};
use botica_core::walk::Schedule;
use botica_core::{walk, Diagnostic, ExtensionRun, Severity};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
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
}

#[derive(Args)]
struct LintArgs {
    /// Directory to walk, or explicit files (as prek passes them).
    paths: Vec<PathBuf>,
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
    let Cli {
        command: Command::Lint(args),
    } = Cli::parse();
    match lint(args) {
        Ok(true) => ExitCode::from(1),
        Ok(false) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("botica: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn lint(args: LintArgs) -> Result<bool> {
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
    let engine = rules::build_engine(&rules::EngineOptions {
        astgrep: with_astgrep,
        prefilter: !args.no_prefilter,
        extra_rules: args.extra_rules.as_deref(),
        describe_prefilter: args.describe_prefilter,
        regions: !args.no_regions,
        subtree_skip: !args.no_subtree_skip,
    })?;
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
    for path in &paths {
        if !path.is_dir() {
            files.push(path.clone());
        } else if overlapped {
            let (read, found) =
                walk::walk_and_lint(&engine, &root, path, &|p| engine.astgrep().covers(p));
            files_read += read;
            overlapped_found.extend(found);
        } else {
            files.extend(walk::discover(path, &|p| engine.astgrep().covers(p)));
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
                &rules::fingerprint(with_astgrep, args.extra_rules.as_deref())?,
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

    if let Some(command) = args.ext {
        let command: Vec<String> = command.split_whitespace().map(str::to_owned).collect();
        let mut run = ExtensionRun::start(&command)?;
        mark("ext start");
        diagnostics.extend(run.lint(
            sources.iter().map(|f| (f.path.as_str(), f.text.as_str())),
            args.batch,
        )?);
        mark("ext lint");
        run.finish()?;
        mark("ext shutdown");
    }

    report(&diagnostics)?;
    if timings {
        eprintln!(
            "[timing] total {:.2} ms, {} files, {} diagnostics",
            total.elapsed().as_secs_f64() * 1e3,
            files_read.max(sources.len()),
            diagnostics.len()
        );
    }
    Ok(diagnostics.iter().any(|d| d.severity == Severity::Error))
}

fn report(diagnostics: &[Diagnostic]) -> Result<()> {
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for diagnostic in diagnostics {
        writeln!(out, "{diagnostic}")?;
    }
    out.flush()?;
    Ok(())
}
