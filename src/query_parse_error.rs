use std::fmt;

#[derive(Debug, Clone)]
pub struct QueryParseError {
    pub name: String,
    pub message: String,
    pub start: usize,
    pub end: usize,
}

impl QueryParseError {
    pub fn new(message: &str, start: usize, end: usize) -> Self {
        QueryParseError {
            name: "QueryParseError".to_string(),
            message: message.to_string(),
            start,
            end,
        }
    }
}

impl fmt::Display for QueryParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {} at position {}-{}", self.name, self.message, self.start, self.end)
    }
}

impl std::error::Error for QueryParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_parse_error_shape() {
        let err = QueryParseError::new("unexpected character", 3, 6);
        assert_eq!("QueryParseError", err.name);
        assert_eq!("unexpected character", err.message);
        assert_eq!(3, err.start);
        assert_eq!(6, err.end);
    }

    #[test]
    fn query_parse_error_display() {
        let err = QueryParseError::new("unrecognised field 'x'", 0, 1);
        assert_eq!("QueryParseError: unrecognised field 'x' at position 0-1", format!("{}", err));
    }

    #[test]
    fn query_parse_error_is_std_error() {
        let err = QueryParseError::new("test", 0, 0);
        let _e: &dyn std::error::Error = &err;
    }
}
