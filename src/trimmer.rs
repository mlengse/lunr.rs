use crate::pipeline::PipelineResult;
use crate::token::Token;

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
        || ('\u{00AA}'..='\u{00BA}').contains(&c)
        || ('\u{00C0}'..='\u{00D6}').contains(&c)
        || ('\u{00D8}'..='\u{00F6}').contains(&c)
        || ('\u{00F8}'..='\u{02FF}').contains(&c)
        || ('\u{0370}'..='\u{037D}').contains(&c)
        || ('\u{037F}'..='\u{1FFF}').contains(&c)
        || ('\u{200C}'..='\u{200D}').contains(&c)
        || ('\u{2070}'..='\u{218F}').contains(&c)
        || ('\u{2C00}'..='\u{2FEF}').contains(&c)
        || ('\u{3001}'..='\u{D7FF}').contains(&c)
        || ('\u{F900}'..='\u{FDCF}').contains(&c)
        || ('\u{FDF0}'..='\u{FFFD}').contains(&c)
}

pub fn trimmer(token: Token) -> PipelineResult {
    let term = &token.term;

    // Fast path: if the whole term is already word characters (the common case
    // for tokenizer output), return it unchanged without any allocation.
    if let Some((s, _)) = term.char_indices().find(|&(_, c)| is_word_char(c)) {
        let e = term
            .char_indices()
            .rev()
            .find(|&(_, c)| is_word_char(c))
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);

        if s == 0 && e == term.len() {
            return PipelineResult::Token(token);
        }

        let trimmed = term[s..e].to_string();
        if trimmed.is_empty() {
            PipelineResult::None
        } else {
            PipelineResult::Token(Token { term: trimmed, metadata: token.metadata })
        }
    } else {
        PipelineResult::None
    }
}
