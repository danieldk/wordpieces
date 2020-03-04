use std::cmp;
use std::convert::TryFrom;
use std::io::{BufRead, Lines};

use ordslice::Ext;

use crate::WordPiecesError;

pub struct SortedStrings(Vec<(String, u64)>);

impl SortedStrings {
    pub fn new(strings: impl Into<Vec<(String, u64)>>) -> Self {
        let mut strings = strings.into();

        strings.sort_unstable_by(|a, b| a.0.cmp(&b.0));

        SortedStrings(strings)
    }
}

/// A set of word pieces.
pub struct WordPieces {
    word_initial: SortedStrings,
    continuation: SortedStrings,
}

impl WordPieces {
    /// Construct new word pieces instance.
    ///
    /// The arguments are set of word-initial pieces and the set o
    /// continuation pieces. The continuation set pieces must not
    /// have continuation markers (such as `##`).
    pub fn new(word_initial: SortedStrings, continuation: SortedStrings) -> Self {
        WordPieces {
            word_initial,
            continuation,
        }
    }

    fn longest_prefix_len(mut sorted_pieces: &[(String, u64)], word: &str) -> (usize, u64) {
        // We find the longest prefix length by traversing over
        // increasingly long prefixes of `word`. For each prefix, we
        // find the upper and lower bounds of that prefix in
        // `sorted_pieces`. For the next prefix we then only have to
        // search this subarray.
        //
        // In total, we at most N log K steps, where N is the length
        // of `word` and `K` the length of `sorted_pieces`. Of course,
        // in practice K will decrease drasticly for each n in 1..N.

        let mut idx = 0;
        let mut previous_prefix_len = 0;

        for (index, ch) in word.char_indices() {
            let prefix_len = index + ch.len_utf8();
            let affix = &word[previous_prefix_len..prefix_len];

            let range = sorted_pieces.equal_range_by(|probe| {
                let probe_len = probe.0.len();
                probe.0.as_bytes()[previous_prefix_len..cmp::min(probe_len, prefix_len)]
                    .cmp(affix.as_bytes())
            });

            // No match found.
            if range.start == range.end {
                return (index, idx);
            }

            idx = sorted_pieces[range.start].1;
            sorted_pieces = &sorted_pieces[range.start..range.end];

            previous_prefix_len = prefix_len;
        }

        (word.len(), idx)
    }

    /// Look up the index of an initial word piece.
    pub fn get_continuation(&self, piece: &str) -> Option<u64> {
        self.continuation
            .0
            .binary_search_by(|probe| probe.0.as_str().cmp(piece))
            .ok()
            .map(|idx| self.continuation.0[idx].1)
    }

    /// Look up the index of an continuation word piece.
    pub fn get_initial(&self, piece: &str) -> Option<u64> {
        self.word_initial
            .0
            .binary_search_by(|probe| probe.0.as_str().cmp(piece))
            .ok()
            .map(|idx| self.word_initial.0[idx].1)
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
        let mut word_initial = Vec::new();
        let mut continuation = Vec::new();

        for (idx, line) in lines.enumerate() {
            let line = line?;

            if line.starts_with("##") {
                continuation.push((line[2..].to_string(), idx as u64));
            } else {
                word_initial.push((line, idx as u64));
            }
        }

        Ok(WordPieces {
            word_initial: SortedStrings::new(word_initial),
            continuation: SortedStrings::new(continuation),
        })
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
        let (prefix_len, prefix_idx) = WordPieces::longest_prefix_len(&set.0, self.word);
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
    use std::convert::TryFrom;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    use super::{SortedStrings, WordPiece, WordPieces};

    fn example_word_pieces() -> WordPieces {
        WordPieces {
            word_initial: SortedStrings::new(vec![
                ("voor".to_string(), 0),
                ("coördina".to_string(), 2),
            ]),
            continuation: SortedStrings::new(vec![
                ("tie".to_string(), 1),
                ("kom".to_string(), 3),
                ("en".to_string(), 4),
            ]),
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
            word_initial: SortedStrings::new(vec![("foo".to_string(), 0), ("fo".to_string(), 2)]),
            continuation: SortedStrings::new(vec![
                ("o".to_string(), 1),
                ("bar".to_string(), 3),
                ("b".to_string(), 4),
                ("a".to_string(), 5),
                ("r".to_string(), 6),
            ]),
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
    fn test_word_pieces_file() {
        let f = File::open("testdata/test.pieces").unwrap();
        let word_pieces = WordPieces::try_from(BufReader::new(f).lines()).unwrap();

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
