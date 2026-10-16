//! `botica server`: the Language Server Protocol on stdin and stdout.

use anyhow::{Context, Result};
use botica_lsp::Options;
use clap::Args;

use crate::lint_cmd::Editor;
use crate::project::ProjectArgs;

#[derive(Args)]
pub struct ServerArgs {
    #[command(flatten)]
    project: ProjectArgs,
    /// Run the extension rules of the project on the buffers; the option `{"extensions": true}` of
    /// `initialize` asks for the same. Heavy, and a risk: opening a repository in the editor runs
    /// the scripts of the project (Python, Node, commands).
    #[arg(long)]
    extensions: bool,
}

pub fn run(args: ServerArgs) -> Result<()> {
    let editor = Editor::build(args.project)?;
    let root = std::env::current_dir().context("cannot read the working directory")?;
    let options = Options {
        extensions: Some(editor.extensions()),
        enabled: args.extensions,
        ..Options::default()
    };
    botica_lsp::serve(&root, &|path, text| editor.lint(path, text), options)
        .context("the server failed")
}
