use crate::document::Document;
use crate::field_ref::FieldRef;
use crate::idf;
use crate::inverted_index::InvertedIndex;
use crate::pipeline::Pipeline;
use crate::vector::Vector;

use crate::token::{Term, Tokens};
use std::collections::{HashMap, HashSet};

pub struct FieldOpts {
    pub boost: f64,
}

impl Default for FieldOpts {
    fn default() -> Self {
        FieldOpts { boost: 1.0 }
    }
}

pub fn create() -> Builder {
    Builder::default()
}

struct FieldConfig {
    name: String,
    boost: f64,
}

pub struct Builder {
    pub inverted_index: InvertedIndex,
    pub field_vectors: HashMap<FieldRef, Vector>,
    pub fields: Vec<String>,
    pub pipeline: Pipeline,
    pub search_pipeline: Pipeline,

    field_configs: Vec<FieldConfig>,
    term_frequencies: HashMap<FieldRef, HashMap<Term, u32>>,
    field_lengths: HashMap<FieldRef, usize>,
    field_refs: Vec<FieldRef>,
    document_refs: HashSet<String>,

    k1: f64,
    b: f64,
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            inverted_index: InvertedIndex::default(),
            field_vectors: HashMap::new(),
            fields: Vec::new(),
            pipeline: Pipeline::new(),
            search_pipeline: Pipeline::new(),
            field_configs: Vec::new(),
            term_frequencies: HashMap::new(),
            field_lengths: HashMap::new(),
            field_refs: Vec::new(),
            document_refs: HashSet::new(),
            k1: 1.2,
            b: 0.75,
        }
    }
}

impl Builder {
    pub fn field(&mut self, name: &str, opts: FieldOpts) {
        if self.field_configs.iter().any(|f| f.name == name) {
            panic!("already registered field '{}'", name);
        }
        self.field_configs.push(FieldConfig {
            name: name.to_string(),
            boost: opts.boost,
        });
    }

    pub fn add<T: Document>(&mut self, document: T) {
        let doc_ref = document.id();
        assert!(!doc_ref.is_empty(),
            "cannot add a document without a 'ref' field to the index");
        assert!(self.document_refs.insert(doc_ref.clone()),
            "cannot add a document with a duplicate ref '{}'", doc_ref);

        for field in document.fields() {
            let field_ref = FieldRef::new(doc_ref.clone(), field.name.to_owned());
            let tokens: Tokens = field.text.into();
            let field_length = tokens.len();

            if !self.fields.contains(&field.name) {
                self.fields.push(field.name);
            }

            *self.field_lengths.entry(field_ref.clone()).or_insert(0) += field_length;

            let mut pipeline = std::mem::take(&mut self.pipeline);
            let processed_tokens: Vec<_> = pipeline.run(tokens.into_iter().collect());
            self.pipeline = pipeline;

            for token in processed_tokens {
                *self.term_frequencies
                     .entry(field_ref.clone())
                     .or_insert_with(HashMap::new)
                     .entry(token.term.to_owned())
                     .or_insert(0) += 1;

                self.inverted_index.add(field_ref.clone(), token);
            }

            self.field_refs.push(field_ref);
        }
    }

    pub fn build(&mut self) {
        assert!(!self.field_refs.is_empty(),
            "cannot build index with no documents");

        let avg_field_length = self.calculate_average_field_length();
        let document_count = self.field_lengths.len();

        for field_ref in &self.field_refs {
            let mut vector: Vector = Default::default();
            let term_frequencies =
                self.term_frequencies.get(field_ref).expect("token frequencies missing");
            let field_length = *self.field_lengths.get(field_ref).expect("field length missing");

            let field_boost = self.field_configs
                .iter()
                .find(|f| f.name == field_ref.field_name)
                .map(|f| f.boost)
                .unwrap_or(1.0);

            for term in term_frequencies.keys() {
                let tf = f64::from(*term_frequencies.get(term).expect("token frequency missing"));
                let posting = self.inverted_index.posting(term).expect("posting missing");
                let idf = idf::idf(posting, document_count);
                let score = idf * (tf * (self.k1 + 1.0))
                    / (tf + self.k1 * (1.0 - self.b + self.b * field_length as f64 / avg_field_length))
                    * field_boost;

                vector.insert(posting.index as u32, score);
            }

            self.field_vectors.insert(field_ref.clone(), vector);
        }
    }

    fn calculate_average_field_length(&self) -> f64 {
        if self.field_lengths.is_empty() {
            return 0.0;
        }
        let total: usize = self.field_lengths.values().sum();
        total as f64 / self.field_lengths.len() as f64
    }

    pub fn b(&mut self, n: f64) {
        self.b = n.clamp(0.0, 1.0);
    }

    pub fn k1(&mut self, n: f64) {
        self.k1 = n.max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Document, Field};

    struct TestDoc {
        id: String,
        text: String,
    }

    impl Document for TestDoc {
        fn id(&self) -> String { self.id.clone() }
        fn fields(&self) -> Vec<Field> {
            vec![Field { name: "text".into(), text: self.text.clone() }]
        }
    }

    fn doc(id: &str, text: &str) -> TestDoc {
        TestDoc { id: id.into(), text: text.into() }
    }

    #[test]
    fn b_clamps_to_valid_range() {
        let mut b = Builder::default();
        b.b(2.0);
        b.add(doc("a", "hello world"));
        b.build();
        // b is clamped to 1.0, should not panic
    }

    #[test]
    fn b_clamps_negative() {
        let mut b = Builder::default();
        b.b(-0.5);
        b.add(doc("a", "hello world"));
        b.build();
        // b is clamped to 0.0
    }

    #[test]
    fn k1_clamps_negative() {
        let mut b = Builder::default();
        b.k1(-1.0);
        b.add(doc("a", "hello world"));
        b.build();
        // k1 is clamped to 0.0
    }

    #[test]
    fn field_boost_applied() {
        let mut b1 = Builder::default();
        b1.field("text", FieldOpts { boost: 1.0 });
        b1.add(doc("a", "hello world"));
        b1.add(doc("b", "hello there"));
        b1.build();

        let mut b2 = Builder::default();
        b2.field("text", FieldOpts { boost: 2.0 });
        b2.add(doc("a", "hello world"));
        b2.add(doc("b", "hello there"));
        b2.build();

        let arr1 = b1.field_vectors.values().next().unwrap().to_array();
        let arr2 = b2.field_vectors.values().next().unwrap().to_array();
        assert_eq!(arr1.len(), arr2.len());
        for (v1, v2) in arr1.iter().zip(arr2.iter()) {
            assert!((v2 - v1 * 2.0).abs() < 1e-10, "boost 2x should double score");
        }
    }

    #[test]
    #[should_panic(expected = "already registered field")]
    fn field_duplicate_panics() {
        let mut b = Builder::default();
        b.field("text", FieldOpts::default());
        b.field("text", FieldOpts::default());
    }

    #[test]
    fn search_pipeline_transferred_to_index() {
        let index = crate::lunr(|b| {
            b.field("text", FieldOpts::default());
            b.add(doc("a", "hello world"));
        });
        assert_eq!(index.search_pipeline.to_json(), vec!["stemmer"]);
        assert_eq!(index.pipeline.to_json(), vec!["trimmer", "stopWordFilter", "stemmer"]);
    }
}
