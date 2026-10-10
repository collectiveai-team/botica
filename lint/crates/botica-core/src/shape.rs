use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use crate::native::{NativeRule, Needs};

const ARTIFACT_DIRS: [&str; 3] = ["resources", "reports", "data"];

/// The path from inside the package of `packages` that holds `path`. A package written as `""` is
/// the directory the paths are relative to.
fn within<'p>(packages: &[String], path: &'p str) -> Option<&'p str> {
    packages.iter().find_map(|package| {
        if package.is_empty() {
            return Some(path);
        }
        path.strip_prefix(package.as_str())?.strip_prefix('/')
    })
}

/// Keeps Jupyter notebooks out of a package. A rule of its own, switched off by default, so a
/// project that ships notebooks on purpose can leave it off without giving up `repo-shape`.
pub struct NoNotebooks {
    pub packages: Vec<String>,
}

impl NativeRule for NoNotebooks {
    fn id(&self) -> &'static str {
        "no-notebooks"
    }

    fn needs(&self) -> Needs {
        Needs::Path
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        if !doc.path.ends_with(".ipynb") || within(&self.packages, doc.path).is_none() {
            return;
        }
        out.push(Diagnostic::file_level(
            doc.path,
            self.id(),
            Severity::Error,
            format!(
                "CES-32 (no-notebooks): {} is a notebook inside a package — keep notebooks outside the package.",
                doc.path
            ),
        ));
    }
}

/// Keeps data, reports and resources out of a package. The rule only reads paths, so the caller
/// says which directories are packages, as paths in the same terms as the ones it is given.
pub struct RepoShape {
    pub packages: Vec<String>,
}

impl NativeRule for RepoShape {
    fn id(&self) -> &'static str {
        "repo-shape"
    }

    fn needs(&self) -> Needs {
        Needs::Path
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let Some(inside) = within(&self.packages, doc.path) else {
            return;
        };
        let Some((directories, _file)) = inside.rsplit_once('/') else {
            return;
        };
        let Some(artifact) = directories
            .split('/')
            .find(|part| ARTIFACT_DIRS.contains(part))
        else {
            return;
        };
        out.push(Diagnostic::file_level(
            doc.path,
            self.id(),
            Severity::Error,
            format!(
                "CES-32 (repo-shape): {} is under a `{artifact}/` directory inside a package — keep data, reports and resources outside the package.",
                doc.path
            ),
        ));
    }
}
