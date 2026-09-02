use std::fmt;
use std::convert::From;
use std::collections::HashMap;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

pub type Metadata = Value;

#[derive(Debug, Clone)]
pub struct Tokens(Vec<Token>);

impl Tokens {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// Tokenizer over a configurable separator character set.
///
/// Defaults to splitting on whitespace and hyphens, matching lunr.js
/// `tokenizer.separator = /[\s-]+/`. The separator is a set of single
/// characters treated as delimiters; the default includes the common
/// whitespace characters plus `-`.
#[derive(Clone)]
pub struct Tokenizer {
    separators: Vec<char>,
}

impl Default for Tokenizer {
    fn default() -> Self {
        Tokenizer {
            separators: vec![' ', '\t', '\r', '\n', '\u{0B}', '\u{0C}', '-'],
        }
    }
}

impl Tokenizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the separator character set, e.g. `" ,"` to split on spaces
    /// and commas. Empty string means never split (a single token).
    pub fn with_separator(mut self, separators: &str) -> Self {
        self.separators = separators.chars().collect();
        self
    }

    /// A predicate marking whether a character is a token separator.
    pub fn is_separator(&self, ch: char) -> bool {
        self.separators.contains(&ch)
    }

    /// Tokenize `text` into a sequence: NFC-normalize, lowercase, then split
    /// on the configured separator set.
    pub fn tokenize(&self, text: &str) -> Tokens {
        // Fast path: pure ASCII needs no NFC normalization (identity on
        // ASCII), so skip the unicode_normalization machinery entirely.
        if text.is_ascii() {
            return self.tokenize_ascii(text);
        }
        // NFC-normalize and lowercase in a single pass so only one String is
        // allocated for the whole corpus (previously two: NFC result, then
        // to_lowercase result).
        let normalized: String = text.nfc().flat_map(char::to_lowercase).collect();
        self.split(&normalized)
    }

    fn tokenize_ascii(&self, text: &str) -> Tokens {
        let lower = text.to_ascii_lowercase();
        self.split(&lower)
    }

    /// Split a normalized (already NFC'd + lowercased) string on the configured
    /// separator set.
    fn split(&self, normalized: &str) -> Tokens {
        let mut tokens = Vec::new();
        let mut start = 0;
        let len = normalized.len();

        for (end, ch) in normalized.char_indices() {
            if self.is_separator(ch) {
                if end > start {
                    tokens.push(Token::new(&normalized[start..end]));
                }
                start = end + ch.len_utf8();
            }
        }

        if start < len {
            tokens.push(Token::new(&normalized[start..len]));
        }

        Tokens(tokens)
    }
}

impl From<String> for Tokens {
    fn from(text: String) -> Tokens {
        Tokenizer::default().tokenize(&text)
    }
}

impl<'a> From<&'a str> for Tokens {
    fn from(text: &'a str) -> Tokens {
        text.to_owned().into()
    }
}

impl IntoIterator for Tokens {
    type Item = Token;
    type IntoIter = ::std::vec::IntoIter<Token>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub type Term = String;

#[derive(Clone)]
pub struct Token {
    pub term: Term,
    pub metadata: HashMap<String, Metadata>,
}

impl Token {
    pub fn new<S: Into<String>>(term: S) -> Token {
        Token {
            term: term.into(),
            metadata: HashMap::new(),
        }
    }

    pub fn update<F: FnMut(String, HashMap<String, Metadata>) -> String>(&mut self, mut f: F) {
        self.term = f(self.term.clone(), self.metadata.clone());
    }

    pub fn clone_with<F: FnMut(String, HashMap<String, Metadata>) -> String>(&self, mut f: F) -> Token {
        Token {
            term: f(self.term.clone(), self.metadata.clone()),
            metadata: self.metadata.clone(),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.term)
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Token({})", self.term)
    }
}

impl PartialEq for Token {
    fn eq(&self, other: &Token) -> bool {
        self.term == other.term
    }
}

impl Eq for Token {}

impl Ord for Token {
    fn cmp(&self, other: &Token) -> Ordering {
        self.term.cmp(&other.term)
    }
}

impl PartialOrd for Token {
    fn partial_cmp(&self, other: &Token) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Token {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.term.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_string() {
        let tokens: Tokens = "foo bar baz".into();
        let mut iter = tokens.into_iter().map(|t| t.term);

        assert_eq!(Some(String::from("foo")), iter.next());
        assert_eq!(Some(String::from("bar")), iter.next());
        assert_eq!(Some(String::from("baz")), iter.next());

        assert!(iter.next().is_none());
    }

    #[test]
    fn token_metadata() {
        let mut token = Token::new("foo");
        token.metadata.insert("string".into(), Value::String("string".into()));
        token.metadata.insert("number".into(), Value::Number(123.into()));
    }

    #[test]
    fn nfc_normalizes_decomposed_accents() {
        // e + combining acute (U+00E9 decomposed U+0065 U+0301) composes to é.
        let tokens: Tokens = "cafe\u{301}".into();
        let terms: Vec<String> = tokens.into_iter().map(|t| t.term).collect();
        assert_eq!(terms, vec!["caf\u{e9}"]);
    }

    #[test]
    fn nfc_leaves_ascii_untouched() {
        let tokens: Tokens = "Hello World".into();
        let terms: Vec<String> = tokens.into_iter().map(|t| t.term).collect();
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn default_separator_splits_whitespace_and_hyphen() {
        let tokens: Tokens = "foo bar-baz".into();
        let terms: Vec<String> = tokens.into_iter().map(|t| t.term).collect();
        assert_eq!(terms, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn custom_separator_overrides_default() {
        let tokenizer = Tokenizer::new().with_separator(" ,");
        let tokens = tokenizer.tokenize("foo,bar baz");
        let terms: Vec<String> = tokens.into_iter().map(|t| t.term).collect();
        assert_eq!(terms, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn custom_separator_keeps_hyphens() {
        let tokenizer = Tokenizer::new().with_separator(" ");
        let tokens = tokenizer.tokenize("foo-bar baz");
        let terms: Vec<String> = tokens.into_iter().map(|t| t.term).collect();
        assert_eq!(terms, vec!["foo-bar", "baz"]);
    }
}
