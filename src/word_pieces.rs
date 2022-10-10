use std::collections::BTreeMap;
use std::io::BufRead;

use fst::raw::Output;
use fst::{self, Map, MapBuilder, Streamer};

use crate::WordPiecesError;

pub struct WordPiecesBuilder {
    word_initial: BTreeMap<String, u64>,
    continuation: BTreeMap<String, u64>,
}

impl Default for WordPiecesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WordPiecesBuilder {
    pub fn new() -> Self {
        WordPiecesBuilder {
            word_initial: BTreeMap::new(),
            continuation: BTreeMap::new(),
        }
    }

    pub fn build(self) -> Result<WordPieces, WordPiecesError> {
        let mut word_initial_set = MapBuilder::memory();
        word_initial_set.extend_iter(self.word_initial)?;

        let mut continuation_set = MapBuilder::memory();
        continuation_set.extend_iter(self.continuation)?;

        Ok(WordPieces {
            word_initial: Map::new(word_initial_set.into_inner()?)?,
            continuation: Map::new(continuation_set.into_inner()?)?,
        })
    }

    pub fn insert(&mut self, piece: &str, idx: u64) {
        match piece.strip_prefix("##") {
            Some(stripped) => self.continuation.insert(stripped.to_string(), idx as u64),
            None => self.word_initial.insert(piece.to_owned(), idx as u64),
        };
    }
}

/// A set of word pieces.
pub struct WordPieces {
    word_initial: Map<Vec<u8>>,
    continuation: Map<Vec<u8>>,
}

impl WordPieces {
    /// Construct new word pieces instance.
    ///
    /// The arguments are set of word-initial pieces and the set o
    /// continuation pieces. The continuation set pieces must not
    /// have continuation markers (such as `##`).
    pub fn new(word_initial: Map<Vec<u8>>, continuation: Map<Vec<u8>>) -> Self {
        WordPieces {
            word_initial,
            continuation,
        }
    }

    pub fn from_buf_read(buf_read: impl BufRead) -> Result<Self, WordPiecesError> {
        let mut builder = WordPiecesBuilder::new();

        for (idx, piece) in buf_read.lines().enumerate() {
            let piece = piece?;
            builder.insert(&piece, idx as u64);
        }

        builder.build()
    }

    fn longest_prefix_len<D>(piece_map: &Map<D>, word: &str) -> (usize, u64)
    where
        D: AsRef<[u8]>,
    {
        let fst = piece_map.as_fst();

        let mut node = fst.root();
        let mut out = Output::zero();
        let mut longest_prefix = 0;
        let mut longest_prefix_out = Output::zero();

        for (idx, &byte) in word.as_bytes().iter().enumerate() {
            // Attempt to move to the next state.
            match node.find_input(byte) {
                Some(trans_idx) => {
                    let trans = node.transition(trans_idx);

                    out = out.cat(trans.out);
                    node = fst.node(trans.addr);
                }
                None => return (longest_prefix, longest_prefix_out.value()),
            };

            // We have found the next prefix, save it.
            if node.is_final() {
                longest_prefix = idx + 1;
                longest_prefix_out = node.final_output().cat(out);
            }
        }

        (longest_prefix, longest_prefix_out.value())
    }

    /// Look up the index of an initial word piece.
    pub fn get_continuation(&self, piece: &str) -> Option<u64> {
        self.continuation.get(piece)
    }

