//! `botica server`: the Language Server Protocol on stdin and stdout.

use anyhow::{Context, Result};
use botica_lsp::{Linter, Options};
use clap::Args;

use crate::lint_cmd::Editor;
use crate::project::ProjectArgs;

#[derive(Args)]
pub struct ServerArgs {
    #[command(flatten)]
    project: ProjectArgs,
    /// Run the extension rules of the catalogs of the project on the buffers; the option `{"extensions": true}` of
    /// `initialize` asks for the same. Heavy, and a risk: opening a repository in the editor runs
    /// the scripts of the project (Python, Node, commands).
    #[arg(long)]
    extensions: bool,
}

pub fn run(args: ServerArgs) -> Result<()> {
    let editor = Editor::build(args.project.clone())?;
    let root = std::env::current_dir().context("cannot read the working directory")?;
    let reload = || {
        Editor::build(args.project.clone())
            .map(|editor| Box::new(editor) as Box<dyn Linter>)
            .map_err(|error| format!("{error:#}"))
    };
    let options = Options {
        extensions: Some(editor.extensions()),
        enabled: args.extensions,
        reload: Some(&reload),
        ..Options::default()
    };
    botica_lsp::serve(&root, &editor, options).context("the server failed")
}
