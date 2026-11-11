//! The local history of findings (ADR 0019): `runs.jsonl` and `findings.jsonl` in
//! `.botica/history/` under the root of the project, to which each recorded run appends.

use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The version of the lines that this writer appends.
pub const VERSION: u32 = 1;

/// How long a run waits for another run to finish appending.
const LOCK_WAIT: Duration = Duration::from_secs(10);

/// One line of `runs.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunLine {
    pub v: u32,
    pub run_id: String,
    pub started_at: u64,
    pub commit: Option<String>,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub botica: String,
    pub rules_fingerprint: String,
    pub key_id: Option<String>,
    pub mode: String,
    pub origin: String,
    pub rules: BTreeMap<String, RuleEntry>,
}

/// What a run says of one rule that it evaluated. The findings that a suppression channel
/// silenced are not counted yet, so the entry has no `suppressed` key.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RuleEntry {
    pub hits: usize,
    pub baselined: usize,
    /// What can change the findings of the rule; `null` until botica computes it, which leaves
    /// the rule out of every comparison.
    pub hash: Option<String>,
}

/// One line of `findings.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FindingLine {
    pub v: u32,
    pub run_id: String,
    pub code: Option<String>,
    pub slug: String,
    pub severity: String,
    pub path: String,
    pub line: usize,
    pub fingerprint: String,
    pub occurrence: usize,
}

/// A name for a run that no other run of this or another process takes: 16 hex digits of the
/// blake3 of the start time in nanoseconds, the process id and a counter of the process.
pub fn run_id(started: SystemTime) -> String {
    static RUNS: AtomicU64 = AtomicU64::new(0);
    let nanos = started
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let mut hasher = blake3::Hasher::new();
    hasher.update(&nanos.to_le_bytes());
    hasher.update(&std::process::id().to_le_bytes());
    hasher.update(&RUNS.fetch_add(1, Ordering::Relaxed).to_le_bytes());
    hasher.finalize().to_hex()[..16].to_owned()
}

/// Unix seconds of `time`.
pub fn unix_seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// Sets the `occurrence` of each finding: 0 for the first finding of the run with its
/// fingerprint, 1 for the next, in order of path and line.
pub fn number_occurrences(findings: &mut [FindingLine]) {
    findings.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    let mut seen: HashMap<String, usize> = HashMap::new();
    for finding in findings {
        let count = seen.entry(finding.fingerprint.clone()).or_default();
        finding.occurrence = *count;
        *count += 1;
    }
}

/// Appends the findings and then the run to the history of the project at `root`, under an
/// exclusive lock so that the lines of two runs do not interleave.
pub fn append(root: &Path, run: &RunLine, findings: &[FindingLine]) -> Result<()> {
    let dir = root.join(".botica").join("history");
    directory(&root.join(".botica"))?;
    directory(&dir)?;
    ignore_itself(&dir)?;
    let lock = open(
        &dir.join("lock"),
        OpenOptions::new().read(true).write(true).create(true),
    )?;
    wait_for(&lock)?;
    let mut lines = Vec::new();
    for finding in findings {
        serde_json::to_writer(&mut lines, finding)?;
        lines.push(b'\n');
    }
    if !lines.is_empty() {
        append_to(&dir.join("findings.jsonl"), &lines)?;
    }
    let mut line = serde_json::to_vec(run)?;
    line.push(b'\n');
    append_to(&dir.join("runs.jsonl"), &line)
}

/// Creates the directory `dir`, or checks that it is one and not a symbolic link.
fn directory(dir: &Path) -> Result<()> {
    match std::fs::symlink_metadata(dir) {
        Ok(found) if found.file_type().is_symlink() => {
            bail!("{} is a symbolic link", dir.display())
        }
        Ok(found) if !found.is_dir() => bail!("{} is not a directory", dir.display()),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match std::fs::create_dir(dir) {
                Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
                    Err(error).with_context(|| format!("cannot create {}", dir.display()))
                }
                _ => directory(dir),
            }
        }
        Err(error) => Err(error).with_context(|| format!("cannot read {}", dir.display())),
    }
}

/// Makes the `.gitignore` of the history end with a line `*`, which keeps the history out of git
/// whatever the lines above it say, and leaves one that already ends so as it is.
fn ignore_itself(dir: &Path) -> Result<()> {
    let path = dir.join(".gitignore");
    let mut file = open(
        &path,
        OpenOptions::new().read(true).append(true).create(true),
    )?;
    let mut text = String::new();
    file.read_to_string(&mut text)
        .with_context(|| format!("cannot read {}", path.display()))?;
    if text
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        == Some("*")
    {
        return Ok(());
    }
    let line: &[u8] = if text.is_empty() || text.ends_with('\n') {
        b"*\n"
    } else {
        b"\n*\n"
    };
    file.write_all(line)
        .with_context(|| format!("cannot write {}", path.display()))
}

/// Takes the exclusive lock of `lock`, waiting at most [`LOCK_WAIT`] for another run.
fn wait_for(lock: &File) -> Result<()> {
    let start = Instant::now();
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) if start.elapsed() < LOCK_WAIT => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(std::fs::TryLockError::WouldBlock) => {
                bail!(
                    "another run held the lock of the history for {} seconds",
                    LOCK_WAIT.as_secs()
                )
            }
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(error).context("cannot lock the history")
            }
        }
    }
}

/// Appends `lines` to the file at `path` in one write, after a `\n` when the file does not end
/// with one.
fn append_to(path: &Path, lines: &[u8]) -> Result<()> {
    let mut file = open(
        path,
        OpenOptions::new().read(true).append(true).create(true),
    )?;
    let mut text = Vec::with_capacity(lines.len() + 1);
    if file.metadata()?.len() > 0 {
        let mut last = [0u8];
        file.seek(SeekFrom::End(-1))?;
        file.read_exact(&mut last)?;
        if last[0] != b'\n' {
            text.push(b'\n');
        }
    }
    text.extend_from_slice(lines);
    file.write_all(&text)
        .with_context(|| format!("cannot write {}", path.display()))
}

/// Opens `path` without following a symbolic link, and refuses what is not a regular file.
fn open(path: &Path, options: &mut OpenOptions) -> Result<File> {
    no_follow(options);
    #[cfg(not(unix))]
    if std::fs::symlink_metadata(path).is_ok_and(|found| found.file_type().is_symlink()) {
        bail!("{} is a symbolic link", path.display());
    }
    let file = options
        .open(path)
        .with_context(|| format!("cannot open {}", path.display()))?;
    if !file.metadata()?.is_file() {
        bail!("{} is not a regular file", path.display());
    }
    Ok(file)
}

/// `O_NOFOLLOW` makes the open of a symbolic link fail, and `O_NONBLOCK` keeps the open of a FIFO
/// from waiting for a reader; on a regular file it changes nothing.
#[cfg(unix)]
fn no_follow(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
}

#[cfg(not(unix))]
fn no_follow(_options: &mut OpenOptions) {}
