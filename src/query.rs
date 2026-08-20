#[derive(Debug, Clone)]
pub struct Query {
    pub clauses: Vec<Clause>,
    pub all_fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Clause {
    pub fields: Vec<String>,
    pub boost: f64,
    pub edit_distance: Option<usize>,
    pub use_pipeline: bool,
    pub wildcard: u8,
    pub presence: u8,
    pub term: String,
}

pub const WILDCARD_NONE: u8 = 0;
pub const WILDCARD_LEADING: u8 = 1;
pub const WILDCARD_TRAILING: u8 = 2;

pub const PRESENCE_OPTIONAL: u8 = 1;
pub const PRESENCE_REQUIRED: u8 = 2;
pub const PRESENCE_PROHIBITED: u8 = 3;

impl Query {
    pub fn new(all_fields: Vec<String>) -> Self {
        Query {
            clauses: Vec::new(),
            all_fields,
        }
    }

    pub fn clause(&mut self, clause: &mut Clause) {
        if clause.fields.is_empty() {
            clause.fields = self.all_fields.clone();
        }
        if clause.boost == 0.0 {
            clause.boost = 1.0;
        }
        if clause.wildcard == 0 {
            clause.wildcard = WILDCARD_NONE;
        }
        if clause.presence == 0 {
            clause.presence = PRESENCE_OPTIONAL;
        }

        if (clause.wildcard & WILDCARD_LEADING) != 0 && !clause.term.starts_with('*') {
            clause.term = format!("*{}", clause.term);
        }
        if (clause.wildcard & WILDCARD_TRAILING) != 0 && !clause.term.ends_with('*') {
            clause.term = format!("{}*", clause.term);
        }

        self.clauses.push(clause.clone());
    }

    pub fn term(&mut self, term: &str, options: Option<ClauseOptions>) {
        let mut clause = options.map(|o| o.to_clause()).unwrap_or_default();
        clause.term = term.to_string();
        self.clause(&mut clause);
    }

