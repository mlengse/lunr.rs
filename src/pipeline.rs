use std::sync::Arc;

use crate::token::Token;

pub type PipelineFn = Arc<dyn Fn(Token) -> PipelineResult + Send + Sync>;

#[derive(Clone)]
pub enum PipelineFunction {
    Trimmer,
    StopWordFilter,
    Stemmer,
    Custom { label: String, run: PipelineFn },
}

impl PipelineFunction {
    pub fn run(&mut self, token: Token) -> PipelineResult {
        match self {
            PipelineFunction::Trimmer => crate::trimmer::trimmer(token),
            PipelineFunction::StopWordFilter => crate::stop_word_filter::stop_word_filter(token),
            PipelineFunction::Stemmer => crate::stemmer::stemmer(token),
            PipelineFunction::Custom { run, .. } => run(token),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            PipelineFunction::Trimmer => "trimmer",
            PipelineFunction::StopWordFilter => "stopWordFilter",
            PipelineFunction::Stemmer => "stemmer",
            PipelineFunction::Custom { label, .. } => label,
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

    /// Register a plugin pipeline function (e.g. a non-English stemmer) under a
    /// label. The label is what gets serialized to the index JSON `pipeline` /
    /// `searchPipeline` arrays.
    pub fn add_custom(
        &mut self,
        label: &str,
        f: impl Fn(Token) -> PipelineResult + Send + Sync + 'static,
    ) {
        self.stack.push(PipelineFunction::Custom {
            label: label.to_string(),
            run: Arc::new(f),
        });
    }

    pub fn run(&mut self, tokens: Vec<Token>) -> Vec<Token> {
        let mut tokens = tokens;
        for f in &mut self.stack {
            let mut memo = Vec::with_capacity(tokens.len());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_stemmer_variant_runs_and_labels() {
        // A trivial "non-English" stemmer plugin: identity with a label.
        let mut p = Pipeline::new();
        p.add_custom("germanStemmer", |t: Token| PipelineResult::Token(t));

        let out = p.run_string("running");
        assert_eq!(out, vec!["running"]);
        assert_eq!(p.to_json(), vec!["germanStemmer"]);
    }

    #[test]
    fn custom_stemmer_transforms_term() {
        // Example plugin stemmer that lowercases-only, demonstrating the
        // plugin hook can fully transform tokens (here: drop a suffix).
        let mut p = Pipeline::new();
        p.add_custom("suffixStrip", |t: Token| {
            let term = t.term.trim_end_matches("ing").to_string();
            if term.is_empty() {
                PipelineResult::None
            } else {
                PipelineResult::Token(Token { term, metadata: t.metadata })
            }
        });

        assert_eq!(p.run_string("testing"), vec!["test"]);
    }
}
