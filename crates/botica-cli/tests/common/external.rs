//! The scenario of the tests of external references: a repository on disk with two commits, a
//! project and an empty cache.

use std::path::{Path, PathBuf};
use std::process::Output;

use super::git_remote::{refs_at, refs_run, Origin, Setup};
use super::project_in;
use super::refs::add_topic;

pub const REPO: &str = "github.com/acme/app";
pub const FILE: &[u8] = b"one\ntwo\nthree\n";

pub struct World {
    pub dir: PathBuf,
    pub origin: Origin,
    /// The commit that holds `FILE`; the head of the origin has other text.
    pub first: String,
    pub cache: PathBuf,
}

pub fn world(name: &str) -> World {
    let origin = Origin::new(name);
    let first = origin.commit(&[("src/a.py", FILE)]);
    origin.commit(&[("src/a.py", b"changed\n")]);
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("refs-cache")
        .join(name);
    let _ = std::fs::remove_dir_all(&cache);
    World {
        dir: project_in("refs-external", name, None),
        origin,
        first,
        cache,
    }
}

impl World {
    /// The topic `demo`, with one external reference to `src/a.py` at `rev`.
    pub fn topic_at(&self, repo: &str, rev: &str, extra: &str) {
        add_topic(
            &self.dir,
            "demo",
            &format!(
                "title = \"E\"\n\n[[references]]\nrepo = \"{repo}\"\nrev = \"{rev}\"\n\
                 license = \"MIT\"\npath = \"src/a.py\"\nnote = \"the setup\"\n{extra}"
            ),
        );
    }

    pub fn topic(&self, repo: &str, extra: &str) {
        self.topic_at(repo, &self.first, extra);
    }

    /// The topic `demo` points at `path` instead of `src/a.py`.
    pub fn point_at(&self, path: &str) {
        let file = self.dir.join("references/demo/topic.toml");
        let toml = std::fs::read_to_string(&file).expect("the topic");
        std::fs::write(file, toml.replace("src/a.py", path)).expect("write the topic");
    }

    /// A folder with a `git` that runs `script`, for the `PATH`.
    #[cfg(unix)]
    pub fn fake_git(&self, script: &str) -> String {
        use std::os::unix::fs::PermissionsExt;

        let bin = self.cache.join("fake-bin");
        std::fs::create_dir_all(&bin).expect("create the folder");
        let fake = bin.join("git");
        std::fs::write(&fake, format!("#!/bin/sh\n{script}\n")).expect("write the script");
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        format!("{}:/usr/bin:/bin", bin.display())
    }

    pub fn cache_arg(&self) -> String {
        self.cache.display().to_string()
    }

    /// `refs <args> --cache-dir <cache>` where `https://<repo>.git` is the origin.
    pub fn run(&self, repo: &str, args: &[&str]) -> Output {
        let cache = self.cache_arg();
        let mut all = args.to_vec();
        all.extend(["--cache-dir", &cache]);
        refs_at(&self.dir, &all, repo, &self.origin)
    }

    /// The same, with no repository behind any address.
    pub fn run_offline(&self, args: &[&str]) -> Output {
        let cache = self.cache_arg();
        let mut all = args.to_vec();
        all.extend(["--cache-dir", &cache]);
        refs_run(&self.dir, &all, &Setup::default())
    }

    pub fn cached(&self, repo: &str, relative: &str) -> PathBuf {
        self.cache.join(repo).join(&self.first).join(relative)
    }
}
