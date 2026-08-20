use crate::inverted_index::InvertedIndex;
use crate::field_ref::FieldRef;
use crate::vector::Vector;
use crate::builder::Builder;
use crate::match_data::MatchData;
use crate::set::Set;
use crate::token_set::TokenSet;
use crate::query::{self, Query};
use crate::query_parser::QueryParser;
use crate::pipeline::{Pipeline, PipelineFunction};

use std::collections::HashMap;

use serde::ser::{Serialize, Serializer, SerializeStruct};

pub type FieldVector = (String, Vector);

pub struct SearchResult {
    pub ref_: String,
    pub score: f64,
    pub match_data: MatchData,
}

pub struct Index {
    pub version: String,
    pub inverted_index: InvertedIndex,
    field_vectors: HashMap<String, Vector>,
    pub fields: Vec<String>,
    pub pipeline: Pipeline,
    pub token_set: TokenSet,
}

impl Index {
    pub fn search(&mut self, query_string: &str) -> Vec<SearchResult> {
        let fields = self.fields.clone();
        let query = Query::new(fields);
        let parser = QueryParser::new(query_string, query);
        let query = match parser.parse() {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };
        self.execute_query(query)
    }

    pub fn query(&mut self, query: Query) -> Vec<SearchResult> {
        self.execute_query(query)
    }

