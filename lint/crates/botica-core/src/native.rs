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
                    "CES-71 (file-size-guard): {} is too large ({lines} lines, limit is {})",
                    doc.path, self.error_at
                ),
            )
        } else if lines >= self.warn_at {
            (
                Severity::Warning,
                format!(
                    "CES-71 (file-size-guard): {} is large ({lines} lines, recommended limit is {})",
                    doc.path, self.warn_at
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

pub struct NoUtils;

const CATCH_ALL_STEMS: [&str; 5] = ["utils", "helpers", "aux", "misc", "common"];
const CODE_EXTENSIONS: [&str; 3] = ["py", "ts", "js"];

impl NativeRule for NoUtils {
    fn id(&self) -> &'static str {
        "no-utils"
    }

    fn needs(&self) -> Needs {
        Needs::Path
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let path = doc.path;
        let in_tests = path.starts_with("tests/") || path.contains("/tests/");
        let name = path.rsplit('/').next().unwrap_or(path);
        let Some((stem, extension)) = name.rsplit_once('.') else {
            return;
        };
        if in_tests || !CATCH_ALL_STEMS.contains(&stem) || !CODE_EXTENSIONS.contains(&extension) {
            return;
        }
        out.push(Diagnostic::file_level(
            path,
            self.id(),
            Severity::Error,
            format!(
                "CES-63 (no-utils): {path} is a catch-all module — name modules for what they hold, not utils/helpers/aux/misc/common."
            ),
        ));
    }
}
