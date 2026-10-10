use std::path::Path;

use botica_catalog::Ecosystem;
use botica_core::CodeDir;

/// Directories that are never a project of their own, however deep: dependencies, build output,
/// and the places where examples, test data and templates keep their own manifests.
const NOT_PROJECTS: [&str; 20] = [
    "node_modules",
    "vendor",
    "third_party",
    "target",
    "dist",
    "build",
    "venv",
    "env",
    "site-packages",
    "__pycache__",
    "examples",
    "example",
    "fixtures",
    "testdata",
    "templates",
    "template",
    "tests",
    "test",
    "__tests__",
    "docs",
];

/// How deep below the root a project is looked for: the root, `backend/` and `packages/ui/`. A
/// project deeper than that is named in the configuration.
const DEPTH: usize = 2;

/// Directories next to a Python package that are never one, even with an `__init__.py` in them.
const NOT_PACKAGES: [&str; 8] = [
    "tests",
    "test",
    "src",
    "docs",
    "scripts",
    "examples",
    "fixtures",
    "benchmarks",
];

/// What a directory holds that matters to the search for code, read with one listing.
struct Listing {
    files: Vec<String>,
    directories: Vec<String>,
}

fn list(directory: &Path) -> Option<Listing> {
    let mut listing = Listing {
        files: Vec::new(),
        directories: Vec::new(),
    };
    for entry in std::fs::read_dir(directory).ok()?.filter_map(Result::ok) {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => listing.directories.push(name),
            Ok(_) => listing.files.push(name),
            Err(_) => {}
        }
    }
    Some(listing)
}

fn joined(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}

/// The ecosystems whose manifest sits in a directory with these files.
fn ecosystems_of(files: &[String]) -> Vec<Ecosystem> {
    let has = |names: &[&str]| files.iter().any(|file| names.contains(&file.as_str()));
    let mut found = Vec::new();
    if has(&["pyproject.toml", "setup.py"]) {
        found.push(Ecosystem::Python);
    }
    if has(&["package.json"]) {
        found.push(Ecosystem::Js);
    }
    if has(&["Cargo.toml"]) {
        found.push(Ecosystem::Rust);
    }
    if has(&["go.mod"]) {
        found.push(Ecosystem::Go);
    }
    if has(&["pom.xml", "build.gradle", "build.gradle.kts"]) {
        found.push(Ecosystem::Jvm);
    }
    found
}

/// The Python packages of a project: the directories of it, and of its `src/`, with an
/// `__init__.py`.
fn python_packages(root: &Path, project: &str, listing: &Listing) -> Vec<String> {
    let mut packages = Vec::new();
    let mut scan = |parent: &str, directories: &[String]| {
        for name in directories {
            let path = joined(parent, name);
            if !NOT_PACKAGES.contains(&name.as_str())
                && root.join(&path).join("__init__.py").is_file()
            {
                packages.push(path);
            }
        }
    };
    scan(project, &listing.directories);
    if listing.directories.iter().any(|name| name == "src") {
        let src = joined(project, "src");
        if let Some(inner) = list(&root.join(&src)) {
            scan(&src, &inner.directories);
        }
    }
    packages
}

fn code_of(root: &Path, project: &str, ecosystem: Ecosystem, listing: &Listing) -> Vec<String> {
    let has = |name: &str| listing.directories.iter().any(|dir| dir == name);
    match ecosystem {
        Ecosystem::Python => python_packages(root, project, listing),
        Ecosystem::Js | Ecosystem::Rust => {
            if has("src") {
                vec![joined(project, "src")]
            } else {
                Vec::new()
            }
        }
        Ecosystem::Go => ["internal", "pkg", "cmd"]
            .into_iter()
            .filter(|name| has(name))
            .map(|name| joined(project, name))
            .collect(),
        Ecosystem::Jvm => {
            let main = joined(project, "src/main");
            ["java", "kotlin", "scala"]
                .into_iter()
                .map(|name| joined(&main, name))
                .filter(|path| root.join(path).is_dir())
                .collect()
        }
    }
}

