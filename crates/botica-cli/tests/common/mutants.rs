//! A project whose `src/lib.rs` the mutants of a report of cargo-mutants change, and the outcomes
//! of that report as cargo-mutants 27.1.0 writes them in `mutants.out/outcomes.json`
//! (`src/outcome.rs`, `src/mutant.rs`).

use serde_json::{json, Value};
use std::path::PathBuf;

use super::{project_in, write};

pub const SELECTED: &str = "preview = true\nselect = [\"mutant-survived\"]\n";

/// The source that the mutants of the report change.
pub const LIB: &str = "pub fn is_even(n: u32) -> bool {\n    n % 2 == 0\n}\n\n\
                       pub fn double(n: u32) -> u32 {\n    n * 2\n}\n";

/// One outcome of a mutant of `src/lib.rs`.
pub fn mutant(change: &str, start: (u32, u32), end: (u32, u32), summary: &str) -> Value {
    json!({
        "scenario": {"Mutant": {
            "name": format!("src/lib.rs:{}:{}: {change}", start.0, start.1),
            "package": "demo",
            "file": "src/lib.rs",
            "function": null,
            "span": {
                "start": {"line": start.0, "column": start.1},
                "end": {"line": end.0, "column": end.1}
            },
            "replacement": "",
            "genre": "BinaryOperator"
        }},
        "summary": summary,
        "log_path": "log/x.log",
        "diff_path": "diff/x.diff",
        "phase_results": []
    })
}

/// A report with the baseline, two mutants that survived and one of each other outcome.
pub fn outcomes() -> String {
    json!({
        "outcomes": [
            {"scenario": "Baseline", "summary": "Success", "log_path": "log/baseline.log",
             "diff_path": null, "phase_results": []},
            mutant("replace == with != in is_even", (2, 11), (2, 13), "MissedMutant"),
            mutant("replace double -> u32 with 0", (5, 30), (7, 2), "MissedMutant"),
            mutant("replace * with + in double", (6, 7), (6, 8), "CaughtMutant"),
            mutant("replace is_even -> bool with false", (1, 32), (3, 2), "Timeout"),
            mutant("replace double -> u32 with 1", (6, 5), (6, 10), "Unviable"),
        ],
        "total_mutants": 5, "missed": 2, "caught": 1, "timeout": 1, "unviable": 1, "success": 0,
        "start_time": "2026-11-12T10:00:00Z", "end_time": "2026-11-12T10:00:30Z",
        "cargo_mutants_version": "27.1.0"
    })
    .to_string()
}

/// A project in `<scratch>/lint-mutants/<name>` with `config` as `botica.toml`, `lib` as
/// `src/lib.rs` and the report of [`outcomes`] as `outcomes.json`.
pub fn project(name: &str, config: Option<&str>, lib: &str) -> PathBuf {
    let dir = project_in("lint-mutants", name, config);
    write(&dir, "src/lib.rs", lib);
    write(&dir, "outcomes.json", &outcomes());
    dir
}
