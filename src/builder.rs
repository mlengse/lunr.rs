use crate::document::Document;
use crate::field_ref::FieldRef;
use crate::idf;
use crate::inverted_index::InvertedIndex;
use crate::pipeline::Pipeline;
use crate::vector::Vector;

use crate::token::{Term, Tokens};
use std::collections::{HashMap, HashSet};

pub fn create() -> Builder {
    Builder::default()
}

pub struct Builder {
    pub inverted_index: InvertedIndex,
    pub field_vectors: HashMap<FieldRef, Vector>,
    pub fields: Vec<String>,
    pub pipeline: Pipeline,

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

            for term in term_frequencies.keys() {
                let tf = f64::from(*term_frequencies.get(term).expect("token frequency missing"));
                let posting = self.inverted_index.posting(term).expect("posting missing");
                let idf = idf::idf(posting, document_count);
                let score = idf * (tf * (self.k1 + 1.0))
                    / (tf + self.k1 * (1.0 - self.b + self.b * field_length as f64 / avg_field_length));

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
}
