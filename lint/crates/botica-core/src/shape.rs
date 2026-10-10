use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use crate::native::{NativeRule, Needs};

const ARTIFACT_DIRS: [&str; 3] = ["resources", "reports", "data"];

/// The package of `packages` that holds `path`, and the path from inside it.
fn within<'p>(packages: &'p [String], path: &'p str) -> Option<(&'p str, &'p str)> {
    packages.iter().find_map(|package| {
        let inside = path.strip_prefix(package.as_str())?.strip_prefix('/')?;
        Some((package.as_str(), inside))
    })
}

/// Keeps data, reports and resources out of a package. A package is a directory of the project
/// root; the rule only reads paths, so the caller says which directories those are.
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
        let Some((package, inside)) = within(&self.packages, doc.path) else {
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
                "CES-32 (repo-shape): {} is under a `{artifact}/` directory inside the package `{package}` — keep data, reports and resources outside the package.",
                doc.path
            ),
        ));
    }
}
