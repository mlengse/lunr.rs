use std::collections::HashMap;
use crate::token::Metadata;

#[derive(Debug, Clone, Default)]
pub struct MatchData {
    pub metadata: HashMap<String, HashMap<String, HashMap<String, Vec<Metadata>>>>,
}

impl MatchData {
    pub fn new(term: Option<&str>, field: &str, metadata: &HashMap<String, Vec<Metadata>>) -> Self {
        let mut match_data = MatchData {
            metadata: HashMap::new(),
        };

        if let Some(term) = term {
            let term_metadata = match_data.metadata.entry(term.to_string()).or_insert_with(HashMap::new);
            let field_metadata = term_metadata.entry(field.to_string()).or_insert_with(HashMap::new);
            for (key, values) in metadata {
                field_metadata.insert(key.clone(), values.clone());
            }
        }

        match_data
    }

    pub fn add(&mut self, term: &str, field: &str, metadata: &HashMap<String, Vec<Metadata>>) {
        if !self.metadata.contains_key(term) {
            self.metadata.insert(term.to_string(), HashMap::new());
        }
        let term_metadata = self.metadata.get_mut(term).unwrap();

        if !term_metadata.contains_key(field) {
            term_metadata.insert(field.to_string(), HashMap::new());
        }
        let field_metadata = term_metadata.get_mut(field).unwrap();

        for (key, values) in metadata {
            if let Some(existing) = field_metadata.get_mut(key) {
                existing.extend(values.iter().cloned());
            } else {
                field_metadata.insert(key.clone(), values.clone());
            }
        }
    }

    pub fn combine(&mut self, other: &MatchData) {
        for (term, fields) in &other.metadata {
            if !self.metadata.contains_key(term) {
                self.metadata.insert(term.clone(), HashMap::new());
            }
            let term_metadata = self.metadata.get_mut(term).unwrap();

            for (field, keys) in fields {
                if !term_metadata.contains_key(field) {
                    term_metadata.insert(field.clone(), HashMap::new());
                }
                let field_metadata = term_metadata.get_mut(field).unwrap();

                for (key, values) in keys {
                    if let Some(existing) = field_metadata.get_mut(key) {
                        existing.extend(values.iter().cloned());
                    } else {
                        field_metadata.insert(key.clone(), values.clone());
                    }
                }
            }
        }
    }
}
