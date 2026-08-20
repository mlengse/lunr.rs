#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum TokenType {
    EOS,
    FIELD,
    TERM,
    EDIT_DISTANCE,
    BOOST,
    PRESENCE,
}

impl std::fmt::Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TokenType::EOS => write!(f, "EOS"),
            TokenType::FIELD => write!(f, "FIELD"),
            TokenType::TERM => write!(f, "TERM"),
            TokenType::EDIT_DISTANCE => write!(f, "EDIT_DISTANCE"),
            TokenType::BOOST => write!(f, "BOOST"),
            TokenType::PRESENCE => write!(f, "PRESENCE"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lexeme {
    pub lexeme_type: TokenType,
    pub str: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexerStateId {
    LexText,
    LexField,
    LexTerm,
    LexEditDistance,
    LexBoost,
    LexEos,
}

pub struct QueryLexer {
    lexemes: Vec<Lexeme>,
    input: String,
    length: usize,
    pos: usize,
    start: usize,
    escape_char_positions: Vec<usize>,
}

const EOS_CHAR: char = '\0';

impl QueryLexer {
    pub fn new(input: &str) -> Self {
        QueryLexer {
            lexemes: Vec::new(),
            input: input.to_string(),
            length: input.len(),
            pos: 0,
            start: 0,
            escape_char_positions: Vec::new(),
        }
    }

    pub fn run(&mut self) {
        let mut state = Some(LexerStateId::LexText);
        while let Some(s) = state {
            state = match s {
                LexerStateId::LexText => lex_text(self),
                LexerStateId::LexField => lex_field(self),
                LexerStateId::LexTerm => lex_term(self),
                LexerStateId::LexEditDistance => lex_edit_distance(self),
                LexerStateId::LexBoost => lex_boost(self),
                LexerStateId::LexEos => lex_eos(self),
            };
        }
    }

    pub fn lexemes(&self) -> &[Lexeme] {
        &self.lexemes
    }

    pub fn next(&mut self) -> char {
        if self.pos >= self.length {
            return EOS_CHAR;
        }
        let ch = self.input.as_bytes()[self.pos] as char;
        self.pos += 1;
        ch
    }

    pub fn backup(&mut self) {
        self.pos -= 1;
    }

    pub fn ignore(&mut self) {
        if self.start == self.pos {
            self.pos += 1;
        }
        self.start = self.pos;
    }

    pub fn width(&self) -> usize {
        self.pos - self.start
    }

    pub fn emit(&mut self, token_type: TokenType) {
        let s = self.slice_string();
        let start = self.start;
        let end = self.pos;
        self.lexemes.push(Lexeme {
            lexeme_type: token_type,
            str: s,
            start,
            end,
        });
        self.start = self.pos;
    }

    pub fn slice_string(&mut self) -> String {
        let mut result = String::new();
        let mut slice_start = self.start;

        for &esc_pos in &self.escape_char_positions {
            let slice_end = esc_pos;
            result.push_str(&self.input[slice_start..slice_end]);
            slice_start = esc_pos + 1;
        }

        result.push_str(&self.input[slice_start..self.pos]);
        self.escape_char_positions.clear();
        result
    }

    pub fn escape_character(&mut self) {
        self.escape_char_positions.push(self.pos - 1);
        self.pos += 1;
    }

    pub fn accept_digit_run(&mut self) {
        loop {
            let ch = self.next();
            if ch == EOS_CHAR {
                break;
            }
            let code = ch as u32;
            if code < 48 || code > 57 {
                self.backup();
                break;
            }
        }
    }

    pub fn accept_decimal_run(&mut self) {
        self.accept_digit_run();
        if self.more() {
            let ch = self.input.as_bytes()[self.pos] as char;
            if ch == '.' {
                self.pos += 1;
                self.accept_digit_run();
            }
        }
    }

    pub fn more(&self) -> bool {
        self.pos < self.length
    }
}

fn is_separator(ch: char) -> bool {
    ch.is_whitespace() || ch == '-'
}

fn lex_text(lexer: &mut QueryLexer) -> Option<LexerStateId> {
    loop {
        let ch = lexer.next();

        if ch == EOS_CHAR {
            return Some(LexerStateId::LexEos);
        }

        if ch == '\\' {
            lexer.escape_character();
            continue;
        }

        if ch == ':' {
            return Some(LexerStateId::LexField);
        }

        if ch == '~' {
            lexer.backup();
            if lexer.width() > 0 {
                lexer.emit(TokenType::TERM);
            }
            return Some(LexerStateId::LexEditDistance);
        }

        if ch == '^' {
            lexer.backup();
            if lexer.width() > 0 {
                lexer.emit(TokenType::TERM);
            }
            return Some(LexerStateId::LexBoost);
        }

        if ch == '+' && lexer.width() == 1 {
            lexer.emit(TokenType::PRESENCE);
            return Some(LexerStateId::LexText);
        }

        if ch == '-' && lexer.width() == 1 {
            lexer.emit(TokenType::PRESENCE);
            return Some(LexerStateId::LexText);
        }

        if is_separator(ch) {
            return Some(LexerStateId::LexTerm);
        }
    }
}

fn lex_field(lexer: &mut QueryLexer) -> Option<LexerStateId> {
    lexer.backup();
    lexer.emit(TokenType::FIELD);
    lexer.ignore();
    Some(LexerStateId::LexText)
}

fn lex_term(lexer: &mut QueryLexer) -> Option<LexerStateId> {
    if lexer.width() > 1 {
        lexer.backup();
        lexer.emit(TokenType::TERM);
    }

    lexer.ignore();

    if lexer.more() {
        Some(LexerStateId::LexText)
    } else {
        Some(LexerStateId::LexEos)
    }
}

fn lex_edit_distance(lexer: &mut QueryLexer) -> Option<LexerStateId> {
    lexer.ignore();
    lexer.accept_digit_run();
    lexer.emit(TokenType::EDIT_DISTANCE);
    Some(LexerStateId::LexText)
}

fn lex_boost(lexer: &mut QueryLexer) -> Option<LexerStateId> {
    lexer.ignore();
    lexer.accept_decimal_run();
    lexer.emit(TokenType::BOOST);
    Some(LexerStateId::LexText)
}

fn lex_eos(lexer: &mut QueryLexer) -> Option<LexerStateId> {
    if lexer.width() > 0 {
        lexer.emit(TokenType::TERM);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(input: &str) -> Vec<Lexeme> {
        let mut lexer = QueryLexer::new(input);
        lexer.run();
        lexer.lexemes().to_vec()
    }

    #[test]
    fn single_term() {
        let lexemes = lex("foo");
        assert_eq!(1, lexemes.len());
        assert_eq!(TokenType::TERM, lexemes[0].lexeme_type);
        assert_eq!("foo", lexemes[0].str);
    }

    #[test]
    fn multiple_terms() {
        let lexemes = lex("foo bar");
        assert_eq!(2, lexemes.len());
        assert_eq!(TokenType::TERM, lexemes[0].lexeme_type);
        assert_eq!("foo", lexemes[0].str);
        assert_eq!(TokenType::TERM, lexemes[1].lexeme_type);
        assert_eq!("bar", lexemes[1].str);
    }

    #[test]
    fn field_and_term() {
        let lexemes = lex("title:foo");
        assert_eq!(2, lexemes.len());
        assert_eq!(TokenType::FIELD, lexemes[0].lexeme_type);
        assert_eq!("title", lexemes[0].str);
        assert_eq!(TokenType::TERM, lexemes[1].lexeme_type);
        assert_eq!("foo", lexemes[1].str);
    }

    #[test]
    fn presence_plus() {
        let lexemes = lex("+foo");
        assert_eq!(2, lexemes.len());
        assert_eq!(TokenType::PRESENCE, lexemes[0].lexeme_type);
        assert_eq!("+", lexemes[0].str);
        assert_eq!(TokenType::TERM, lexemes[1].lexeme_type);
        assert_eq!("foo", lexemes[1].str);
    }

    #[test]
    fn presence_minus() {
        let lexemes = lex("-foo");
        assert_eq!(2, lexemes.len());
        assert_eq!(TokenType::PRESENCE, lexemes[0].lexeme_type);
        assert_eq!("-", lexemes[0].str);
        assert_eq!(TokenType::TERM, lexemes[1].lexeme_type);
        assert_eq!("foo", lexemes[1].str);
    }

    #[test]
    fn edit_distance() {
        let lexemes = lex("foo~2");
        assert_eq!(2, lexemes.len());
        assert_eq!(TokenType::TERM, lexemes[0].lexeme_type);
        assert_eq!("foo", lexemes[0].str);
        assert_eq!(TokenType::EDIT_DISTANCE, lexemes[1].lexeme_type);
        assert_eq!("2", lexemes[1].str);
    }

    #[test]
    fn boost() {
        let lexemes = lex("foo^2");
        assert_eq!(2, lexemes.len());
        assert_eq!(TokenType::TERM, lexemes[0].lexeme_type);
        assert_eq!("foo", lexemes[0].str);
        assert_eq!(TokenType::BOOST, lexemes[1].lexeme_type);
        assert_eq!("2", lexemes[1].str);
    }

    #[test]
    fn field_term_boost_edit_distance() {
        let lexemes = lex("title:foo^2~1");
        assert_eq!(4, lexemes.len());
        assert_eq!(TokenType::FIELD, lexemes[0].lexeme_type);
        assert_eq!("title", lexemes[0].str);
        assert_eq!(TokenType::TERM, lexemes[1].lexeme_type);
        assert_eq!("foo", lexemes[1].str);
        assert_eq!(TokenType::BOOST, lexemes[2].lexeme_type);
        assert_eq!("2", lexemes[2].str);
        assert_eq!(TokenType::EDIT_DISTANCE, lexemes[3].lexeme_type);
        assert_eq!("1", lexemes[3].str);
    }

    #[test]
    fn presence_field_term() {
        let lexemes = lex("+title:foo");
        assert_eq!(3, lexemes.len());
        assert_eq!(TokenType::PRESENCE, lexemes[0].lexeme_type);
        assert_eq!("+", lexemes[0].str);
        assert_eq!(TokenType::FIELD, lexemes[1].lexeme_type);
        assert_eq!("title", lexemes[1].str);
        assert_eq!(TokenType::TERM, lexemes[2].lexeme_type);
        assert_eq!("foo", lexemes[2].str);
    }

    #[test]
    fn complex_query() {
        let lexemes = lex("title:bar^2 baz~1 +foo -bar");
        assert_eq!(9, lexemes.len());
        assert_eq!(TokenType::FIELD, lexemes[0].lexeme_type);
        assert_eq!("title", lexemes[0].str);
        assert_eq!(TokenType::TERM, lexemes[1].lexeme_type);
        assert_eq!("bar", lexemes[1].str);
        assert_eq!(TokenType::BOOST, lexemes[2].lexeme_type);
        assert_eq!("2", lexemes[2].str);
        assert_eq!(TokenType::TERM, lexemes[3].lexeme_type);
        assert_eq!("baz", lexemes[3].str);
        assert_eq!(TokenType::EDIT_DISTANCE, lexemes[4].lexeme_type);
        assert_eq!("1", lexemes[4].str);
        assert_eq!(TokenType::PRESENCE, lexemes[5].lexeme_type);
        assert_eq!("+", lexemes[5].str);
        assert_eq!(TokenType::TERM, lexemes[6].lexeme_type);
        assert_eq!("foo", lexemes[6].str);
        assert_eq!(TokenType::PRESENCE, lexemes[7].lexeme_type);
        assert_eq!("-", lexemes[7].str);
        assert_eq!(TokenType::TERM, lexemes[8].lexeme_type);
        assert_eq!("bar", lexemes[8].str);
    }

    #[test]
    fn hyphen_separator() {
        let lexemes = lex("foo-bar");
        assert_eq!(2, lexemes.len());
        assert_eq!(TokenType::TERM, lexemes[0].lexeme_type);
        assert_eq!("foo", lexemes[0].str);
        assert_eq!(TokenType::TERM, lexemes[1].lexeme_type);
        assert_eq!("bar", lexemes[1].str);
    }

    #[test]
    fn wildcard_term() {
        let lexemes = lex("foo*");
        assert_eq!(1, lexemes.len());
        assert_eq!(TokenType::TERM, lexemes[0].lexeme_type);
        assert_eq!("foo*", lexemes[0].str);
    }

    #[test]
    fn empty_input() {
        let lexemes = lex("");
        assert_eq!(0, lexemes.len());
    }
}
