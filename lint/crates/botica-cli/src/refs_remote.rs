//! The files of external references: read with the `git` of the user at the commit of the
//! reference, and kept in a cache with one file per repository, commit and path.

use std::fs;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::refs_collection::relative;

/// What one command of git may take. `BOTICA_REFS_TIMEOUT`, in seconds, replaces it.
const DEADLINE: Duration = Duration::from_secs(60);
const DEADLINE_VARIABLE: &str = "BOTICA_REFS_TIMEOUT";
const POLL: Duration = Duration::from_millis(20);

pub struct Remote {
    cache: PathBuf,
    deadline: Duration,
}

/// The remote that keeps its files in `cache_dir`, or in `botica/refs` of the cache of the user.
pub fn open(cache_dir: Option<&Path>) -> Result<Remote, String> {
    let cache = match cache_dir {
        Some(dir) => dir.to_path_buf(),
        None => default_cache()?,
    };
    Ok(Remote {
        cache,
        deadline: deadline(),
    })
}

fn default_cache() -> Result<PathBuf, String> {
    let set = |name: &str| {
        std::env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let base = set("XDG_CACHE_HOME")
        .or_else(|| set("LOCALAPPDATA").filter(|_| cfg!(windows)))
        .or_else(|| set("HOME").map(|home| home.join(".cache")))
        .ok_or(
            "there is no place for the cache: set XDG_CACHE_HOME or HOME, or pass --cache-dir",
        )?;
    Ok(base.join("botica").join("refs"))
}

fn deadline() -> Duration {
    std::env::var(DEADLINE_VARIABLE)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map_or(DEADLINE, Duration::from_secs)
}

impl Remote {
    /// The text of `path` in `repo` at the commit `rev`: from the cache, or fetched and kept.
    pub fn text(&self, repo: &str, rev: &str, path: &str) -> Result<String, String> {
        let file = self
            .cache
            .join(repo)
            .join(rev)
            .join(relative(path, "the repository")?);
        if let Ok(text) = fs::read_to_string(&file) {
            return Ok(text);
        }
        let text = self.fetch(repo, rev, path)?;
        keep(&file, &text)?;
        Ok(text)
    }

    fn fetch(&self, repo: &str, rev: &str, path: &str) -> Result<String, String> {
        let scratch = Scratch::in_cache(&self.cache)?;
        let git_dir = scratch.0.to_string_lossy().into_owned();
        let url = format!("https://{repo}.git");
        self.git(&["init", "--bare", "--quiet", &git_dir])
            .map_err(|reason| format!("cannot fetch {repo}@{rev}: {reason}"))?;
        self.git(&[
            "--git-dir",
            &git_dir,
            "fetch",
            "--quiet",
            "--depth",
            "1",
            "--filter=blob:none",
            "--",
            &url,
            rev,
        ])
        .map_err(|reason| format!("cannot fetch {repo}@{rev}: {reason}"))?;
        let listing = self
            .git(&["--git-dir", &git_dir, "ls-tree", rev, "--", path])
            .map_err(|reason| format!("cannot read {repo}@{rev}: {reason}"))?;
        let kind = String::from_utf8_lossy(&listing)
            .split_whitespace()
            .nth(1)
            .map(str::to_owned);
        if kind.as_deref() != Some("blob") {
            return Err(format!("`{path}` is not a file in {repo}@{rev}"));
        }
        let bytes = self
            .git(&["--git-dir", &git_dir, "show", &format!("{rev}:{path}")])
            .map_err(|reason| format!("cannot read `{path}` of {repo}@{rev}: {reason}"))?;
        String::from_utf8(bytes).map_err(|_| format!("`{path}` is not UTF-8 text in {repo}@{rev}"))
    }

    /// Runs `git`, never asking for a password, and returns what it printed. A command that
    /// outlasts the deadline is killed.
    fn git(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        let mut child = Command::new("git")
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| match error.kind() {
                ErrorKind::NotFound => {
                    "git is needed to read external references and it is not on the PATH".to_owned()
                }
                _ => format!("cannot run git: {error}"),
            })?;
        let stdout = drain(child.stdout.take());
        let stderr = drain(child.stderr.take());
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() >= self.deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "git did not answer within {}s and was killed",
                        self.deadline.as_secs()
                    ));
                }
                Ok(None) => thread::sleep(POLL),
                Err(error) => return Err(format!("cannot wait for git: {error}")),
            }
        };
        let output = stdout.join().unwrap_or_default();
        if status.success() {
            return Ok(output);
        }
        let message = String::from_utf8_lossy(&stderr.join().unwrap_or_default()).into_owned();
        let reason = message.lines().find(|line| !line.trim().is_empty());
        Err(reason.map_or_else(|| format!("git {status}"), |line| line.trim().to_owned()))
    }
}

fn drain<R: Read + Send + 'static>(stream: Option<R>) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut stream) = stream {
            let _ = stream.read_to_end(&mut bytes);
        }
        bytes
    })
}

/// Writes the file whole or not at all: another run never reads half of it.
fn keep(file: &Path, text: &str) -> Result<(), String> {
    let parent = file.parent().unwrap_or(file);
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create `{}`: {error}", parent.display()))?;
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let partial = file.with_file_name(format!("{name}.partial-{}", std::process::id()));
    fs::write(&partial, text)
        .and_then(|()| fs::rename(&partial, file))
        .map_err(|error| format!("cannot write `{}`: {error}", file.display()))
}

/// A directory of the cache for one fetch, removed when it goes out of scope.
struct Scratch(PathBuf);

impl Scratch {
    fn in_cache(cache: &Path) -> Result<Self, String> {
        fs::create_dir_all(cache)
            .map_err(|error| format!("cannot create `{}`: {error}", cache.display()))?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        Ok(Self(
            cache.join(format!(".fetch-{}-{nanos}", std::process::id())),
        ))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
