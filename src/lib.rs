use std::borrow::Cow;
use std::collections::HashSet;

/// A set of word pieces.
pub struct WordPieces(pub HashSet<String>);

impl WordPieces {
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
                let candidate_piece = if begin != 0 {
                    Cow::Owned(format!(
                        "##{}",
                        &word[char_indices[begin]..char_indices[end]]
                    ))
                } else {
                    Cow::Borrowed(&word[..char_indices[end]])
                };

                if self.0.contains(candidate_piece.as_ref()) {
                    pieces.push(candidate_piece.into_owned());
                    break;
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::iter::FromIterator;

    use super::WordPieces;

    fn example_word_pieces() -> WordPieces {
        WordPieces(HashSet::from_iter(
            (&[
                "voor".to_string(),
                "coördina".to_string(),
                "##tie".to_string(),
            ])
                .iter()
                .map(Clone::clone),
        ))
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
}
