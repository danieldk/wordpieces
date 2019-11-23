use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::io::{self, BufRead, Lines};

use failure::Fail;
use fst::{self, IntoStreamer, Set, SetBuilder, Streamer};

mod automaton;
use automaton::PrefixAutomaton;

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

/// A set of word pieces.
pub struct WordPieces {
    prefixes: Set,
    suffixes: Set,
}

impl WordPieces {
    /// Construct new word pieces instance.
    ///
    /// The arguments are the prefix and suffix set. The suffix set
    /// should not have suffix markers (`##`).
    pub fn new(prefixes: Set, suffixes: Set) -> Self {
        WordPieces { prefixes, suffixes }
    }

    fn longest_prefix_len(affix_set: &Set, word: &str) -> usize {
        let mut stream = affix_set.search(PrefixAutomaton::from(word)).into_stream();

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
    pub fn split(&self, mut word: &str) -> Result<Vec<String>, Vec<String>> {
        let mut pieces = Vec::new();

        // Find the word's prefix
        let prefix_len = Self::longest_prefix_len(&self.prefixes, word);
        if prefix_len == 0 {
            return Err(pieces);
        }
        pieces.push(word[..prefix_len].to_string());
        word = &word[prefix_len..];

        // Find the word's suffixes.
        while !word.is_empty() {
            let prefix_len = Self::longest_prefix_len(&self.suffixes, word);
            if prefix_len == 0 {
                return Err(pieces);
            }

            pieces.push(format!("##{}", word[..prefix_len].to_string()));
            word = &word[prefix_len..];
        }

        Ok(pieces)
    }
}

impl<R> TryFrom<Lines<R>> for WordPieces
where
    R: BufRead,
{
    type Error = WordPiecesError;

    fn try_from(lines: Lines<R>) -> Result<Self, Self::Error> {
        let mut prefixes = BTreeSet::new();
        let mut suffixes = BTreeSet::new();

        for line in lines {
            let line = line?;

            if line.starts_with("##") {
                suffixes.insert(line[2..].to_string());
            } else {
                prefixes.insert(line);
            }
        }

        let mut prefix_set = SetBuilder::memory();
        prefix_set.extend_iter(prefixes)?;

        let mut suffix_set = SetBuilder::memory();
        suffix_set.extend_iter(suffixes)?;

        Ok(WordPieces {
            prefixes: Set::from_bytes(prefix_set.into_inner()?)?,
            suffixes: Set::from_bytes(suffix_set.into_inner()?)?,
        })
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

    use super::WordPieces;

    fn affixes_to_set(affixes: &[&str]) -> Set {
        let affixes = BTreeSet::from_iter(affixes);
        let mut builder = SetBuilder::memory();
        builder.extend_iter(affixes).unwrap();
        Set::from_bytes(builder.into_inner().unwrap()).unwrap()
    }

    fn example_word_pieces() -> WordPieces {
        WordPieces {
            prefixes: affixes_to_set(&["voor", "coördina"]),
            suffixes: affixes_to_set(&["tie", "kom", "en"]),
        }
    }

    #[test]
    fn test_word_pieces() {
        let word_pieces = example_word_pieces();

        assert_eq!(word_pieces.split("voor"), Ok(vec!["voor".to_string()]));
        assert_eq!(word_pieces.split("unknown"), Err(Vec::<String>::new()));
        assert_eq!(word_pieces.split("voorman"), Err(vec!["voor".to_string()]));
        assert_eq!(
            word_pieces.split("coördinatie"),
            Ok(vec!["coördina".to_string(), "##tie".to_string()])
        );
        assert_eq!(
            word_pieces.split("voorkomen"),
            Ok(vec![
                "voor".to_string(),
                "##kom".to_string(),
                "##en".to_string()
            ])
        );
    }

    #[test]
    fn longest_prefix_used() {
        let word_pieces = WordPieces {
            prefixes: affixes_to_set(&["foo", "fo"]),
            suffixes: affixes_to_set(&["o", "bar", "b", "a", "r"]),
        };

        assert_eq!(
            word_pieces.split("foobar"),
            Ok(vec!["foo".to_string(), "##bar".to_string()])
        );
    }

    #[test]
    fn test_word_pieces_file() {
        let f = File::open("testdata/test.pieces").unwrap();
        let word_pieces = WordPieces::try_from(BufReader::new(f).lines()).unwrap();

        assert_eq!(word_pieces.split("voor"), Ok(vec!["voor".to_string()]));
        assert_eq!(word_pieces.split("unknown"), Err(Vec::<String>::new()));
        assert_eq!(word_pieces.split("voorman"), Err(vec!["voor".to_string()]));
        assert_eq!(
            word_pieces.split("coördinatie"),
            Ok(vec!["coördina".to_string(), "##tie".to_string()])
        );
    }
}
