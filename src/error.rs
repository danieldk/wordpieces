use std::io;

use thiserror::Error;

/// Errors that can occur while reading word pieces.
#[derive(Debug, Error)]
pub enum WordPiecesError {
    /// Finite state automaton error.
    #[error(transparent)]
    FstError(#[from] fst::Error),

    /// IO error.
    #[error(transparent)]
    IOError(#[from] io::Error),
}