    fn execute_query(&mut self, query: Query) -> Vec<SearchResult> {
        let mut matching_fields: HashMap<String, MatchData> = HashMap::new();
        let mut query_vectors: HashMap<String, Vector> = HashMap::new();
        let mut term_field_cache: HashMap<String, bool> = HashMap::new();
        let mut required_matches: HashMap<String, Set> = HashMap::new();
        let mut prohibited_matches: HashMap<String, Set> = HashMap::new();

        for field in &self.fields {
            query_vectors.insert(field.clone(), Vector::new());
        }

        for clause in &query.clauses {
            let terms = if clause.use_pipeline {
                let mut pipeline = self.pipeline.clone();
                pipeline.run_string(&clause.term)
            } else {
                vec![clause.term.clone()]
            };

            let mut clause_matches = Set::empty();

            for term in &terms {
                let ts_clause = crate::token_set::Clause {
                    term: term.clone(),
                    edit_distance: clause.edit_distance,
                    boost: None,
                    presence: None,
                    wildcard: Some(clause.wildcard),
                    use_pipeline: None,
                    fields: None,
                };

                let term_token_set = TokenSet::from_clause(&ts_clause);
                let expanded_terms = self.token_set.intersect(&term_token_set).to_array();

                if expanded_terms.is_empty() && clause.presence == query::PRESENCE_REQUIRED {
                    for field in &clause.fields {
                        required_matches.insert(field.clone(), Set::empty());
                    }
                    break;
                }

                for expanded_term in &expanded_terms {
                    let posting = match self.inverted_index.posting(expanded_term) {
                        Some(p) => p,
                        None => continue,
                    };
                    let term_index = posting.index as u32;

                    for field in &clause.fields {
                        let field_posting = match posting.field_posting(field) {
                            Some(fp) => fp,
                            None => continue,
                        };

                        let matching_doc_refs: Vec<String> = field_posting
                            .document_refs()
                            .map(|s| s.to_string())
                            .collect();

                        let term_field = format!("{}/{}", expanded_term, field);

                        let matching_set = Set::new(matching_doc_refs.clone());

                        if clause.presence == query::PRESENCE_REQUIRED {
                            clause_matches = clause_matches.union(&matching_set);

                            if !required_matches.contains_key(field) {
                                required_matches.insert(field.clone(), Set::complete());
                            }
                        }

                        if clause.presence == query::PRESENCE_PROHIBITED {
                            let entry = prohibited_matches
                                .entry(field.clone())
                                .or_insert_with(Set::empty);
                            *entry = entry.union(&matching_set);
                            continue;
                        }

                        let qv = query_vectors.get_mut(field).unwrap();
                        let boost = clause.boost;
                        qv.upsert(term_index, boost, |a, b| a + b);

                        if term_field_cache.contains_key(&term_field) {
                            continue;
                        }

                        for doc_ref in &matching_doc_refs {
                            let metadata = field_posting.metadata(doc_ref).cloned().unwrap_or_default();
                            let field_ref = FieldRef::new(doc_ref, field).to_string();

                            if let Some(existing) = matching_fields.get_mut(&field_ref) {
                                existing.add(expanded_term, field, &metadata);
                            } else {
                                matching_fields.insert(
                                    field_ref,
                                    MatchData::new(Some(expanded_term), field, &metadata),
                                );
                            }
                        }

                        term_field_cache.insert(term_field, true);
                    }
                }
            }

            if clause.presence == query::PRESENCE_REQUIRED {
                for field in &clause.fields {
                    if let Some(existing) = required_matches.get(field) {
                        required_matches.insert(field.clone(), existing.intersect(&clause_matches));
                    }
                }
            }
        }

        let mut all_required = Set::complete();
        let mut all_prohibited = Set::empty();

        for field in &self.fields {
            if let Some(req) = required_matches.get(field) {
                all_required = all_required.intersect(req);
            }
            if let Some(proh) = prohibited_matches.get(field) {
                all_prohibited = all_prohibited.union(proh);
            }
        }

        let matching_field_refs: Vec<String> = if query.is_negated() {
            self.field_vectors.keys().cloned().collect()
        } else {
            matching_fields.keys().cloned().collect()
        };

        if query.is_negated() {
            for field_ref in &matching_field_refs {
                matching_fields
                    .entry(field_ref.clone())
                    .or_insert_with(MatchData::default);
            }
        }

        let mut results_map: HashMap<String, SearchResult> = HashMap::new();
        let mut results_order: Vec<String> = Vec::new();

        for field_ref_str in &matching_field_refs {
            let field_ref = match FieldRef::from_string(field_ref_str) {
                Some(fr) => fr,
                None => continue,
            };
            let doc_ref = &field_ref.document_ref;

            if !all_required.contains(doc_ref) {
                continue;
            }
            if all_prohibited.contains(doc_ref) {
                continue;
            }

            let field_vector = match self.field_vectors.get(field_ref_str) {
                Some(fv) => fv,
                None => continue,
            };

            let score = if let Some(qv) = query_vectors.get_mut(&field_ref.field_name) {
                qv.similarity(field_vector)
            } else {
                0.0
            };

            if let Some(existing) = results_map.get_mut(doc_ref) {
                existing.score += score;
                if let Some(md) = matching_fields.get(field_ref_str) {
                    existing.match_data.combine(md);
                }
            } else {
                let match_data = matching_fields
                    .get(field_ref_str)
                    .cloned()
                    .unwrap_or_default();
                results_map.insert(
                    doc_ref.clone(),
                    SearchResult {
                        ref_: doc_ref.clone(),
                        score,
                        match_data,
                    },
                );
                results_order.push(doc_ref.clone());
            }
        }

        let mut results: Vec<SearchResult> = results_order
            .into_iter()
            .filter_map(|r| results_map.remove(&r))
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    pub fn load(json: &serde_json::Value) -> Result<Index, String> {
        let version = json["version"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let fields: Vec<String> = json["fields"]
            .as_array()
            .ok_or("fields must be an array")?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let serialized_vectors = json["fieldVectors"]
            .as_array()
            .ok_or("fieldVectors must be an array")?;

        let mut field_vectors = HashMap::new();
        for tuple in serialized_vectors {
            let arr = tuple.as_array().ok_or("fieldVectors tuples must be arrays")?;
            if arr.len() < 2 {
                return Err("fieldVectors tuples must have at least 2 elements".into());
            }
            let ref_ = arr[0].as_str().ok_or("fieldVectors ref must be a string")?.to_string();
            let elements: Vec<f64> = arr[1]
                .as_array()
                .ok_or("fieldVectors elements must be an array")?
                .iter()
                .filter_map(|v| v.as_f64())
                .collect();
            field_vectors.insert(ref_, Vector::from_elements(elements));
        }

        let serialized_index = json["invertedIndex"]
            .as_array()
            .ok_or("invertedIndex must be an array")?;

        let mut inverted_index = InvertedIndex::default();
        let mut token_set_builder = TokenSet::builder();

        for tuple in serialized_index {
            let arr = tuple.as_array().ok_or("invertedIndex tuples must be arrays")?;
            if arr.len() < 2 {
                return Err("invertedIndex tuples must have at least 2 elements".into());
            }
            let term = arr[0].as_str().ok_or("invertedIndex term must be a string")?.to_string();
            token_set_builder.insert(&term);

            let posting_obj = &arr[1];
            if let Some(index_val) = posting_obj["_index"].as_u64() {
                let mut posting = crate::inverted_index::Posting::new_raw(index_val as usize);
                for (field_name, field_value) in posting_obj.as_object().unwrap() {
                    if field_name == "_index" {
                        continue;
                    }
                    if let Some(field_obj) = field_value.as_object() {
                        for (doc_ref, meta_obj) in field_obj {
                            if let Some(meta) = meta_obj.as_object() {
                                for (key, values_json) in meta {
                                    if let Some(arr) = values_json.as_array() {
                                        let values: Vec<crate::token::Metadata> = arr
                                            .iter()
                                            .cloned()
                                            .collect();
                                        posting.add_metadata_raw(
                                            field_name.clone(),
                                            doc_ref.clone(),
                                            key.clone(),
                                            values,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                inverted_index.add_posting_raw(term, posting);
            }
        }

        token_set_builder.finish();
        let token_set = token_set_builder.root;

        let pipeline_labels: Vec<String> = json["pipeline"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let mut pipeline = Pipeline::default();
        for label in &pipeline_labels {
            match label.as_str() {
                "trimmer" => pipeline.add_function(PipelineFunction::Trimmer),
                "stopWordFilter" => pipeline.add_function(PipelineFunction::StopWordFilter),
                "stemmer" => pipeline.add_function(PipelineFunction::Stemmer),
                _ => {}
            }
        }

        Ok(Index {
            version,
            inverted_index,
            field_vectors,
            fields,
            pipeline,
            token_set,
        })
    }
}

impl From<Builder> for Index {
    fn from(mut builder: Builder) -> Index {
        builder.build();

        let mut token_set_builder = TokenSet::builder();
        for term in builder.inverted_index.terms() {
            token_set_builder.insert(term);
        }
        token_set_builder.finish();
        let token_set = token_set_builder.root;

        let field_vectors: HashMap<String, Vector> = builder
            .field_vectors
            .into_iter()
            .map(|(fr, v)| (fr.to_string(), v))
            .collect();

        Index {
            version: String::from("2.3.9"),
            pipeline: builder.pipeline,
            inverted_index: builder.inverted_index,
            field_vectors,
            fields: builder.fields,
            token_set,
        }
    }
}

impl Serialize for Index {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut index = serializer.serialize_struct("Index", 5)?;

        index.serialize_field("version", &self.version)?;
        index.serialize_field("pipeline", &self.pipeline.to_json())?;
        index.serialize_field("fields", &self.fields)?;

        let fv: Vec<(&str, &Vector)> = self
            .field_vectors
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect();
        index.serialize_field("fieldVectors", &fv)?;
        index.serialize_field("invertedIndex", &self.inverted_index)?;

        index.end()
    }
}
