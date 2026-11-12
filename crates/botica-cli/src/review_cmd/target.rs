//! Where the export goes: a file, or the standard output.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::Host;
use crate::output::print;

/// The files that GitLab reads before `.gitlab/CODEOWNERS`, from the root: it takes the first
/// one it finds.
pub(super) const GITLAB_FIRST: [&str; 2] = ["CODEOWNERS", "docs/CODEOWNERS"];

pub(super) enum Target {
    Stdout,
    /// A file, and whether it is the one that the host reads by default.
    File {
        path: PathBuf,
        default: bool,
    },
}

impl Target {
    pub(super) fn of(output: Option<&Path>, host: Host, root: &Path, check: bool) -> Result<Self> {
        let target = match output {
            Some(path) if path.as_os_str() == "-" => Self::Stdout,
            Some(path) => Self::File {
                path: path.to_path_buf(),
                default: false,
            },
            None if !root.join(".git").exists() => bail!(
                "the configuration is in `{}`, which is not the root of the git repository: GitHub \
                 and GitLab read CODEOWNERS only there, and the paths of the file start there. \
                 Give the file to write with --output",
                root.display()
            ),
            None => Self::File {
                path: root.join(default_path(host)),
                default: true,
            },
        };
        if check && matches!(target, Self::Stdout) {
            bail!("--check compares the export with a file: leave out `--output -`");
        }
        Ok(target)
    }

    /// GitLab reads `CODEOWNERS` or `docs/CODEOWNERS` when one is there, and never sees
    /// `.gitlab/CODEOWNERS`.
    pub(super) fn refuse_shadowed(&self, host: Host, root: &Path) -> Result<()> {
        if host != Host::Gitlab || !matches!(self, Self::File { default: true, .. }) {
            return Ok(());
        }
        if let Some(first) = GITLAB_FIRST.iter().find(|file| root.join(file).is_file()) {
            bail!(
                "GitLab reads `{first}` and not `.gitlab/CODEOWNERS`: remove `{first}`, or give \
                 the file to write with --output"
            );
        }
        Ok(())
    }

    pub(super) fn deliver(&self, text: &str, check: bool, host: Host) -> Result<()> {
        let path = match self {
            Self::Stdout => {
                print!("{text}");
                return Ok(());
            }
            Self::File { path, .. } => path,
        };
        if check {
            return compare(path, text, &self.command(host));
        }
        write(path, text)
    }

    /// The command that writes the file again.
    fn command(&self, host: Host) -> String {
        let base = format!(
            "botica review export --format codeowners --host {}",
            host.name()
        );
        match self {
            Self::File {
                path,
                default: false,
            } => format!("{base} --output {}", path.display()),
            _ => base,
        }
    }
}

fn default_path(host: Host) -> &'static str {
    match host {
        Host::Github => ".github/CODEOWNERS",
        Host::Gitlab => ".gitlab/CODEOWNERS",
    }
}

fn compare(path: &Path, text: &str, command: &str) -> Result<()> {
    if fs::read_to_string(path).is_ok_and(|there| there == text) {
        return Ok(());
    }
    bail!(
        "`{}` differs from the areas of the configuration: run `{command}`",
        path.display()
    )
}

fn write(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create `{}`", parent.display()))?;
    }
    fs::write(path, text).with_context(|| format!("cannot write `{}`", path.display()))
}
