use std::collections::HashSet;
use std::convert::TryFrom;
use std::io::{self, BufRead, Lines};

/// A set of word pieces.
pub struct WordPieces {
    prefixes: HashSet<String>,
    suffixes: HashSet<String>,
}

impl WordPieces {
    /// Construct new word pieces instance.
    ///
    /// The arguments are the prefix and suffix set. The prefix set
    /// should have the suffix marker `##` removed.
    pub fn new(prefixes: HashSet<String>, suffixes: HashSet<String>) -> Self {
        WordPieces { prefixes, suffixes }
    }

    /// Split a string into word pieces.
    pub fn split(&self, word: &str) -> Result<Vec<String>, Vec<String>> {
        // Get character offsets into `word`. Add the length of the
        // word, for the upper bound of the string.
        let mut char_indices: Vec<usize> = word.char_indices().map(|(idx, _)| idx).collect();
        char_indices.push(word.len());

        let mut pieces = Vec::new();
        let mut begin = 0;
        while begin < (char_indices.len() - 1) {
            let mut end = char_indices.len() - 1;

            while begin < end {
                let candidate_piece = &word[char_indices[begin]..char_indices[end]];

                if begin == 0 {
                    // Prefix
                    if self.prefixes.contains(candidate_piece) {
                        pieces.push(candidate_piece.to_owned());
                        break;
                    }
                } else {
                    // Suffix
                    if self.suffixes.contains(candidate_piece) {
                        pieces.push(format!("##{}", candidate_piece.to_owned()));
                        break;
                    }
                }

                end -= 1;
            }

            if begin == end {
                // No valid prefix could be found, return partial results.
                return Err(pieces);
            } else {
                begin = end;
            }
        }

        Ok(pieces)
    }
}

impl<R> TryFrom<Lines<R>> for WordPieces
where
    R: BufRead,
{
    type Error = io::Error;

    fn try_from(lines: Lines<R>) -> Result<Self, Self::Error> {
        let mut prefixes = HashSet::new();
        let mut suffixes = HashSet::new();

        for line in lines {
            let line = line?;

            if line.starts_with("##") {
                suffixes.insert(line[2..].to_string());
            } else {
                prefixes.insert(line);
            }
        }

        Ok(WordPieces { prefixes, suffixes })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::convert::TryFrom;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::iter::FromIterator;

    use super::WordPieces;

    fn example_word_pieces() -> WordPieces {
        WordPieces {
            prefixes: HashSet::from_iter(
                (&["voor".to_string(), "coördina".to_string()])
                    .iter()
                    .map(Clone::clone),
            ),
            suffixes: HashSet::from_iter((&["tie".to_string()]).iter().map(Clone::clone)),
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
