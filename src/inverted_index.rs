use crate::token::{Metadata, Term, Token};
use crate::field_ref::FieldRef;

use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
pub struct InvertedIndex {
    index: BTreeMap<Term, Posting>,
}

impl InvertedIndex {
    pub fn add(&mut self, field_ref: FieldRef, token: Token) {
        let index = self.index.len();

        let posting =
            self.index.entry(token.term.to_owned()).or_insert_with(|| Posting::new(index));

        posting.insert(field_ref, token);
    }

    pub fn posting<'a>(&self, term: &'a Term) -> Option<&Posting> {
        self.index.get(term)
    }

    fn len(&self) -> usize {
        self.index.len()
    }
}

impl Serialize for InvertedIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;

        for pair in &self.index {
            seq.serialize_element(&pair)?;
        }

        seq.end()
    }
}

pub struct Posting {
    pub index: usize,
    field_postings: HashMap<String, FieldPosting>,
}

impl Posting {
    fn new(index: usize) -> Posting {
        Posting {
            index,
            field_postings: HashMap::new(),
        }
    }

    pub fn insert(&mut self, field_ref: FieldRef, token: Token) {
        self.field_postings
            .entry(field_ref.field_name)
            .or_insert_with(FieldPosting::default)
            .insert(field_ref.document_ref, token)
    }

    pub fn len(&self) -> usize {
        self.field_postings.len()
    }

    pub fn document_count(&self) -> usize {
        self.field_postings
            .values()
            .map(|fp| fp.documents.len())
            .sum()
    }
}

impl Serialize for Posting {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut posting = serializer.serialize_map(Some(self.len()))?;

        posting.serialize_entry("_index", &self.index)?;

        for (field_name, field_posting) in &self.field_postings {
            posting.serialize_entry(field_name, field_posting)?;
        }

        posting.end()
    }
}

#[derive(Default)]
struct FieldPosting {
    // HashMap<document_ref, HashMap<metadata_key, Vec<Metadata>>>
    documents: HashMap<String, HashMap<String, Vec<Metadata>>>,
}

impl FieldPosting {
    fn insert(&mut self, document_ref: String, token: Token) {
        let metadata = self.documents.entry(document_ref).or_insert_with(HashMap::new);

        for (key, value) in token.metadata {
            metadata.entry(key).or_insert_with(Vec::new).push(value);
        }
    }
}

impl Serialize for FieldPosting {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut field_posting = serializer.serialize_map(Some(self.documents.len()))?;

        for (document_ref, metadata) in &self.documents {
            field_posting.serialize_entry(document_ref, metadata)?;
        }

        field_posting.end()
    }
}
