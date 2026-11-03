//! CLI output does not interrupt a command when its stdout consumer closes the pipe.

use std::fmt;
use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::{Mutex, OnceLock};

macro_rules! print {
    ($($args:tt)*) => {
        $crate::output::print_stdout(format_args!($($args)*))
    };
}

macro_rules! println {
    ($($args:tt)*) => {
        $crate::output::print_stdout(format_args!("{}\n", format_args!($($args)*)))
    };
}

macro_rules! eprintln {
    ($($args:tt)*) => {
        $crate::output::print_stderr(format_args!("{}\n", format_args!($($args)*)))
    };
}

pub(crate) use {eprintln, print, println};

struct Output<W> {
    inner: W,
    broken: bool,
}

impl<W> Output<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            broken: false,
        }
    }
}

impl<W: Write> Write for Output<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.broken {
            return Ok(bytes.len());
        }
        match self.inner.write(bytes) {
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                self.broken = true;
                Ok(bytes.len())
            }
            result => result,
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.broken {
            return Ok(());
        }
        match self.inner.flush() {
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                self.broken = true;
                Ok(())
            }
            result => result,
        }
    }
}

struct State {
    writer: Output<io::Stdout>,
    error: Option<String>,
}

impl State {
    fn record<T>(&mut self, result: io::Result<T>) -> io::Result<T> {
        if let Err(error) = &result {
            self.error.get_or_insert_with(|| error.to_string());
        }
        result
    }
}

static STDOUT: OnceLock<Mutex<State>> = OnceLock::new();

fn state() -> &'static Mutex<State> {
    STDOUT.get_or_init(|| {
        Mutex::new(State {
            writer: Output::new(io::stdout()),
            error: None,
        })
    })
}

pub(crate) struct Stdout;

/// Writes CLI output, discarding it after the consumer closes stdout.
pub(crate) fn stdout() -> Stdout {
    Stdout
}

impl Write for Stdout {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut state = state().lock().unwrap_or_else(|error| error.into_inner());
        state.writer.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state = state().lock().unwrap_or_else(|error| error.into_inner());
        state.writer.flush()
    }
}

pub(crate) fn print_stdout(args: fmt::Arguments<'_>) {
    let result = stdout().write_fmt(args);
    let mut state = state().lock().unwrap_or_else(|error| error.into_inner());
    let _ = state.record(result);
}

pub(crate) fn print_stderr(args: fmt::Arguments<'_>) {
    let _ = io::stderr().lock().write_fmt(args);
}

/// Flushes CLI output and preserves command failures if diagnostics cannot be written.
pub(crate) fn finish(code: ExitCode) -> ExitCode {
    let Some(state) = STDOUT.get() else {
        return code;
    };
    let error = {
        let mut state = state.lock().unwrap_or_else(|error| error.into_inner());
        let result = state.writer.flush();
        let _ = state.record(result);
        state.error.take()
    };
    match error {
        Some(error) => {
            if code != ExitCode::from(2) {
                print_stderr(format_args!("botica: {error}\n"));
            }
            ExitCode::from(2)
        }
        None => code,
    }
}

#[cfg(test)]
mod tests;
