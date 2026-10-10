use botica_catalog::Ecosystem;

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use crate::native::{NativeRule, Needs};

/// A directory that holds the code of a project, as a path in the same terms as the paths the
/// rules are given. `""` is the directory the paths are relative to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeDir {
    pub path: String,
    pub ecosystem: Ecosystem,
}

/// Directories whose contents are not the code of the package, even when they sit inside it.
const EXEMPT_DIRS: [&str; 9] = [
    "tests",
    "test",
    "__tests__",
    "fixtures",
    "testdata",
    "examples",
    "docs",
    "benches",
    "snapshots",
];

const TEST_DIRS: [&str; 2] = ["tests", "test"];

/// Directories that hold what the code reads, not code, in the house layout of a Python package.
const ARTIFACT_DIRS: [&str; 3] = ["data", "resources", "reports"];

/// Files of data or of trained models: what a notebook or a pipeline produces or consumes.
const DATA_EXTENSIONS: [&str; 23] = [
    "csv",
    "tsv",
    "parquet",
    "feather",
    "arrow",
    "h5",
    "hdf5",
    "pkl",
    "pickle",
    "npy",
    "npz",
    "xlsx",
    "xls",
    "sqlite",
    "sqlite3",
    "db",
    "joblib",
    "pt",
    "pth",
    "ckpt",
    "safetensors",
    "onnx",
    "gz",
];

const PYTHON_CODE_EXTENSIONS: [&str; 4] = ["py", "pyi", "pyx", "pxd"];

/// Where `path` sits inside the code of one of `dirs` that `applies` to, as the path from there.
fn within<'p>(
    dirs: &[CodeDir],
    path: &'p str,
    applies: impl Fn(Ecosystem) -> bool,
) -> Option<&'p str> {
    dirs.iter()
        .filter(|dir| applies(dir.ecosystem))
        .find_map(|dir| {
            if dir.path.is_empty() {
                return Some(path);
            }
            path.strip_prefix(dir.path.as_str())?.strip_prefix('/')
        })
}

/// The directories of `inside` (a path from a code directory) and its file name.
fn split(inside: &str) -> (Vec<&str>, &str) {
    match inside.rsplit_once('/') {
        Some((directories, file)) => (directories.split('/').collect(), file),
        None => (Vec::new(), inside),
    }
}

fn extension(file: &str) -> Option<String> {
    file.rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
}

fn is_exempt(directory: &str) -> bool {
    EXEMPT_DIRS.contains(&directory)
        || directory.ends_with("_tests")
        || directory.ends_with("-tests")
}

fn is_python_test_file(file: &str) -> bool {
    (file.starts_with("test_") && file.ends_with(".py"))
        || file.ends_with("_test.py")
        || file == "conftest.py"
}

fn found(doc: &Document<'_>, rule: &'static str, message: String) -> Diagnostic {
    Diagnostic::file_level(doc.path, rule, Severity::Error, message)
}

/// Tests live in a `tests/` directory beside the package, not inside it.
pub struct NoTestsInPackage {
    pub code_dirs: Vec<CodeDir>,
}

impl NativeRule for NoTestsInPackage {
    fn id(&self) -> &'static str {
        "no-tests-in-package"
    }

    fn needs(&self) -> Needs {
        Needs::Path
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let Some(inside) = within(&self.code_dirs, doc.path, |e| e == Ecosystem::Python) else {
            return;
        };
        let (directories, file) = split(inside);
        let in_tests_directory = directories.iter().any(|part| TEST_DIRS.contains(part));
        if !in_tests_directory && !is_python_test_file(file) {
            return;
        }
        out.push(found(
            doc,
            self.id(),
            format!(
                "no-tests-in-package: {} is a test inside the package — keep tests in a `tests/` directory beside the package, not in it.",
                doc.path
            ),
        ));
    }
}

/// Notebooks live beside the code, not in it.
pub struct NoNotebooks {
    pub code_dirs: Vec<CodeDir>,
}

impl NativeRule for NoNotebooks {
    fn id(&self) -> &'static str {
        "no-notebooks"
    }

    fn needs(&self) -> Needs {
        Needs::Path
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        if extension(doc.path).as_deref() != Some("ipynb")
            || within(&self.code_dirs, doc.path, |_| true).is_none()
        {
            return;
        }
        out.push(found(
            doc,
            self.id(),
            format!(
                "CES-32 (no-notebooks): {} is a notebook inside the code of the project — keep notebooks outside it.",
                doc.path
            ),
        ));
    }
}

/// Data and trained models live beside the code, not in it.
pub struct NoArtifactsInPackage {
    pub code_dirs: Vec<CodeDir>,
}

impl NoArtifactsInPackage {
    /// Why `inside` (from a code directory of `ecosystem`) is an artifact, if it is one.
    fn reason(ecosystem: Ecosystem, inside: &str) -> Option<String> {
        let (directories, file) = split(inside);
        if directories.iter().copied().any(is_exempt) {
            return None;
        }
        if extension(file).is_some_and(|e| DATA_EXTENSIONS.contains(&e.as_str())) {
            return Some("a data or model file".to_owned());
        }
        if ecosystem != Ecosystem::Python {
            return None;
        }
        let artifact = directories
            .iter()
            .find(|part| ARTIFACT_DIRS.contains(part))?;
        let is_code = extension(file).is_some_and(|e| PYTHON_CODE_EXTENSIONS.contains(&e.as_str()))
            || file == "py.typed";
        (!is_code).then(|| format!("a file under a `{artifact}/` directory"))
    }
}

impl NativeRule for NoArtifactsInPackage {
    fn id(&self) -> &'static str {
        "no-artifacts-in-package"
    }

    fn needs(&self) -> Needs {
        Needs::Path
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let Some((ecosystem, inside)) = self.code_dirs.iter().find_map(|dir| {
            within(std::slice::from_ref(dir), doc.path, |_| true).map(|i| (dir.ecosystem, i))
        }) else {
            return;
        };
        let Some(what) = Self::reason(ecosystem, inside) else {
            return;
        };
        out.push(found(
            doc,
            self.id(),
            format!(
                "CES-32 (no-artifacts-in-package): {} is {what} inside the code of the project — keep data, models and reports outside it.",
                doc.path
            ),
        ));
    }
}
