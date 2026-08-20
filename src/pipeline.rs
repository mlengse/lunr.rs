use crate::token::Token;

#[derive(Clone)]
pub enum PipelineFunction {
    Trimmer,
    StopWordFilter,
    Stemmer,
}

impl PipelineFunction {
    pub fn run(&mut self, token: Token) -> PipelineResult {
        match self {
            PipelineFunction::Trimmer => crate::trimmer::trimmer(token),
            PipelineFunction::StopWordFilter => crate::stop_word_filter::stop_word_filter(token),
            PipelineFunction::Stemmer => crate::stemmer::stemmer(token),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            PipelineFunction::Trimmer => "trimmer",
            PipelineFunction::StopWordFilter => "stopWordFilter",
            PipelineFunction::Stemmer => "stemmer",
        }
    }
}

pub enum PipelineResult {
    Token(Token),
    Tokens(Vec<Token>),
    None,
}

#[derive(Clone)]
pub struct Pipeline {
    stack: Vec<PipelineFunction>,
}

impl Pipeline {
    pub fn new() -> Self {
        Pipeline { stack: Vec::new() }
    }

    pub fn add_function(&mut self, func: PipelineFunction) {
        self.stack.push(func);
    }

    pub fn run(&mut self, tokens: Vec<Token>) -> Vec<Token> {
        let mut tokens = tokens;
        for f in &mut self.stack {
            let mut memo = Vec::new();
            for token in tokens {
                match f.run(token) {
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

    pub fn to_json(&self) -> Vec<String> {
        self.stack.iter().map(|f| f.label().to_string()).collect()
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}