    pub fn is_negated(&self) -> bool {
        if self.clauses.is_empty() {
            return false;
        }
        self.clauses.iter().all(|c| c.presence == PRESENCE_PROHIBITED)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClauseOptions {
    pub fields: Option<Vec<String>>,
    pub boost: Option<f64>,
    pub edit_distance: Option<usize>,
    pub use_pipeline: Option<bool>,
    pub wildcard: Option<u8>,
    pub presence: Option<u8>,
}

impl ClauseOptions {
    fn to_clause(self) -> Clause {
        Clause {
            fields: self.fields.unwrap_or_default(),
            boost: self.boost.unwrap_or(1.0),
            edit_distance: self.edit_distance,
            use_pipeline: self.use_pipeline.unwrap_or(true),
            wildcard: self.wildcard.unwrap_or(WILDCARD_NONE),
            presence: self.presence.unwrap_or(PRESENCE_OPTIONAL),
            term: String::new(),
        }
    }
}

impl Default for Clause {
    fn default() -> Self {
        Clause {
            fields: Vec::new(),
            boost: 1.0,
            edit_distance: None,
            use_pipeline: true,
            wildcard: WILDCARD_NONE,
            presence: PRESENCE_OPTIONAL,
            term: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_fields() -> Vec<String> {
        vec!["title".to_string(), "body".to_string()]
    }

    #[test]
    fn clause_defaults() {
        let c = Clause::default();
        assert!(c.fields.is_empty());
        assert_eq!(1.0, c.boost);
        assert_eq!(None, c.edit_distance);
        assert!(c.use_pipeline);
        assert_eq!(WILDCARD_NONE, c.wildcard);
        assert_eq!(PRESENCE_OPTIONAL, c.presence);
        assert_eq!("", c.term);
    }

    #[test]
    fn add_clause_defaults_fields() {
        let mut q = Query::new(all_fields());
        let mut c = Clause {
            term: "foo".to_string(),
            ..Clause::default()
        };
        q.clause(&mut c);
        assert_eq!(1, q.clauses.len());
        assert_eq!(vec!["title".to_string(), "body".to_string()], q.clauses[0].fields);
        assert_eq!(1.0, q.clauses[0].boost);
        assert_eq!(PRESENCE_OPTIONAL, q.clauses[0].presence);
        assert_eq!("foo", q.clauses[0].term);
    }

    #[test]
    fn add_clause_wildcard_trailing() {
        let mut q = Query::new(all_fields());
        let mut c = Clause {
            term: "foo*".to_string(),
            wildcard: WILDCARD_TRAILING,
            ..Clause::default()
        };
        q.clause(&mut c);
        assert_eq!("foo*", q.clauses[0].term);
    }

    #[test]
    fn add_clause_wildcard_leading() {
        let mut q = Query::new(all_fields());
        let mut c = Clause {
            term: "*foo".to_string(),
            wildcard: WILDCARD_LEADING,
            ..Clause::default()
        };
        q.clause(&mut c);
        assert_eq!("*foo", q.clauses[0].term);
    }

    #[test]
    fn add_clause_wildcard_both() {
        let mut q = Query::new(all_fields());
        let mut c = Clause {
            term: "foo".to_string(),
            wildcard: WILDCARD_LEADING | WILDCARD_TRAILING,
            ..Clause::default()
        };
        q.clause(&mut c);
        assert_eq!("*foo*", q.clauses[0].term);
    }

    #[test]
    fn add_clause_boost_default() {
        let mut q = Query::new(all_fields());
        let mut c = Clause {
            term: "foo".to_string(),
            ..Clause::default()
        };
        q.clause(&mut c);
        assert_eq!(1.0, q.clauses[0].boost);
    }

    #[test]
    fn add_clause_boost_explicit() {
        let mut q = Query::new(all_fields());
        let mut c = Clause {
            term: "foo".to_string(),
            boost: 2.0,
            ..Clause::default()
        };
        q.clause(&mut c);
        assert_eq!(2.0, q.clauses[0].boost);
    }

    #[test]
    fn term_method() {
        let mut q = Query::new(all_fields());
        q.term("bar", None);
        assert_eq!(1, q.clauses.len());
        assert_eq!("bar", q.clauses[0].term);
        assert_eq!(vec!["title".to_string(), "body".to_string()], q.clauses[0].fields);
    }

    #[test]
    fn term_method_with_options() {
        let mut q = Query::new(all_fields());
        let opts = ClauseOptions {
            fields: Some(vec!["title".to_string()]),
            boost: Some(3.0),
            ..Default::default()
        };
        q.term("baz", Some(opts));
        assert_eq!(1, q.clauses.len());
        assert_eq!("baz", q.clauses[0].term);
        assert_eq!(vec!["title".to_string()], q.clauses[0].fields);
        assert_eq!(3.0, q.clauses[0].boost);
    }

    #[test]
    fn is_negated_all_prohibited() {
        let mut q = Query::new(all_fields());
        q.term("foo", Some(ClauseOptions { presence: Some(PRESENCE_PROHIBITED), ..Default::default() }));
        q.term("bar", Some(ClauseOptions { presence: Some(PRESENCE_PROHIBITED), ..Default::default() }));
        assert!(q.is_negated());
    }

    #[test]
    fn is_negated_mixed() {
        let mut q = Query::new(all_fields());
        q.term("foo", Some(ClauseOptions { presence: Some(PRESENCE_PROHIBITED), ..Default::default() }));
        q.term("bar", Some(ClauseOptions { presence: Some(PRESENCE_REQUIRED), ..Default::default() }));
        assert!(!q.is_negated());
    }

    #[test]
    fn is_negated_empty() {
        let q = Query::new(all_fields());
        assert!(!q.is_negated());
    }

    #[test]
    fn is_negated_all_optional() {
        let mut q = Query::new(all_fields());
        q.term("foo", Some(ClauseOptions { presence: Some(PRESENCE_OPTIONAL), ..Default::default() }));
        assert!(!q.is_negated());
    }
}
