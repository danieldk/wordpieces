use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::io::{BufRead, Lines};

use fst::{self, IntoStreamer, Set, SetBuilder, Streamer};

use crate::{PrefixAutomaton, WordPiecesError};

/// A set of word pieces.
pub struct WordPieces {
    word_initial: Set,
    continuation: Set,
}

impl WordPieces {
    /// Construct new word pieces instance.
    ///
    /// The arguments are set of word-initial pieces and the set o
    /// continuation pieces. The continuation set pieces must not
    /// have continuation markers (such as `##`).
    pub fn new(word_initial: Set, continuation: Set) -> Self {
        WordPieces {
            word_initial,
            continuation,
        }
    }

    fn longest_prefix_len(piece_set: &Set, word: &str) -> usize {
        let mut stream = piece_set.search(PrefixAutomaton::from(word)).into_stream();

        let mut longest_len = match stream.next() {
            Some(prefix) => prefix.len(),
            None => return 0,
        };

        while let Some(prefix) = stream.next() {
            if prefix.len() > longest_len {
                longest_len = prefix.len()
            }
        }

        longest_len
    }

    /// Split a string into word pieces.
    ///
    /// Returns an iterator over the word pieces.
    pub fn split<'a, 'b>(&'a self, word: &'b str) -> WordPieceIter<'a, 'b> {
        WordPieceIter {
            word_pieces: self,
            word,
            initial: true,
        }
    }
}

impl<R> TryFrom<Lines<R>> for WordPieces
where
    R: BufRead,
{
    type Error = WordPiecesError;

    fn try_from(lines: Lines<R>) -> Result<Self, Self::Error> {
        let mut word_initial = BTreeSet::new();
        let mut continuation = BTreeSet::new();

        for line in lines {
            let line = line?;

            if line.starts_with("##") {
                continuation.insert(line[2..].to_string());
            } else {
                word_initial.insert(line);
            }
        }

        let mut word_initial_set = SetBuilder::memory();
        word_initial_set.extend_iter(word_initial)?;

        let mut continuation_set = SetBuilder::memory();
        continuation_set.extend_iter(continuation)?;

        Ok(WordPieces {
            word_initial: Set::from_bytes(word_initial_set.into_inner()?)?,
            continuation: Set::from_bytes(continuation_set.into_inner()?)?,
        })
    }
}

/// A single word piece.
#[derive(Debug, Eq, PartialEq)]
pub enum WordPiece<'a> {
    /// The next found word piece.
    Found(&'a str),

    /// No piece was found for the (remaining part of) the word.
    Missing,
}

impl<'a> WordPiece<'a> {
    /// Unwrap a piece if present.
    pub fn piece(&self) -> Option<&'a str> {
        match self {
            WordPiece::Found(piece) => Some(piece),
            WordPiece::Missing => None,
        }
    }
}

impl<'a> From<&WordPiece<'a>> for Option<&'a str> {
    fn from(word_piece: &WordPiece<'a>) -> Self {
        word_piece.piece()
    }
}

/// Iterator over word pieces.
pub struct WordPieceIter<'a, 'b> {
    word_pieces: &'a WordPieces,

    /// The remaining word.
    word: &'b str,

    /// Is this the initial word piece?
    initial: bool,
}

impl<'a, 'b> Iterator for WordPieceIter<'a, 'b> {
    type Item = WordPiece<'b>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.word.is_empty() {
            assert!(
                !self.initial,
                "Cannot break an empty string into word pieces"
            );
            return None;
        }

        // Pick the word-initial or continuation set.
        let set = if self.initial {
            self.initial = false;
            &self.word_pieces.word_initial
        } else {
            &self.word_pieces.continuation
        };

        // Find the word's prefix in the set.
        let prefix_len = WordPieces::longest_prefix_len(set, self.word);
        if prefix_len == 0 {
            // If there is no matching set, empty the word.
            self.word = &self.word[self.word.len()..];
            return Some(WordPiece::Missing);
        }

        let piece = &self.word[..prefix_len];

        self.word = &self.word[prefix_len..];

        Some(WordPiece::Found(piece))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::convert::TryFrom;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::iter::FromIterator;

    use fst::{Set, SetBuilder};

    use super::{WordPiece, WordPieces};

    fn pieces_to_set(pieces: &[&str]) -> Set {
        let pieces = BTreeSet::from_iter(pieces);
        let mut builder = SetBuilder::memory();
        builder.extend_iter(pieces).unwrap();
        Set::from_bytes(builder.into_inner().unwrap()).unwrap()
    }

    fn example_word_pieces() -> WordPieces {
        WordPieces {
            word_initial: pieces_to_set(&["voor", "coördina"]),
            continuation: pieces_to_set(&["tie", "kom", "en"]),
        }
    }

    #[test]
    fn test_word_pieces() {
        let word_pieces = example_word_pieces();

        assert_eq!(
            word_pieces.split("voor").collect::<Vec<_>>(),
            vec![WordPiece::Found("voor")]
        );
        assert_eq!(
            word_pieces.split("unknown").collect::<Vec<_>>(),
            vec![WordPiece::Missing]
        );
        assert_eq!(
            word_pieces.split("voorman").collect::<Vec<_>>(),
            vec![WordPiece::Found("voor"), WordPiece::Missing]
        );
        assert_eq!(
            word_pieces.split("coördinatie").collect::<Vec<_>>(),
            vec![WordPiece::Found("coördina"), WordPiece::Found("tie")]
        );
        assert_eq!(
            word_pieces.split("voorkomen").collect::<Vec<_>>(),
            vec![
                WordPiece::Found("voor"),
                WordPiece::Found("kom"),
                WordPiece::Found("en")
            ]
        );
    }

    #[test]
    #[should_panic]
    fn splitting_empty_should_panic() {
        let word_pieces = example_word_pieces();
        assert_eq!(word_pieces.split("").collect::<Vec<_>>(), vec![]);
    }

    #[test]
    fn longest_prefix_used() {
        let word_pieces = WordPieces {
            word_initial: pieces_to_set(&["foo", "fo"]),
            continuation: pieces_to_set(&["o", "bar", "b", "a", "r"]),
        };

        assert_eq!(
            word_pieces.split("foobar").collect::<Vec<_>>(),
            vec![WordPiece::Found("foo"), WordPiece::Found("bar")]
        );
    }

    #[test]
    fn test_word_pieces_file() {
        let f = File::open("testdata/test.pieces").unwrap();
        let word_pieces = WordPieces::try_from(BufReader::new(f).lines()).unwrap();

        assert_eq!(
            word_pieces.split("voor").collect::<Vec<_>>(),
            vec![WordPiece::Found("voor")]
        );
        assert_eq!(
            word_pieces.split("unknown").collect::<Vec<_>>(),
            vec![WordPiece::Missing]
        );
        assert_eq!(
            word_pieces.split("voorman").collect::<Vec<_>>(),
            vec![WordPiece::Found("voor"), WordPiece::Missing]
        );
        assert_eq!(
            word_pieces.split("coördinatie").collect::<Vec<_>>(),
            vec![WordPiece::Found("coördina"), WordPiece::Found("tie")]
        );
    }
}