    /// Look up the index of an continuation word piece.
    pub fn get_initial(&self, piece: &str) -> Option<u64> {
        self.word_initial.get(piece)
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

impl From<&WordPieces> for Vec<String> {
    fn from(word_pieces: &WordPieces) -> Self {
        let mut pieces =
            vec![String::new(); word_pieces.word_initial.len() + word_pieces.continuation.len()];

        let mut stream = word_pieces.word_initial.stream();
        while let Some((piece, idx)) = stream.next() {
            pieces[idx as usize] =
                String::from_utf8(piece.to_owned()).expect("Pieces should all be valid UTF-8")
        }

        stream = word_pieces.continuation.stream();
        while let Some((piece, idx)) = stream.next() {
            let piece =
                String::from_utf8(piece.to_owned()).expect("Pieces should all be valid UTF-8");
            pieces[idx as usize] = format!("##{}", piece);
        }

        pieces
    }
}

/// A single word piece.
#[derive(Debug, Eq, PartialEq)]
pub enum WordPiece<'a> {
    /// The next found word piece.
    Found { piece: &'a str, idx: u64 },

    /// No piece was found for the (remaining part of) the word.
    Missing,
}

impl<'a> WordPiece<'a> {
    /// Unwrap an index if present.
    pub fn idx(&self) -> Option<u64> {
        match self {
            WordPiece::Found { idx, .. } => Some(*idx),
            WordPiece::Missing => None,
        }
    }

    /// Unwrap a piece if present.
    pub fn piece(&self) -> Option<&'a str> {
        match self {
            WordPiece::Found { piece, .. } => Some(piece),
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
        let (prefix_len, prefix_idx) = WordPieces::longest_prefix_len(set, self.word);
        if prefix_len == 0 {
            // If there is no matching set, empty the word.
            self.word = &self.word[self.word.len()..];
            return Some(WordPiece::Missing);
        }

        let piece = &self.word[..prefix_len];

        self.word = &self.word[prefix_len..];

        Some(WordPiece::Found {
            piece,
            idx: prefix_idx,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use std::iter::FromIterator;
    use std::{collections::BTreeMap, io::BufRead};

    use fst::{Map, MapBuilder};

    use crate::WordPiecesBuilder;

    use super::{WordPiece, WordPieces};

    fn pieces_to_map(pieces: &[(&str, u64)]) -> Map<Vec<u8>> {
        let pieces =
            BTreeMap::from_iter(pieces.iter().map(|(piece, idx)| (piece.to_string(), *idx)));
        let mut builder = MapBuilder::memory();
        builder.extend_iter(pieces).unwrap();
        Map::new(builder.into_inner().unwrap()).unwrap()
    }

    fn example_word_pieces() -> WordPieces {
        WordPieces {
            word_initial: pieces_to_map(&[("voor", 0), ("coördina", 2)]),
            continuation: pieces_to_map(&[("tie", 1), ("kom", 3), ("en", 4), ("komt", 1)]),
        }
    }

    #[test]
    fn test_word_pieces() {
        let word_pieces = example_word_pieces();

        assert_eq!(
            word_pieces.split("voor").collect::<Vec<_>>(),
            vec![WordPiece::Found {
                piece: "voor",
                idx: 0
            }]
        );
        assert_eq!(
            word_pieces.split("unknown").collect::<Vec<_>>(),
            vec![WordPiece::Missing]
        );
        assert_eq!(
            word_pieces.split("voorman").collect::<Vec<_>>(),
            vec![
                WordPiece::Found {
                    piece: "voor",
                    idx: 0
                },
                WordPiece::Missing
            ]
        );
        assert_eq!(
            word_pieces.split("coördinatie").collect::<Vec<_>>(),
            vec![
                WordPiece::Found {
                    piece: "coördina",
                    idx: 2
                },
                WordPiece::Found {
                    piece: "tie",
                    idx: 1
                }
            ]
        );
        assert_eq!(
            word_pieces.split("voorkomen").collect::<Vec<_>>(),
            vec![
                WordPiece::Found {
                    piece: "voor",
                    idx: 0,
                },
                WordPiece::Found {
                    piece: "kom",
                    idx: 3
                },
                WordPiece::Found {
                    piece: "en",
                    idx: 4
                },
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
            word_initial: pieces_to_map(&[("foo", 0), ("fo", 2)]),
            continuation: pieces_to_map(&[("o", 1), ("bar", 3), ("b", 4), ("a", 5), ("r", 6)]),
        };

        assert_eq!(
            word_pieces.split("foobar").collect::<Vec<_>>(),
            vec![
                WordPiece::Found {
                    piece: "foo",
                    idx: 0
                },
                WordPiece::Found {
                    piece: "bar",
                    idx: 3
                }
            ]
        );
    }

    #[test]
    fn test_original_pieces_are_returned() {
        let f = File::open("testdata/test.pieces").unwrap();
        let pieces = BufReader::new(f)
            .lines()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut builder = WordPiecesBuilder::default();
        for (idx, piece) in pieces.iter().enumerate() {
            builder.insert(piece, idx as u64);
        }
        let wordpieces = builder.build().unwrap();
        assert_eq!(Vec::from(&wordpieces), pieces);
    }

    #[test]
    fn test_word_pieces_file() {
        let f = File::open("testdata/test.pieces").unwrap();
        let word_pieces = WordPieces::from_buf_read(BufReader::new(f)).unwrap();

        assert_eq!(
            word_pieces.split("voor").collect::<Vec<_>>(),
            vec![WordPiece::Found {
                piece: "voor",
                idx: 0
            }]
        );
        assert_eq!(
            word_pieces.split("unknown").collect::<Vec<_>>(),
            vec![WordPiece::Missing]
        );
        assert_eq!(
            word_pieces.split("voorman").collect::<Vec<_>>(),
            vec![
                WordPiece::Found {
                    piece: "voor",
                    idx: 0
                },
                WordPiece::Missing
            ]
        );
        assert_eq!(
            word_pieces.split("coördinatie").collect::<Vec<_>>(),
            vec![
                WordPiece::Found {
                    piece: "coördina",
                    idx: 2
                },
                WordPiece::Found {
                    piece: "tie",
                    idx: 1
                }
            ]
        );
    }
}
