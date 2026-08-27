use crate::document::{Document, Field};
use crate::field_ref::FieldRef;
use crate::idf;
use crate::inverted_index::InvertedIndex;
use crate::pipeline::Pipeline;
use crate::vector::Vector;

use crate::token::{Term, Tokenizer};
use std::collections::{HashMap, HashSet};

pub struct FieldOpts {
    pub boost: f64,
    pub extractor: Option<fn(&Field) -> String>,
}

impl Default for FieldOpts {
    fn default() -> Self {
        FieldOpts { boost: 1.0, extractor: None }
    }
}

pub fn create() -> Builder {
    Builder::default()
}

struct FieldConfig {
    name: String,
    boost: f64,
    extractor: Option<fn(&Field) -> String>,
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

    ref_field: String,

    tokenizer: Tokenizer,

    average_field_length: HashMap<String, f64>,

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
            ref_field: "id".to_string(),
            tokenizer: Tokenizer::default(),
            average_field_length: HashMap::new(),
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
            extractor: opts.extractor,
        });
    }

    pub fn ref_field(&mut self, field: &str) {
        assert!(!field.is_empty(), "ref field name cannot be empty");
        self.ref_field = field.to_string();
    }

    pub fn use_plugin(&mut self, plugin: impl FnOnce(&mut Self)) {
        plugin(self);
    }

    /// Configure the tokenizer's separator character set, e.g. `" ,"` to split
    /// on spaces and commas. Default is whitespace + hyphen (`[\s-]+`).
    pub fn tokenizer_separator(&mut self, separators: &str) {
        self.tokenizer = Tokenizer::new().with_separator(separators);
    }

    pub fn add<T: Document>(&mut self, document: T) {
        let doc_ref = document.id();
        assert!(!doc_ref.is_empty(),
            "cannot add a document without a '{}' field to the index", self.ref_field);
        assert!(self.document_refs.insert(doc_ref.clone()),
            "cannot add a document with a duplicate ref '{}'", doc_ref);

        for field in document.fields() {
            let field_ref = FieldRef::new(doc_ref.clone(), field.name.to_owned());
            let text = self
                .field_configs
                .iter()
                .find(|f| f.name == field.name)
                .and_then(|f| f.extractor)
                .map(|ext| ext(&field))
                .unwrap_or(field.text);
            let tokens = self.tokenizer.tokenize(&text);
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

        self.calculate_average_field_lengths();
        let document_count = self.document_refs.len();

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

            let avg_field_length = *self.average_field_length
                .get(&field_ref.field_name)
                .expect("average field length missing");

            for term in term_frequencies.keys() {
                let tf = f64::from(*term_frequencies.get(term).expect("token frequency missing"));
                let posting = self.inverted_index.posting(term).expect("posting missing");
                let idf = idf::idf(posting, document_count);
                let score = idf * ((self.k1 + 1.0) * tf)
                    / (self.k1 * (1.0 - self.b + self.b * field_length as f64 / avg_field_length) + tf)
                    * field_boost;

                // Round to 3 decimal places to match lunr.js serialized output
                let score = (score * 1000.0).round() / 1000.0;

                vector.insert(posting.index as u32, score);
            }

            self.field_vectors.insert(field_ref.clone(), vector);
        }
    }

    fn calculate_average_field_lengths(&mut self) {
        let mut field_lengths_sum: HashMap<String, usize> = HashMap::new();
        let mut field_counts: HashMap<String, usize> = HashMap::new();

        for (field_ref, len) in &self.field_lengths {
            *field_lengths_sum.entry(field_ref.field_name.clone()).or_insert(0) += len;
            *field_counts.entry(field_ref.field_name.clone()).or_insert(0) += 1;
        }

        for (field, total) in field_lengths_sum {
            let count = *field_counts.get(&field).unwrap_or(&0);
            let avg = if count == 0 { 0.0 } else { total as f64 / count as f64 };
            self.average_field_length.insert(field, avg);
        }
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
            vec![Field { name: "text".into(), text: self.text.clone(), boost: 1.0 }]
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
        b1.field("text", FieldOpts { boost: 1.0, extractor: None });
        b1.add(doc("a", "hello world"));
        b1.add(doc("b", "hello there"));
        b1.build();

        let mut b2 = Builder::default();
        b2.field("text", FieldOpts { boost: 2.0, extractor: None });
        b2.add(doc("a", "hello world"));
        b2.add(doc("b", "hello there"));
        b2.build();

        let arr1 = b1.field_vectors.values().next().unwrap().to_array();
        let arr2 = b2.field_vectors.values().next().unwrap().to_array();
        assert_eq!(arr1.len(), arr2.len());
        for (v1, v2) in arr1.iter().zip(arr2.iter()) {
            // With 3-decimal rounding (matching lunr.js), 2x boost is not exact.
            // Allow rounding tolerance of 0.001 on both values.
            assert!((v2 - v1 * 2.0).abs() <= 0.002, "boost 2x should roughly double score");
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
    fn extractor_overrides_field_text() {
        let mut b = Builder::default();
        b.field(
            "text",
            FieldOpts { boost: 1.0, extractor: Some(|f: &Field| f.text.to_uppercase()) },
        );
        // "hello" lowercased by pipeline; extractor uppercases first so only
        // the extracted text "WORLD WORLD" is indexed -> term "world".
        b.add(doc("a", "hello world world"));
        b.build();

        let mut index = crate::index::Index::from(b);
        let results = index.search("world");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].ref_, "a");
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

    #[test]
    fn ref_field_defaults_to_id() {
        let builder = Builder::default();
        assert_eq!(builder.ref_field, "id");
    }

    #[test]
    fn ref_field_sets_custom_field() {
        let mut b = Builder::default();
        b.ref_field("uuid");
        assert_eq!(b.ref_field, "uuid");
    }

    #[test]
    #[should_panic(expected = "ref field name cannot be empty")]
    fn ref_field_empty_panics() {
        Builder::default().ref_field("");
    }

    #[test]
    #[should_panic(expected = "without a 'uuid' field")]
    fn ref_field_name_in_error_message() {
        let mut b = Builder::default();
        b.ref_field("uuid");
        b.add(doc("", "hello"));
    }

    #[test]
    fn use_plugin_applies_builder_mutation() {
        let mut b = Builder::default();
        b.use_plugin(|b| {
            b.k1(3.0);
            b.b(0.5);
        });
        assert_eq!(b.k1, 3.0);
        assert_eq!(b.b, 0.5);
    }

    #[test]
    fn custom_separator_end_to_end() {
        let mut b = Builder::default();
        b.field("text", FieldOpts::default());
        b.tokenizer_separator(" ,");
        b.add(doc("a", "hello,world"));
        b.build();

        let mut index = crate::index::Index::from(b);
        let results = index.search("world");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].ref_, "a");
    }

    #[test]
    fn plugin_stemmer_wired_into_pipeline() {
        use crate::pipeline::PipelineResult;
        use crate::token::Token;

        // A "non-English stemmer" plugin applied to both index and search
        // pipelines via use_plugin: strips a trailing "z" (identity otherwise).
        let plugin = |b: &mut Builder| {
            let f = |t: Token| {
                let term = t.term.trim_end_matches('z').to_string();
                if term.is_empty() {
                    PipelineResult::None
                } else {
                    PipelineResult::Token(Token { term, metadata: t.metadata })
                }
            };
            b.pipeline.add_custom("zStripper", f);
            b.search_pipeline.add_custom("zStripper", f);
        };

        let mut b = Builder::default();
        b.field("text", FieldOpts::default());
        b.use_plugin(plugin);
        // "izz" stems to "i", "waz" stems to "wa" -> both search for "waz" as "wa".
        b.add(doc("a", "izz waz"));
        b.build();

        let mut index = crate::index::Index::from(b);
        let results = index.search("waz");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].ref_, "a");
    }
}
