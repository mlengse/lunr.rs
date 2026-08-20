use std::fmt;
use std::convert::From;
use std::collections::HashMap;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use serde_json::Value;

pub type Metadata = Value;

#[derive(Debug, Clone)]
pub struct Tokens(Vec<Token>);

impl Tokens {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<String> for Tokens {
    fn from(text: String) -> Tokens {
        let normalized = text.to_lowercase();
        let mut tokens = Vec::new();
        let mut start = 0;
        let mut index = 0;

        for (end, ch) in normalized.char_indices() {
            if ch.is_whitespace() || ch == '-' {
                if end > start {
                    let term = normalized[start..end].to_string();
                    let mut metadata = HashMap::new();
                    metadata.insert("position".to_string(), Value::Array(vec![
                        Value::Number(start.into()),
                        Value::Number((end - start).into()),
                    ]));
                    metadata.insert("index".to_string(), Value::Number(index.into()));
                    tokens.push(Token { term, metadata });
                    index += 1;
                }
                start = end + ch.len_utf8();
            }
        }

        if start < normalized.len() {
            let term = normalized[start..].to_string();
            let mut metadata = HashMap::new();
            metadata.insert("position".to_string(), Value::Array(vec![
                Value::Number(start.into()),
                Value::Number((normalized.len() - start).into()),
            ]));
            metadata.insert("index".to_string(), Value::Number(index.into()));
            tokens.push(Token { term, metadata });
        }

        Tokens(tokens)
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
}
