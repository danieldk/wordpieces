use std::io;

use failure::Fail;

/// Errors that can occur while reading word pieces.
#[derive(Debug, Fail)]
pub enum WordPiecesError {
    /// IO error.
    #[fail(display = "IO error: {}", _0)]
    IOError(io::Error),
}

impl From<io::Error> for WordPiecesError {
    fn from(error: io::Error) -> Self {
        WordPiecesError::IOError(error)
    }
}
