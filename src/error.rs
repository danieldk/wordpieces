use std::io;

use failure::Fail;

/// Errors that can occur while reading word pieces.
#[derive(Debug, Fail)]
pub enum WordPiecesError {
    /// Finite state automaton error.
    #[fail(display = "Finite state automaton error: {}", _0)]
    FstError(fst::Error),

    /// IO error.
    #[fail(display = "IO error: {}", _0)]
    IOError(io::Error),
}

impl From<io::Error> for WordPiecesError {
    fn from(error: io::Error) -> Self {
        WordPiecesError::IOError(error)
    }
}

impl From<fst::Error> for WordPiecesError {
    fn from(error: fst::Error) -> Self {
        WordPiecesError::FstError(error)
    }
}
