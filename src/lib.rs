mod automaton;
use automaton::PrefixAutomaton;

mod error;
pub use error::WordPiecesError;

mod word_pieces;
pub use word_pieces::{WordPiece, WordPieces};
