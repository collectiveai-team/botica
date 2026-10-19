//! The Language Server Protocol front-end of botica: the findings of the rules for the buffers
//! of an editor, framed as the protocol frames them (`Content-Length` headers, JSON-RPC 2.0).
//!
//! Each `didOpen` lints the text the editor sends and publishes the findings, and `didClose`
//! clears them. The editor sends each change as a range to replace (incremental sync), and the
//! buffer keeps its syntax tree between them. A change shows the findings of the rules that run as
//! the text is typed (`type` in their `run-on`); an open and a save add those of the rules that run
//! at save, and a save starts the rules of project scope on a thread of their own. A save of the
//! configuration builds the linter again.
//!
//! The extension rules, when they are asked for, run in their own processes, one thread for each
//! rule: the text of a document is sent to them when it has stayed as it is for the debounce, and
//! what they find is published after the findings of the engine, together with them. Columns count
//! in UTF-16 code units, or in bytes when the editor offers UTF-8.

mod extensions;
mod linter;
mod server;
mod wire;

use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use extensions::Event;
pub use extensions::{Extensions, Process, MAX_RESTARTS};
pub use linter::{Lint, Linter, Plain, ProjectRun, Reload, When};
use wire::read_message;

/// How long the text of a document has to stay as it is before the extensions lint it.
pub const DEBOUNCE: Duration = Duration::from_millis(250);

/// What the server runs besides the engine.
pub struct Options<'a> {
    /// The extension rules of the project, if there are any to run.
    pub extensions: Option<&'a dyn Extensions>,
    /// Whether the extensions run although the editor did not ask for them.
    pub enabled: bool,
    pub debounce: Duration,
    /// What builds the linter again when the configuration changes; without it the configuration
    /// is the one the server started with.
    pub reload: Option<Reload<'a>>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self {
            extensions: None,
            enabled: false,
            debounce: DEBOUNCE,
            reload: None,
        }
    }
}

/// Serves the messages of `input` until `exit` or the end of `input`. `lint` lints a document by
/// its path from `root`.
///
/// # Errors
/// Fails when `input` or `output` does, or when a message is not framed or is not JSON.
pub fn serve_on(
    input: impl BufRead + Send + 'static,
    output: impl Write,
    root: &Path,
    lint: Lint<'_>,
) -> io::Result<()> {
    serve_with(input, output, root, lint, Options::default())
}

/// As [`serve_on`], and the extension rules of `options` lint the documents too.
///
/// # Errors
/// As [`serve_on`].
pub fn serve_with(
    input: impl BufRead + Send + 'static,
    output: impl Write,
    root: &Path,
    lint: Lint<'_>,
    options: Options<'_>,
) -> io::Result<()> {
    serve_linter(input, output, root, &Plain(lint), options)
}

/// As [`serve_with`], with `linter` for the documents.
///
/// # Errors
/// As [`serve_on`].
pub fn serve_linter(
    mut input: impl BufRead + Send + 'static,
    output: impl Write,
    root: &Path,
    linter: &dyn Linter,
    options: Options<'_>,
) -> io::Result<()> {
    let (events, inbox) = mpsc::channel();
    let reader = events.clone();
    std::thread::spawn(move || loop {
        let message = read_message(&mut input);
        let last = !matches!(message, Ok(Some(_)));
        if reader.send(Event::Input(message)).is_err() || last {
            break;
        }
    });
    std::thread::scope(|scope| {
        let mut server = server::fresh(output, root, linter, options, scope, events);
        let served = server.run(&inbox);
        drop(server);
        served
    })
}

/// Serves on stdin and stdout, as [`serve_linter`].
///
/// # Errors
/// As [`serve_on`].
pub fn serve(root: &Path, linter: &dyn Linter, options: Options<'_>) -> io::Result<()> {
    serve_linter(
        io::BufReader::new(io::stdin()),
        io::stdout().lock(),
        root,
        linter,
        options,
    )
}
