use serde_json::Value;

use lunr::lunr;
use lunr::builder::FieldOpts;
use lunr::document::{Document, Field};

struct TestDoc {
    id: String,
    title: String,
    body: String,
}

impl Document for TestDoc {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn fields(&self) -> Vec<Field> {
        vec![
            Field {
                name: String::from("title"),
                text: self.title.clone(),
                boost: 1.0,
            },
            Field {
                name: String::from("body"),
                text: self.body.clone(),
                boost: 1.0,
            },
        ]
    }
}

fn make_doc(id: &str, title: &str, body: &str) -> TestDoc {
    TestDoc {
        id: id.into(),
        title: title.into(),
        body: body.into(),
    }
}

const REF_INDEX: &str = include_str!("fixtures/ref-index.json");
const REF_SEARCH: &str = include_str!("fixtures/ref-search.json");

#[test]
fn integration_build_serialize() {
    let index = lunr(|builder| {
        builder.field("title", FieldOpts::default());
        builder.field("body", FieldOpts::default());
        builder.add(make_doc(
            "a",
            "Mr. Green kills Colonel Mustard",
            "Mr. Green killed Colonel Mustard in the study with the candlestick. Mr. Green is not a very nice fellow.",
        ));
        builder.add(make_doc(
            "b",
            "Plumb waters plant",
            "Professor Plumb has a green plant in his study",
        ));
        builder.add(make_doc(
            "c",
            "Scarlett helps Professor",
            "Miss Scarlett watered Professor Plumbs green plant while he was away from his office last week.",
        ));
    });

    let json = serde_json::to_value(&index).expect("json serialization failed");

    // Remove searchPipeline key (lunr.js v2.3.9 toJSON doesnt have it)
    let mut json_val = json;
    if let Value::Object(map) = &mut json_val {
        map.remove("searchPipeline");
    }

    let ref_json = serde_json::from_str::<Value>(REF_INDEX).expect("ref-index JSON invalid");

    // Compare top-level structural elements
    let version_str = json_val["version"].as_str().unwrap_or("");
    assert_eq!(version_str, "2.3.9", "Expected version 2.3.9, got {}", version_str);

    let fields_eq = json_val["fields"] == ref_json["fields"];
    assert!(fields_eq, "Fields mismatch: {:?} vs {:?}", json_val["fields"], ref_json["fields"]);

    // pipeline: check both contain stemmer
    let rust_pipeline = json_val["pipeline"].as_array().unwrap();
    let ref_pipeline = ref_json["pipeline"].as_array().unwrap();
    let has_stemmer_rust = rust_pipeline.iter().any(|v| v.as_str() == Some("stemmer"));
    let has_stemmer_ref = ref_pipeline.iter().any(|v| v.as_str() == Some("stemmer"));
    assert!(has_stemmer_rust, "Rust pipeline should contain stemmer");
    assert!(has_stemmer_ref, "Reference pipeline should contain stemmer");

    // fieldVectors: should have 1 entry per (field, doc) = 6 entries
    let fv_len = json_val["fieldVectors"].as_array().unwrap().len();
    assert_eq!(fv_len, 6, "Expected 6 fieldVectors entries, got {}", fv_len);

    // invertedIndex: should have entries
    let ii_len = json_val["invertedIndex"].as_array().unwrap().len();
    assert!(ii_len > 0, "invertedIndex should have entries");

    // Each invertedIndex entry should have _index
    for entry in json_val["invertedIndex"].as_array().unwrap() {
        let arr = entry.as_array().unwrap();
        let posting = &arr[1];
        assert!(posting.is_object(), "invertedIndex posting should be object");
        let posting_obj = posting.as_object().unwrap();
        assert!(posting_obj.contains_key("_index"), "posting should have _index key");
    }
}

#[test]
fn integration_search() {
    let fixtures = serde_json::from_str::<Value>(REF_SEARCH).expect("ref-search JSON invalid");
    let fixtures_arr = fixtures["results"].as_array().unwrap();

    let mut index = lunr(|builder| {
        builder.field("title", FieldOpts::default());
        builder.field("body", FieldOpts::default());
        builder.add(make_doc(
            "a",
            "Mr. Green kills Colonel Mustard",
            "Mr. Green killed Colonel Mustard in the study with the candlestick. Mr. Green is not a very nice fellow.",
        ));
        builder.add(make_doc(
            "b",
            "Plumb waters plant",
            "Professor Plumb has a green plant in his study",
        ));
        builder.add(make_doc(
            "c",
            "Scarlett helps Professor",
            "Miss Scarlett watered Professor Plumbs green plant while he was away from his office last week.",
        ));
    });

    for fixture_entry in fixtures_arr.iter() {
        let query = fixture_entry
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let fi_results_val = fixture_entry.get("results").and_then(|v| v.as_array());
        let fixture_results: Vec<&Value> = fi_results_val
            .map(|arr| arr.iter().collect())
            .unwrap_or_default();

        let runtime_results = index.search(&query);

        assert_eq!(
            runtime_results.len(),
            fixture_results.len(),
            "Query results count mismatch"
        );

        for (fi_json, rr) in fixture_results.iter().zip(runtime_results.iter()) {
            let fi_ref = fi_json
                .get("ref")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            assert_eq!(
                rr.ref_, fi_ref,
                "Query result ref mismatch"
            );

            let fi_score = fi_json.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            assert!(rr.score > 0.0, "Query result score should be > 0");

            let score_diff = (fi_score - rr.score).abs();
            assert!(score_diff <= 1.0, "Query result score diff too large");

            let terms_count = rr.match_data.metadata.keys().len();
            assert!(terms_count > 0, "Query result should have terms");
        }
    }
}

#[test]
fn integration_roundtrip() {
    let json_val = serde_json::from_str::<Value>(REF_INDEX).expect("ref-index JSON invalid");
    let mut json_val = json_val;

    // Remove searchPipeline key before loading
    if let Value::Object(map) = &mut json_val {
        map.remove("searchPipeline");
    }

    let mut loaded = lunr::index::Index::load(&json_val).expect("load serialized index failed");

    let fixtures = serde_json::from_str::<Value>(REF_SEARCH).expect("ref-search JSON invalid");
    let fixtures_arr = fixtures["results"].as_array().unwrap();

    for fixture_entry in fixtures_arr.iter() {
        let query = fixture_entry
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let fi_results_val = fixture_entry.get("results").and_then(|v| v.as_array());
        let fixture_results: Vec<&Value> = fi_results_val
            .map(|arr| arr.iter().collect())
            .unwrap_or_default();

        let runtime_results = loaded.search(&query);

        assert_eq!(
            runtime_results.len(),
            fixture_results.len(),
            "Roundtrip query results count mismatch"
        );

        for (fi_json, rr) in fixture_results.iter().zip(runtime_results.iter()) {
            let fi_ref = fi_json
                .get("ref")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            assert_eq!(
                rr.ref_, fi_ref,
                "Roundtrip result ref mismatch"
            );

            assert!(rr.score > 0.0, "Roundtrip result score should be > 0");

            let terms_count = rr.match_data.metadata.keys().len();
            assert!(terms_count > 0, "Roundtrip result should have terms");
        }
    }
}