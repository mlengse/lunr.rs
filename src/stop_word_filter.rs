use std::collections::HashSet;

use crate::pipeline::PipelineResult;
use crate::token::Token;

const STOP_WORDS: &[&str] = &[
    "a", "able", "about", "across", "after", "all", "almost", "also", "am", "among",
    "an", "and", "any", "are", "as", "at", "be", "because", "been", "but", "by",
    "can", "cannot", "could", "dear", "did", "do", "does", "either", "else", "ever",
    "every", "for", "from", "get", "got", "had", "has", "have", "he", "her", "hers",
    "him", "his", "how", "however", "i", "if", "in", "into", "is", "it", "its",
    "just", "least", "let", "like", "likely", "may", "me", "might", "most", "must",
    "my", "neither", "no", "nor", "not", "of", "off", "often", "on", "only", "or",
    "other", "our", "own", "rather", "said", "say", "says", "she", "should", "since",
    "so", "some", "than", "that", "the", "their", "them", "then", "there", "these",
    "they", "this", "tis", "to", "too", "twas", "us", "wants", "was", "we", "were",
    "what", "when", "where", "which", "while", "who", "whom", "why", "will", "with",
    "would", "yet", "you", "your",
];

pub fn generate_stop_word_filter(stop_words: &[&str]) -> Box<dyn FnMut(Token) -> PipelineResult + Send + Sync> {
    let words: HashSet<String> = stop_words.iter().map(|s| s.to_string()).collect();
    Box::new(move |token: Token| {
        if words.contains(&token.term) {
            PipelineResult::None
        } else {
            PipelineResult::Token(token)
        }
    })
}

pub fn stop_word_filter(token: Token) -> PipelineResult {
    if STOP_WORDS.contains(&&*token.term) {
        PipelineResult::None
    } else {
        PipelineResult::Token(token)
    }
}
