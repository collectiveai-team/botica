//! A git repository on disk that stands in for `https://<repo>.git`, and `botica refs` run with it.
//!
//! The configuration of git that the binary inherits (`GIT_CONFIG_COUNT` and the pairs after it)
//! rewrites `https://<repo>.git` to the directory of the repository, so no test reaches the network.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub struct Origin {
    pub dir: PathBuf,
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("UTF-8 output")
}

impl Origin {
    /// An empty repository named `name` under the temporary folder of the tests.
    pub fn new(name: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("origins")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the origin");
        git(&dir, &["init", "--quiet"]);
        git(&dir, &["config", "uploadpack.allowFilter", "true"]);
        Self { dir }
    }

    /// A commit that writes `files` on top of the last one; returns its SHA.
    pub fn commit(&self, files: &[(&str, &[u8])]) -> String {
        for (path, bytes) in files {
            let file = self.dir.join(path);
            std::fs::create_dir_all(file.parent().expect("has a parent")).expect("create parents");
            std::fs::write(file, bytes).expect("write the file");
        }
        git(&self.dir, &["add", "--all"]);
        git(&self.dir, &["commit", "--quiet", "--message", "a commit"]);
        git(&self.dir, &["rev-parse", "HEAD"]).trim().to_owned()
    }
}

/// How `botica refs` is launched: which addresses are repositories on disk, and the environment.
#[derive(Default)]
pub struct Setup<'a> {
    /// `(repo, origin)`: `https://<repo>.git` is the directory of `origin`.
    pub maps: Vec<(&'a str, &'a Origin)>,
    pub env: Vec<(&'a str, String)>,
    pub removed: Vec<&'a str>,
}

pub fn refs_run(dir: &Path, args: &[&str], setup: &Setup) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_botica"));
    command.arg("refs").args(args).current_dir(dir);
    if !setup.maps.is_empty() {
        command.env("GIT_CONFIG_COUNT", setup.maps.len().to_string());
    }
    for (index, (repo, origin)) in setup.maps.iter().enumerate() {
        command.env(
            format!("GIT_CONFIG_KEY_{index}"),
            format!("url.file://{}.insteadOf", origin.dir.display()),
        );
        command.env(
            format!("GIT_CONFIG_VALUE_{index}"),
            format!("https://{repo}.git"),
        );
    }
    for (name, value) in &setup.env {
        command.env(name, value);
    }
    for name in &setup.removed {
        command.env_remove(name);
    }
    command.output().expect("run botica")
}

/// `botica refs <args>` where `https://<repo>.git` is `origin`.
pub fn refs_at(dir: &Path, args: &[&str], repo: &str, origin: &Origin) -> Output {
    refs_run(
        dir,
        args,
        &Setup {
            maps: vec![(repo, origin)],
            ..Setup::default()
        },
    )
}
