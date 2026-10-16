//! `botica server`: the Language Server Protocol on stdin and stdout.

use anyhow::{Context, Result};
use clap::Args;

use crate::lint_cmd::Editor;
use crate::project::ProjectArgs;

#[derive(Args)]
pub struct ServerArgs {
    #[command(flatten)]
    project: ProjectArgs,
}

pub fn run(args: ServerArgs) -> Result<()> {
    let editor = Editor::build(args.project)?;
    let root = std::env::current_dir().context("cannot read the working directory")?;
    botica_lsp::serve(&root, &|path, text| editor.lint(path, text)).context("the server failed")
}
