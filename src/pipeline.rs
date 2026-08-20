use std::collections::HashMap;

use crate::token::Token;

pub enum PipelineResult {
    Token(Token),
    Tokens(Vec<Token>),
    None,
}

pub trait PipelineFunction: FnMut(Token) -> PipelineResult + Send + Sync {
    fn label(&self) -> &str;
}

pub struct Pipeline {
    stack: Vec<Box<dyn FnMut(Token) -> PipelineResult + Send + Sync>>,
    labels: Vec<String>,
}

impl Pipeline {
    pub fn new() -> Self {
        Pipeline {
            stack: Vec::new(),
            labels: Vec::new(),
        }
    }

    pub fn add(&mut self, label: &str, f: Box<dyn FnMut(Token) -> PipelineResult + Send + Sync>) {
        self.stack.push(f);
        self.labels.push(label.to_string());
    }

    pub fn run(&mut self, tokens: Vec<Token>) -> Vec<Token> {
        let mut tokens = tokens;
        for f in &mut self.stack {
            let mut memo = Vec::new();
            for token in tokens {
                match f(token) {
                    PipelineResult::Token(t) => memo.push(t),
                    PipelineResult::Tokens(ts) => memo.extend(ts),
                    PipelineResult::None => {}
                }
            }
            tokens = memo;
        }
        tokens
    }

    pub fn run_string(&mut self, text: &str) -> Vec<String> {
        let token = Token::new(text);
        let tokens = vec![token];
        self.run(tokens)
            .into_iter()
            .map(|t| t.term)
            .collect()
    }

    pub fn reset(&mut self) {
        self.stack.clear();
        self.labels.clear();
    }

    pub fn to_json(&self) -> Vec<String> {
        self.labels.clone()
    }

    pub fn load(labels: &[String], registry: &HashMap<String, Box<dyn FnMut(Token) -> PipelineResult + Send + Sync>>) -> Self {
        let mut _pipeline = Pipeline::new();
        for label in labels {
            if let Some(_f) = registry.get(label) {
                // Note: We can't clone boxed trait objects, so this is a placeholder
                // In practice, we'd need a registry of factory functions
                eprintln!("Warning: Pipeline::load not yet implemented for dynamic registry");
            } else {
                panic!("Cannot load unregistered function: {}", label);
            }
        }
        _pipeline
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}
