//! Error-chain like mechanism, without the error-chain dependency.

use std::io;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::result;
use std::sync::mpsc;
use queue::queue_in;
use queue::queue_out;

use self::Error::*;

/// Any error that may result from dipstick usage.
#[derive(Debug)]
pub enum Error {
    /// A generic I/O error.
    IO(io::Error),
    /// An error from the async metric queue.
    Async(mpsc::SendError<queue_in::InputQueueCmd>),
    /// An error from the async metric queue.
    RawAsync(mpsc::SendError<queue_out::OutputQueueCmd>)
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> result::Result<(), fmt::Error> {
        match *self {
            IO(ref err) => err.fmt(formatter),
            Async(ref err) => err.fmt(formatter),
            RawAsync(ref err) => err.fmt(formatter),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            IO(ref err) => err.description(),
            Async(ref err) => err.description(),
            RawAsync(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            IO(ref err) => Some(err),
            Async(ref err) => Some(err),
            RawAsync(ref err) => Some(err),
        }
    }
}

/// The result type for dipstick operations that may fail.
pub type Result<T> = result::Result<T, Error>;

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        IO(err)
    }
}

impl From<mpsc::SendError<queue_in::InputQueueCmd>> for Error {
    fn from(err: mpsc::SendError<queue_in::InputQueueCmd>) -> Self {
        Async(err)
    }
}

impl From<mpsc::SendError<queue_out::OutputQueueCmd>> for Error {
    fn from(err: mpsc::SendError<queue_out::OutputQueueCmd>) -> Self {
        RawAsync(err)
    }
}
