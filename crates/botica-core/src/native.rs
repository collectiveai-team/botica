use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;

/// What a native rule reads of a file, like `needs` in `rule.toml`. The engine hands the rule
/// only the files that give it that much.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Needs {
    /// The path alone: the rule runs whatever the contents are.
    Path,
    /// The bytes: the rule runs on any file that is not binary, UTF-8 or not.
    Bytes,
    /// The text: the rule runs on UTF-8 files only.
    Text,
}

pub trait NativeRule: Send + Sync {
    fn id(&self) -> &'static str;

    fn needs(&self) -> Needs {
        Needs::Text
    }

    /// Whether the rule reads `Document::base`, the text of the file before the change, so the run
    /// fetches it for the files of the rule.
    fn reads_base(&self) -> bool {
        false
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>);
}

pub struct FileSizeGuard {
    pub warn_at: usize,
    pub error_at: usize,
}

impl Default for FileSizeGuard {
    fn default() -> Self {
        Self {
            warn_at: 400,
            error_at: 700,
        }
    }
}

impl NativeRule for FileSizeGuard {
    fn id(&self) -> &'static str {
        "file-size-guard"
    }

    fn needs(&self) -> Needs {
        Needs::Bytes
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let lines = doc.line_count();
        let (severity, message) = if lines >= self.error_at {
            (
                Severity::Error,
                format!(
                    "{} is too large ({lines} lines, limit is {})",
                    doc.path, self.error_at
                ),
            )
        } else if lines >= self.warn_at {
            (
                Severity::Warning,
                format!(
                    "{} has {lines} lines (warns at {}, fails at {})",
                    doc.path, self.warn_at, self.error_at
                ),
            )
        } else {
            return;
        };
        out.push(Diagnostic::file_level(
            doc.path,
            self.id(),
            severity,
            message,
        ));
    }
}

#[derive(Default)]
pub struct NoUtils {
    /// Where botica runs inside the project, so `tests/` is recognised from the root and not
    /// only from the directory the paths start at (`""` at the root).
    pub working_prefix: String,
}

const CATCH_ALL_STEMS: [&str; 5] = ["utils", "helpers", "aux", "misc", "common"];

/// Names that JavaScript and TypeScript projects give to the same kind of module; a warning, since
/// a framework may expect them (`lib/` of SvelteKit is a folder, which the rule does not read).
const VAGUE_SCRIPT_STEMS: [&str; 3] = ["util", "shared", "lib"];

const SCRIPT_EXTENSIONS: [&str; 8] = ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

impl NoUtils {
    /// Whether a directory called `tests` is on the way to the file, counting the ones above
    /// where botica runs.
    fn in_tests(&self, path: &str) -> bool {
        let (directories, _) = path.rsplit_once('/').unwrap_or(("", path));
        self.working_prefix
            .split('/')
            .chain(directories.split('/'))
            .any(|part| part == "tests")
    }
}

/// The severity of a module with this name, or `None` when the name says what it holds.
fn severity_of(stem: &str, extension: &str) -> Option<Severity> {
    let script = SCRIPT_EXTENSIONS.contains(&extension);
    if !(script || matches!(extension, "py" | "go" | "rs")) {
        return None;
    }
    if CATCH_ALL_STEMS.contains(&stem) {
        Some(Severity::Error)
    } else if script && VAGUE_SCRIPT_STEMS.contains(&stem) {
        Some(Severity::Warning)
    } else {
        None
    }
}

impl NativeRule for NoUtils {
    fn id(&self) -> &'static str {
        "no-utils"
    }

    fn needs(&self) -> Needs {
        Needs::Path
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let path = doc.path;
        let name = path.rsplit('/').next().unwrap_or(path);
        let Some((stem, extension)) = name.rsplit_once('.') else {
            return;
        };
        let Some(severity) = severity_of(stem, extension) else {
            return;
        };
        if self.in_tests(path) {
            return;
        }
        let message = match severity {
            Severity::Error => format!(
                "{path} is a catch-all module — name modules for what they hold, not utils/helpers/aux/misc/common."
            ),
            _ => format!(
                "{path} has a vague name — name modules for what they hold, not util/shared/lib."
            ),
        };
        out.push(Diagnostic::file_level(path, self.id(), severity, message));
    }
}