/// What one directory of the search brings: the code directories of its project, if it is one, and
/// the directories below it that are worth a look.
struct Inspected {
    code: Vec<CodeDir>,
    below: Vec<String>,
}

/// Looks at the directory `project` (a path from the root). A project is not searched for other
/// projects inside it, as the layout of a monorepo does not nest them; the root is, and so is a
/// directory with no manifest, which groups projects (`packages/`, `crates/`).
fn inspect(root: &Path, project: &str, depth: usize) -> Inspected {
    let mut inspected = Inspected {
        code: Vec::new(),
        below: Vec::new(),
    };
    let Some(listing) = list(&root.join(project)) else {
        return inspected;
    };
    let ecosystems = ecosystems_of(&listing.files);
    for &ecosystem in &ecosystems {
        for path in code_of(root, project, ecosystem, &listing) {
            inspected.code.push(CodeDir { path, ecosystem });
        }
    }
    let is_project = !ecosystems.is_empty();
    if depth < DEPTH && (depth == 0 || !is_project) {
        for name in &listing.directories {
            // Hidden directories hold no code the walk reaches, and `.git` alone has hundreds
            // of directories to list.
            if !name.starts_with('.') && !NOT_PROJECTS.contains(&name.as_str()) {
                inspected.below.push(joined(project, name));
            }
        }
    }
    inspected
}

/// `f` on each item, on as many threads as the machine offers (a few at most), in order.
fn in_parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(8)
        .min(items.len());
    if workers <= 1 {
        return items.iter().map(f).collect();
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(items.len().div_ceil(workers))
            .map(|chunk| scope.spawn(|| chunk.iter().map(&f).collect::<Vec<_>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("a thread of the search"))
            .collect()
    })
}

/// Where the code of the project at `root` lives: for each project found within `DEPTH` levels,
/// the directories its ecosystem keeps code in. Sorted. Each level of directories is listed at
/// once, since this runs before the walk starts and the walk cannot overlap with it.
pub fn detect(root: &Path) -> Vec<CodeDir> {
    let mut found = Vec::new();
    let mut level = vec![String::new()];
    for depth in 0..=DEPTH {
        let mut next = Vec::new();
        for inspected in in_parallel(&level, |project| inspect(root, project, depth)) {
            found.extend(inspected.code);
            next.extend(inspected.below);
        }
        level = next;
    }
    found.sort_by(|a, b| (&a.path, a.ecosystem).cmp(&(&b.path, b.ecosystem)));
    found
}

/// The code directories of the configuration, which replace the detection.
pub fn configured(listed: &std::collections::BTreeMap<String, Vec<String>>) -> Vec<CodeDir> {
    let mut found: Vec<CodeDir> = listed
        .iter()
        .filter_map(|(name, paths)| Some((Ecosystem::parse(name)?, paths)))
        .flat_map(|(ecosystem, paths)| {
            paths.iter().map(move |path| CodeDir {
                path: path.clone(),
                ecosystem,
            })
        })
        .collect();
    found.sort_by(|a, b| (&a.path, a.ecosystem).cmp(&(&b.path, b.ecosystem)));
    found
}

/// `dirs` (paths from the project root) as seen from the directory botica runs in, `prefix` being
/// where that directory sits inside the root. One that holds the directory becomes `""`, the
/// whole tree; one below it loses the prefix; the rest are out of reach and are dropped.
pub fn seen_from(dirs: Vec<CodeDir>, prefix: Option<&str>) -> Vec<CodeDir> {
    let Some(prefix) = prefix else {
        return dirs;
    };
    dirs.into_iter()
        .filter_map(|dir| {
            let path = if dir.path == prefix || prefix.starts_with(&format!("{}/", dir.path)) {
                String::new()
            } else {
                dir.path.strip_prefix(&format!("{prefix}/"))?.to_owned()
            };
            Some(CodeDir { path, ..dir })
        })
        .collect()
}
