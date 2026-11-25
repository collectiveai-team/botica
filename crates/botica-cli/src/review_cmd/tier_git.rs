use std::ffi::OsString;
use std::fs;
use std::io::{Read, Seek, Write};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{bail, ensure, Context, Result};

#[path = "tier_diff.rs"]
mod diff;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Object-only Git boundary. The isolated repository has no user configuration, index or hooks.
pub(crate) struct Git {
    dir: PathBuf,
    objects: OsString,
    pub root: PathBuf,
    pub base: String,
    pub head: String,
    pub shallow: bool,
}

fn command() -> Command {
    let mut command = Command::new("git");
    let keys: Vec<_> = std::env::vars_os()
        .filter(|(key, _)| key.to_string_lossy().starts_with("GIT_"))
        .map(|(key, _)| key)
        .collect();
    for key in keys {
        command.env_remove(key);
    }
    command
        .env_remove("OPENAI_API_KEY")
        .env_remove("NVIDIA_API_KEY")
        .env_remove("TYPESAFE_AI_API_KEY")
        .env_remove("BOTICA_HISTORY_KEY")
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C");
    command
}

fn checked(output: Output) -> Result<Vec<u8>> {
    ensure!(
        output.status.success(),
        "Git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

fn text(bytes: Vec<u8>) -> Result<String> {
    Ok(String::from_utf8(bytes)
        .context("Git metadata is not UTF-8")?
        .trim_end_matches('\n')
        .to_owned())
}

fn original(args: &[&str]) -> Result<Vec<u8>> {
    checked(command().args(args).output().context("run Git")?)
}

fn pin(value: &str, flag: &str) -> Result<String> {
    ensure!(
        !value.is_empty() && !value.starts_with('-'),
        "invalid {flag} reference"
    );
    let split = value.find(['~', '^']).unwrap_or(value.len());
    let (name, suffix) = value.split_at(split);
    ensure!(
        suffix
            .chars()
            .all(|c| c.is_ascii_digit() || c == '~' || c == '^'),
        "unsupported {flag} revision suffix"
    );
    let oid = (name.len() == 40 || name.len() == 64) && name.bytes().all(|b| b.is_ascii_hexdigit());
    let mut expression = value.to_owned();
    if !name.starts_with("refs/") && !oid {
        let mut found = Vec::new();
        for prefix in ["refs/heads/", "refs/remotes/", "refs/tags/", "refs/"] {
            let reference = format!("{prefix}{name}");
            let output = command()
                .args(["show-ref", "--verify", "--quiet", &reference])
                .output()?;
            if output.status.success() {
                found.push(reference);
            } else {
                ensure!(
                    output.status.code() == Some(1),
                    "cannot inspect {flag} reference"
                );
            }
        }
        if name == "HEAD" {
            ensure!(
                found.is_empty(),
                "ambiguous {flag} HEAD is shadowed by a named ref"
            );
        } else {
            ensure!(found.len() == 1 && !found[0].starts_with("refs/tags/"),
            "ambiguous or untrusted short {flag} reference `{name}`; use its full refs/heads/, refs/remotes/ or refs/tags/ name");
            expression = format!("{}{suffix}", found[0]);
        }
    }
    let output = command()
        .args([
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{expression}^{{commit}}"),
        ])
        .output()?;
    ensure!(
        !String::from_utf8_lossy(&output.stderr).contains("ambiguous"),
        "ambiguous {flag} reference"
    );
    let resolved = text(checked(output)?)?;
    ensure!(
        (resolved.len() == 40 || resolved.len() == 64)
            && resolved.bytes().all(|b| b.is_ascii_hexdigit()),
        "{flag} did not resolve to one full commit ID"
    );
    Ok(resolved)
}

impl Git {
    pub fn open(destination: &str, head: &str) -> Result<Self> {
        let base = pin(destination, "--diff")?;
        let head = pin(head, "--head")?;
        let root = PathBuf::from(text(original(&["rev-parse", "--show-toplevel"])?)?);
        let objects = text(original(&[
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "objects",
        ])?)?;
        let shallow = text(original(&["rev-parse", "--is-shallow-repository"])?)? == "true";
        let tmp = root.join(".tmp");
        fs::create_dir_all(&tmp)?;
        ensure!(
            !fs::symlink_metadata(&tmp)?.file_type().is_symlink(),
            "temporary evidence directory is a symbolic link"
        );
        let dir = tmp.join(format!(
            "review-tier-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).context("create isolated Git repository")?;
        let object_format = text(original(&["rev-parse", "--show-object-format"])?)?;
        let init = command()
            .args([
                "init",
                "--bare",
                "--template=",
                &format!("--object-format={object_format}"),
            ])
            .arg(&dir)
            .output();
        if let Err(error) = init.and_then(|output| checked(output).map_err(std::io::Error::other)) {
            let _ = fs::remove_dir_all(&dir);
            return Err(error.into());
        }
        let git = Self {
            dir,
            objects: objects.into(),
            root,
            base,
            head,
            shallow,
        };
        git.read(&["cat-file", "-e", &format!("{}^{{commit}}", git.base)])?;
        git.read(&["cat-file", "-e", &format!("{}^{{commit}}", git.head)])?;
        Ok(git)
    }

    fn command(&self) -> Command {
        let mut command = command();
        command
            .arg("--literal-pathspecs")
            .arg("--git-dir")
            .arg(&self.dir)
            .env("GIT_ALTERNATE_OBJECT_DIRECTORIES", &self.objects)
            .env("GIT_ATTR_SOURCE", &self.base);
        command
    }

    pub fn output(&self, args: &[&str]) -> Result<Output> {
        self.command()
            .args(args)
            .output()
            .context("run isolated Git")
    }

    pub fn read(&self, args: &[&str]) -> Result<Vec<u8>> {
        checked(self.output(args)?)
    }

    pub fn read_limited(&self, args: &[&str], limit: usize) -> Result<Vec<u8>> {
        let mut child = self
            .command()
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut bytes = Vec::new();
        let read = child
            .stdout
            .take()
            .context("missing Git output")?
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes);
        if read.is_err() || bytes.len() > limit {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Git evidence exceeded the context limit or could not be read");
        }
        ensure!(child.wait()?.success(), "Git evidence unavailable");
        Ok(bytes)
    }

    pub fn text(&self, args: &[&str]) -> Result<String> {
        text(self.read(args)?)
    }

    pub fn blob(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let listed = self.read(&["ls-tree", "-z", &self.base, "--", path])?;
        if listed.is_empty() {
            return Ok(None);
        }
        let entry = listed
            .split(|&b| b == 0)
            .next()
            .context("missing tree entry")?;
        let tab = entry
            .iter()
            .position(|&b| b == b'\t')
            .context("invalid tree entry")?;
        let meta = &entry[..tab];
        let fields: Vec<_> = std::str::from_utf8(meta)?.split(' ').collect();
        ensure!(
            fields.len() == 3 && matches!(fields[0], "100644" | "100755") && fields[1] == "blob",
            "trusted configuration `{path}` is not a regular file"
        );
        self.read(&["cat-file", "blob", fields[2]]).map(Some)
    }

    pub fn base_paths(&self) -> Result<Vec<String>> {
        self.read(&["ls-tree", "-r", "--name-only", "-z", &self.base])?
            .split(|&byte| byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| {
                Ok(std::str::from_utf8(path)
                    .context("non-UTF-8 trusted path")?
                    .to_owned())
            })
            .collect()
    }

    pub fn merge_tree(&self) -> Result<Output> {
        self.output(&[
            "merge-tree",
            "--write-tree",
            "--no-messages",
            &self.base,
            &self.head,
        ])
    }

    pub fn validate_merge_attributes(&self, paths: &[String]) -> Result<()> {
        let input_path = self.dir.join("attribute-paths");
        let mut input = fs::OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&input_path)?;
        for path in paths {
            input.write_all(path.as_bytes())?;
            input.write_all(&[0])?;
        }
        input.rewind()?;
        let output = self
            .command()
            .args([
                "check-attr",
                "--source",
                &self.base,
                "-z",
                "--all",
                "--stdin",
            ])
            .stdin(Stdio::from(input))
            .output()?;
        let bytes = checked(output)?;
        let fields: Vec<_> = bytes.split(|&b| b == 0).filter(|f| !f.is_empty()).collect();
        ensure!(fields.len().is_multiple_of(3), "invalid attribute metadata");
        for attribute in fields.chunks(3) {
            if attribute[1] == b"merge"
                && !matches!(attribute[2], b"text" | b"set" | b"unspecified")
            {
                bail!(
                    "unsupported trusted merge attribute on {:?}",
                    String::from_utf8_lossy(attribute[0])
                );
            }
        }
        Ok(())
    }

    pub fn merge_paths(&self) -> Result<Vec<String>> {
        let mut paths = std::collections::BTreeSet::new();
        for revision in [&self.base, &self.head] {
            let bytes = self.read(&["ls-tree", "-r", "--name-only", "-z", revision])?;
            for path in bytes.split(|&b| b == 0).filter(|path| !path.is_empty()) {
                paths.insert(
                    std::str::from_utf8(path)
                        .context("non-UTF-8 merge path")?
                        .to_owned(),
                );
            }
        }
        Ok(paths.into_iter().collect())
    }
}

impl Drop for Git {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
