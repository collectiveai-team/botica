mod rules;

use anyhow::Result;
use botica_core::{walk, Diagnostic, ExtensionRun, Severity};
use clap::{Parser, Subcommand};
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
    Lint {
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
        /// Print phase timings to stderr.
        #[arg(long)]
        timings: bool,
    },
}

fn main() -> ExitCode {
    let Cli { command } = Cli::parse();
    let Command::Lint {
        paths,
        ext,
        batch,
        no_astgrep,
        timings,
    } = command;
    match lint(paths, ext, batch, !no_astgrep, timings) {
        Ok(true) => ExitCode::from(1),
        Ok(false) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("botica: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn lint(
    paths: Vec<PathBuf>,
    ext: Option<String>,
    batch: usize,
    with_astgrep: bool,
    timings: bool,
) -> Result<bool> {
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

    let engine = rules::build_engine(with_astgrep)?;
    mark("compile rules");

    let root = std::env::current_dir()?;
    let paths = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths
    };
    let mut files = Vec::new();
    for path in &paths {
        if path.is_dir() {
            files.extend(walk::discover(path));
        } else {
            files.push(path.clone());
        }
    }
    mark("walk");
    let sources = walk::read_all(&root, &files);
    mark("read");

    let mut diagnostics = walk::lint_files(&engine, &sources);
    mark("native+ast-grep");

    if let Some(command) = ext {
        let command: Vec<String> = command.split_whitespace().map(str::to_owned).collect();
        let mut run = ExtensionRun::start(&command)?;
        mark("ext start");
        diagnostics.extend(run.lint(
            sources.iter().map(|f| (f.path.as_str(), f.text.as_str())),
            batch,
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
            sources.len(),
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
