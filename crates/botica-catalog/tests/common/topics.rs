//! Topics of `references/` as the tests of the references write them.

use botica_catalog::{CatalogError, References, SourceFile};

use super::file;

pub const README: &str = "# Logging\n\nRead the setup.\n";

pub const SHA: &str = "3f2a9c1d5e7b4a6c8d0e1f2a3b4c5d6e7f8a9b0c";

pub const INTERNAL: &str = r#"title = "Structured logging"

[[references]]
path = "scaffolding/templates/snippets/core/logger.py"
lines = "18-30"
anchor = "def get_logger"
note = "the setup"
"#;

pub const EXTERNAL: &str = r#"title = "Structured logging"

[[references]]
repo = "github.com/acme/app"
rev = "3f2a9c1d5e7b4a6c8d0e1f2a3b4c5d6e7f8a9b0c"
path = "src/app/core/logger.py"
license = "MIT"
note = "the setup"
"#;

pub const PAGE: &str = r#"title = "Structured logging"

[[references]]
url = "https://www.structlog.org/en/stable/getting-started.html"
note = "how to configure structlog"
"#;

pub fn topic(id: &str, toml: &str) -> Vec<SourceFile> {
    vec![
        file(&format!("{id}/topic.toml"), toml),
        file(&format!("{id}/README.md"), README),
    ]
}

pub fn load(parts: Vec<Vec<SourceFile>>) -> Result<References, CatalogError> {
    References::from_files(&parts.concat())
}

pub fn rejected(parts: Vec<Vec<SourceFile>>) -> String {
    load(parts)
        .expect_err("the references should be rejected")
        .to_string()
}
