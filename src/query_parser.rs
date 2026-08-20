use crate::query::{Clause, Query, PRESENCE_PROHIBITED, PRESENCE_REQUIRED};
use crate::query_parse_error::QueryParseError;
use crate::query_lexer::{Lexeme, QueryLexer, TokenType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserStateId {
    ParseClause,
    ParsePresence,
    ParseField,
    ParseTerm,
    ParseEditDistance,
    ParseBoost,
}

pub struct QueryParser {
    lexer: QueryLexer,
    query: Query,
    current_clause: Clause,
    lexeme_idx: usize,
    lexemes: Vec<Lexeme>,
}

impl QueryParser {
    pub fn new(input: &str, query: Query) -> Self {
        QueryParser {
            lexer: QueryLexer::new(input),
            query,
            current_clause: Clause::default(),
            lexeme_idx: 0,
            lexemes: Vec::new(),
        }
    }

    pub fn parse(mut self) -> Result<Query, QueryParseError> {
        self.lexer.run();
        self.lexemes = self.lexer.lexemes().to_vec();

        let mut state = Some(ParserStateId::ParseClause);
        while let Some(s) = state {
            state = s.dispatch(&mut self)?;
        }

        Ok(self.query)
    }

    fn peek_lexeme(&self) -> Option<&Lexeme> {
        self.lexemes.get(self.lexeme_idx)
    }

    fn consume_lexeme(&mut self) -> Option<Lexeme> {
        if self.lexeme_idx < self.lexemes.len() {
            let lexeme = self.lexemes[self.lexeme_idx].clone();
            self.lexeme_idx += 1;
            Some(lexeme)
        } else {
            None
        }
    }

    fn next_clause(&mut self) {
        self.query.clause(&mut self.current_clause);
        self.current_clause = Clause::default();
    }
}

impl ParserStateId {
    fn dispatch(
        self,
        parser: &mut QueryParser,
    ) -> Result<Option<ParserStateId>, QueryParseError> {
        match self {
            ParserStateId::ParseClause => parse_clause(parser),
            ParserStateId::ParsePresence => parse_presence(parser),
            ParserStateId::ParseField => parse_field(parser),
            ParserStateId::ParseTerm => parse_term(parser),
            ParserStateId::ParseEditDistance => parse_edit_distance(parser),
            ParserStateId::ParseBoost => parse_boost(parser),
        }
    }
}

fn parse_clause(parser: &mut QueryParser) -> Result<Option<ParserStateId>, QueryParseError> {
    let lexeme = match parser.peek_lexeme() {
        Some(l) => l,
        None => return Ok(None),
    };

    match lexeme.lexeme_type {
        TokenType::PRESENCE => Ok(Some(ParserStateId::ParsePresence)),
        TokenType::FIELD => Ok(Some(ParserStateId::ParseField)),
        TokenType::TERM => Ok(Some(ParserStateId::ParseTerm)),
        _ => {
            let mut msg = format!("expected either a field or a term, found {}", lexeme.lexeme_type);
            if lexeme.str.len() >= 1 {
                msg.push_str(&format!(" with value '{}'", lexeme.str));
            }
            Err(QueryParseError::new(&msg, lexeme.start, lexeme.end))
        }
    }
}

fn parse_presence(parser: &mut QueryParser) -> Result<Option<ParserStateId>, QueryParseError> {
    let lexeme = match parser.consume_lexeme() {
        Some(l) => l,
        None => return Ok(None),
    };

    match lexeme.str.as_str() {
        "-" => parser.current_clause.presence = PRESENCE_PROHIBITED,
        "+" => parser.current_clause.presence = PRESENCE_REQUIRED,
        _ => {
            let msg = format!("unrecognised presence operator'{}'", lexeme.str);
            return Err(QueryParseError::new(&msg, lexeme.start, lexeme.end));
        }
    }

    let next_lexeme = match parser.peek_lexeme() {
        Some(l) => l,
        None => {
            return Err(QueryParseError::new(
                "expecting term or field, found nothing",
                lexeme.start,
                lexeme.end,
            ));
        }
    };

    match next_lexeme.lexeme_type {
        TokenType::FIELD => Ok(Some(ParserStateId::ParseField)),
        TokenType::TERM => Ok(Some(ParserStateId::ParseTerm)),
        _ => {
            let msg = format!("expecting term or field, found '{}'", next_lexeme.lexeme_type);
            Err(QueryParseError::new(&msg, next_lexeme.start, next_lexeme.end))
        }
    }
}

fn parse_field(parser: &mut QueryParser) -> Result<Option<ParserStateId>, QueryParseError> {
    let lexeme = match parser.consume_lexeme() {
        Some(l) => l,
        None => return Ok(None),
    };

    if !parser.query.all_fields.contains(&lexeme.str) {
        let possible = parser
            .query
            .all_fields
            .iter()
            .map(|f| format!("'{}'", f))
            .collect::<Vec<_>>()
            .join(", ");
        let msg = format!("unrecognised field '{}', possible fields: {}", lexeme.str, possible);
        return Err(QueryParseError::new(&msg, lexeme.start, lexeme.end));
    }

    parser.current_clause.fields = vec![lexeme.str.clone()];

    let next_lexeme = match parser.peek_lexeme() {
        Some(l) => l,
        None => {
            return Err(QueryParseError::new(
                "expecting term, found nothing",
                lexeme.start,
                lexeme.end,
            ));
        }
    };

    match next_lexeme.lexeme_type {
        TokenType::TERM => Ok(Some(ParserStateId::ParseTerm)),
        _ => {
            let msg = format!("expecting term, found '{}'", next_lexeme.lexeme_type);
            Err(QueryParseError::new(&msg, next_lexeme.start, next_lexeme.end))
        }
    }
}

fn parse_term(parser: &mut QueryParser) -> Result<Option<ParserStateId>, QueryParseError> {
    let lexeme = match parser.consume_lexeme() {
        Some(l) => l,
        None => return Ok(None),
    };

    parser.current_clause.term = lexeme.str.to_lowercase();

    if lexeme.str.contains('*') {
        parser.current_clause.use_pipeline = false;
    }

    let next_lexeme = match parser.peek_lexeme() {
        Some(l) => l,
        None => {
            parser.next_clause();
            return Ok(None);
        }
    };

    match next_lexeme.lexeme_type {
        TokenType::TERM => {
            parser.next_clause();
            Ok(Some(ParserStateId::ParseTerm))
        }
        TokenType::FIELD => {
            parser.next_clause();
            Ok(Some(ParserStateId::ParseField))
        }
        TokenType::EDIT_DISTANCE => Ok(Some(ParserStateId::ParseEditDistance)),
        TokenType::BOOST => Ok(Some(ParserStateId::ParseBoost)),
        TokenType::PRESENCE => {
            parser.next_clause();
            Ok(Some(ParserStateId::ParsePresence))
        }
        _ => {
            let msg = format!("Unexpected lexeme type '{}'", next_lexeme.lexeme_type);
            Err(QueryParseError::new(&msg, next_lexeme.start, next_lexeme.end))
        }
    }
}

fn parse_edit_distance(parser: &mut QueryParser) -> Result<Option<ParserStateId>, QueryParseError> {
    let lexeme = match parser.consume_lexeme() {
        Some(l) => l,
        None => return Ok(None),
    };

    let edit_distance: usize = match lexeme.str.parse() {
        Ok(v) => v,
        Err(_) => {
            return Err(QueryParseError::new(
                "edit distance must be numeric",
                lexeme.start,
                lexeme.end,
            ));
        }
    };

    parser.current_clause.edit_distance = Some(edit_distance);

    let next_lexeme = match parser.peek_lexeme() {
        Some(l) => l,
        None => {
            parser.next_clause();
            return Ok(None);
        }
    };

    match next_lexeme.lexeme_type {
        TokenType::TERM => {
            parser.next_clause();
            Ok(Some(ParserStateId::ParseTerm))
        }
        TokenType::FIELD => {
            parser.next_clause();
            Ok(Some(ParserStateId::ParseField))
        }
        TokenType::EDIT_DISTANCE => Ok(Some(ParserStateId::ParseEditDistance)),
        TokenType::BOOST => Ok(Some(ParserStateId::ParseBoost)),
        TokenType::PRESENCE => {
            parser.next_clause();
            Ok(Some(ParserStateId::ParsePresence))
        }
        _ => {
            let msg = format!("Unexpected lexeme type '{}'", next_lexeme.lexeme_type);
            Err(QueryParseError::new(&msg, next_lexeme.start, next_lexeme.end))
        }
    }
}

fn parse_boost(parser: &mut QueryParser) -> Result<Option<ParserStateId>, QueryParseError> {
    let lexeme = match parser.consume_lexeme() {
        Some(l) => l,
        None => return Ok(None),
    };

    let boost: f64 = match lexeme.str.parse() {
        Ok(v) => v,
        Err(_) => {
            return Err(QueryParseError::new(
                "boost must be numeric",
                lexeme.start,
                lexeme.end,
            ));
        }
    };

    if boost <= 0.0 {
        return Err(QueryParseError::new(
            "boost must be a positive number",
            lexeme.start,
            lexeme.end,
        ));
    }

    parser.current_clause.boost = boost;

    let next_lexeme = match parser.peek_lexeme() {
        Some(l) => l,
        None => {
            parser.next_clause();
            return Ok(None);
        }
    };

    match next_lexeme.lexeme_type {
        TokenType::TERM => {
            parser.next_clause();
            Ok(Some(ParserStateId::ParseTerm))
        }
        TokenType::FIELD => {
            parser.next_clause();
            Ok(Some(ParserStateId::ParseField))
        }
        TokenType::EDIT_DISTANCE => Ok(Some(ParserStateId::ParseEditDistance)),
        TokenType::BOOST => Ok(Some(ParserStateId::ParseBoost)),
        TokenType::PRESENCE => {
            parser.next_clause();
            Ok(Some(ParserStateId::ParsePresence))
        }
        _ => {
            let msg = format!("Unexpected lexeme type '{}'", next_lexeme.lexeme_type);
            Err(QueryParseError::new(&msg, next_lexeme.start, next_lexeme.end))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{PRESENCE_OPTIONAL, PRESENCE_PROHIBITED, PRESENCE_REQUIRED};

    fn parse(input: &str) -> Result<Vec<Clause>, QueryParseError> {
        let fields = vec!["title".to_string(), "body".to_string()];
        let query = Query::new(fields);
        let parser = QueryParser::new(input, query);
        let q = parser.parse()?;
        Ok(q.clauses)
    }

    #[test]
    fn single_term() {
        let clauses = parse("foo").unwrap();
        assert_eq!(1, clauses.len());
        assert_eq!("foo", clauses[0].term);
        assert_eq!(1.0, clauses[0].boost);
        assert_eq!(PRESENCE_OPTIONAL, clauses[0].presence);
        assert_eq!(vec!["title".to_string(), "body".to_string()], clauses[0].fields);
    }

    #[test]
    fn multiple_terms() {
        let clauses = parse("foo bar").unwrap();
        assert_eq!(2, clauses.len());
        assert_eq!("foo", clauses[0].term);
        assert_eq!("bar", clauses[1].term);
    }

    #[test]
    fn field_term() {
        let clauses = parse("title:foo").unwrap();
        assert_eq!(1, clauses.len());
        assert_eq!("foo", clauses[0].term);
        assert_eq!(vec!["title".to_string()], clauses[0].fields);
    }

    #[test]
    fn presence_required() {
        let clauses = parse("+foo").unwrap();
        assert_eq!(1, clauses.len());
        assert_eq!("foo", clauses[0].term);
        assert_eq!(PRESENCE_REQUIRED, clauses[0].presence);
    }

    #[test]
    fn presence_prohibited() {
        let clauses = parse("-foo").unwrap();
        assert_eq!(1, clauses.len());
        assert_eq!("foo", clauses[0].term);
        assert_eq!(PRESENCE_PROHIBITED, clauses[0].presence);
    }

    #[test]
    fn boost() {
        let clauses = parse("foo^2").unwrap();
        assert_eq!(1, clauses.len());
        assert_eq!("foo", clauses[0].term);
        assert_eq!(2.0, clauses[0].boost);
    }

    #[test]
    fn edit_distance() {
        let clauses = parse("foo~2").unwrap();
        assert_eq!(1, clauses.len());
        assert_eq!("foo", clauses[0].term);
        assert_eq!(Some(2), clauses[0].edit_distance);
    }

    #[test]
    fn field_with_boost_and_edit_distance() {
        let clauses = parse("title:foo^2~1").unwrap();
        assert_eq!(1, clauses.len());
        assert_eq!("foo", clauses[0].term);
        assert_eq!(vec!["title".to_string()], clauses[0].fields);
        assert_eq!(2.0, clauses[0].boost);
        assert_eq!(Some(1), clauses[0].edit_distance);
    }

    #[test]
    fn wildcard_disables_pipeline() {
        let clauses = parse("foo*").unwrap();
        assert_eq!(1, clauses.len());
        assert!(!clauses[0].use_pipeline);
    }

    #[test]
    fn term_lowercased() {
        let clauses = parse("FOO").unwrap();
        assert_eq!("foo", clauses[0].term);
    }

    #[test]
    fn field_case_sensitive() {
        let result = parse("TITLE:foo");
        assert!(result.is_err());
    }

    #[test]
    fn unknown_field_error() {
        let result = parse("unknown:foo");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("unrecognised field"));
    }

    #[test]
    fn empty_field_error() {
        let result = parse("title:");
        assert!(result.is_err());
    }

    #[test]
    fn bad_edit_distance_error() {
        let result = parse("foo~a");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("edit distance must be numeric"));
    }

    #[test]
    fn negative_edit_distance_error() {
        let result = parse("foo~-1");
        assert!(result.is_err());
    }

    #[test]
    fn bad_boost_error() {
        let result = parse("foo^a");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("boost must be numeric"));
    }

    #[test]
    fn negative_boost_error() {
        let result = parse("foo^-1");
        assert!(result.is_err());
    }

    #[test]
    fn zero_boost_error() {
        let result = parse("foo^0");
        assert!(result.is_err());
    }

    #[test]
    fn complex_query() {
        let clauses = parse("+title:bar^2 baz~1 -foo").unwrap();
        assert_eq!(3, clauses.len());

        assert_eq!("bar", clauses[0].term);
        assert_eq!(vec!["title".to_string()], clauses[0].fields);
        assert_eq!(PRESENCE_REQUIRED, clauses[0].presence);
        assert_eq!(2.0, clauses[0].boost);

        assert_eq!("baz", clauses[1].term);
        assert_eq!(PRESENCE_OPTIONAL, clauses[1].presence);
        assert_eq!(Some(1), clauses[1].edit_distance);

        assert_eq!("foo", clauses[2].term);
        assert_eq!(PRESENCE_PROHIBITED, clauses[2].presence);
    }
}
