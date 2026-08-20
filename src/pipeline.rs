use crate::token::Token;

pub enum PipelineResult {
    Token(Token),
    Tokens(Vec<Token>),
    None,
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

    pub fn before(&mut self, existing_label: &str, new_label: &str, f: Box<dyn FnMut(Token) -> PipelineResult + Send + Sync>) {
        let pos = self.labels.iter().position(|l| l == existing_label)
            .expect("Cannot find existing function");
        self.stack.insert(pos, f);
        self.labels.insert(pos, new_label.to_string());
    }

    pub fn after(&mut self, existing_label: &str, new_label: &str, f: Box<dyn FnMut(Token) -> PipelineResult + Send + Sync>) {
        let pos = self.labels.iter().position(|l| l == existing_label)
            .expect("Cannot find existing function");
        self.stack.insert(pos + 1, f);
        self.labels.insert(pos + 1, new_label.to_string());
    }

    pub fn remove(&mut self, label: &str) {
        if let Some(pos) = self.labels.iter().position(|l| l == label) {
            let _ = self.stack.remove(pos);
            self.labels.remove(pos);
        }
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
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}
