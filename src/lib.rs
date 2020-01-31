//! Tokenize words into word pieces.
//!
//! This crate provides a subword tokenizer. A subword tokenizer
//! splits a token into several pieces, so-called *word pieces*.  Word
//! pieces were popularized by and used in the
//! [BERT](https://arxiv.org/abs/1810.04805) natural language encoder.
//!
//! The tokenizer splits a word, providing an iterator over pieces.
//! The piece is represented as a string and its vocabulary index.
//!
//! ~~~
//! use std::convert::TryFrom;
//! use std::fs::File;
//! use std::io::{BufRead, BufReader};
//!
//! use wordpieces::{WordPiece, WordPieces};
//!
//! let f = File::open("testdata/test.pieces").unwrap();
//! let word_pieces = WordPieces::try_from(BufReader::new(f).lines()).unwrap();
//!
//! // A word that can be split fully.
//! let pieces = word_pieces.split("coördinatie")
//!  .map(|p| p.piece()).collect::<Vec<_>>();
//! assert_eq!(pieces, vec![Some("coördina"), Some("tie")]);
//!
//! // A word that can be split partially.
//! let pieces = word_pieces.split("voorkomen")
//!  .map(|p| p.piece()).collect::<Vec<_>>();
//! assert_eq!(pieces, vec![Some("voor"), None]);
//! ~~~

mod automaton;
use automaton::PrefixAutomaton;

mod error;
pub use error::WordPiecesError;

mod word_pieces;
pub use word_pieces::{WordPiece, WordPieces};
