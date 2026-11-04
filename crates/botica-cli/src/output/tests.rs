use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

use super::Output;

#[derive(Default)]
struct Destination {
    bytes: Vec<u8>,
    write_error: Option<io::ErrorKind>,
    flush_error: Option<io::ErrorKind>,
    remaining: Option<usize>,
}

#[derive(Clone, Default)]
struct Writer(Rc<RefCell<Destination>>);

impl Write for Writer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut destination = self.0.borrow_mut();
        let count = destination
            .remaining
            .unwrap_or(bytes.len())
            .min(bytes.len());
        if count == 0 || destination.remaining.is_none() {
            if let Some(kind) = destination.write_error {
                if kind == io::ErrorKind::Interrupted {
                    destination.write_error = None;
                }
                return Err(kind.into());
            }
        }
        destination.bytes.extend_from_slice(&bytes[..count]);
        if let Some(remaining) = &mut destination.remaining {
            *remaining -= count;
        }
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.0.borrow().flush_error {
            Some(kind) => Err(kind.into()),
            None => Ok(()),
        }
    }
}

#[test]
fn writes_reach_an_open_destination() {
    let writer = Writer::default();
    let mut output = Output::new(writer.clone());

    output.write_all(b"hello").expect("write output");
    output.flush().expect("flush output");

    assert_eq!(writer.0.borrow().bytes, b"hello");
}

#[test]
fn a_broken_pipe_discards_the_remaining_bytes_and_all_later_output() {
    for remaining in [0, 3] {
        let writer = Writer::default();
        writer.0.borrow_mut().remaining = Some(remaining);
        writer.0.borrow_mut().write_error = Some(io::ErrorKind::BrokenPipe);
        let mut output = Output::new(writer.clone());

        output.write_all(b"hello").expect("discard closed output");
        writer.0.borrow_mut().remaining = None;
        writer.0.borrow_mut().write_error = None;
        writer.0.borrow_mut().flush_error = Some(io::ErrorKind::PermissionDenied);
        output.write_all(b"later").expect("keep discarding");
        output.flush().expect("no flush after broken pipe");

        assert_eq!(writer.0.borrow().bytes, &b"hello"[..remaining]);
    }
}

#[test]
fn a_broken_pipe_on_flush_also_discards_later_output() {
    let writer = Writer::default();
    writer.0.borrow_mut().flush_error = Some(io::ErrorKind::BrokenPipe);
    let mut output = Output::new(writer.clone());
    output.write_all(b"hello").expect("write output");

    output.flush().expect("discard broken flush");
    writer.0.borrow_mut().flush_error = Some(io::ErrorKind::PermissionDenied);
    output.flush().expect("keep discarding flush");
    output.write_all(b"later").expect("keep discarding output");

    assert_eq!(writer.0.borrow().bytes, b"hello");
}

#[test]
fn other_write_and_flush_errors_are_not_discarded() {
    let writer = Writer::default();
    writer.0.borrow_mut().write_error = Some(io::ErrorKind::PermissionDenied);
    let mut output = Output::new(writer.clone());

    assert_eq!(
        output.write_all(b"hello").unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    writer.0.borrow_mut().write_error = None;
    writer.0.borrow_mut().flush_error = Some(io::ErrorKind::PermissionDenied);
    assert_eq!(
        output.flush().unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    output.write_all(b"later").expect("writer remains open");
    assert_eq!(writer.0.borrow().bytes, b"later");
}

#[test]
fn interrupted_writes_are_retried_without_discarding_output() {
    let writer = Writer::default();
    writer.0.borrow_mut().write_error = Some(io::ErrorKind::Interrupted);
    let mut output = Output::new(writer.clone());

    output.write_all(b"hello").expect("retry interruption");

    assert_eq!(writer.0.borrow().bytes, b"hello");
}
