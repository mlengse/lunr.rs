//! Shared corpus for lunr.rs benchmarks, mirroring lunr.js `perf/`.
//!
//! The word list is the first 1000 words of the npm `word-list` package,
//! sliced and sorted exactly as lunr.js `perf/perf_helper.js` does
//! (`split('\n').slice(0, 1000).sort()`), committed so both sides read
//! identical input.

// Each bench target is its own crate and `mod common;` compiles the whole file,
// so only the items that bench uses are reached. Silence the per-bench noise.
#![allow(dead_code)]

use lunr::document::{Document, Field};

/// The 1000-word corpus (sorted), identical to lunr.js `perf_helper.words`.
pub fn words() -> Vec<String> {
    let text = include_str!("data/words.txt");
    text.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// The Cluedo test documents, identical to lunr.js `perf/builder_perf.js` and
/// `perf/search_perf.js`.
pub struct CluedoDoc {
    pub id: &'static str,
    pub title: &'static str,
    pub body: &'static str,
}

pub fn cluedo_docs() -> Vec<CluedoDoc> {
    vec![
        CluedoDoc {
            id: "a",
            title: "Mr. Green kills Colonel Mustard",
            body: "Mr. Green killed Colonel Mustard in the study with the candlestick. Mr. Green is not a very nice fellow.",
        },
        CluedoDoc {
            id: "b",
            title: "Plumb waters plant",
            body: "Professor Plumb has a green plant in his study",
        },
        CluedoDoc {
            id: "c",
            title: "Scarlett helps Professor",
            body: "Miss Scarlett watered Professor Plumbs green plant while he was away from his office last week.",
        },
    ]
}

impl Document for CluedoDoc {
    fn id(&self) -> String {
        self.id.to_string()
    }

    fn fields(&self) -> Vec<Field> {
        vec![
            Field { name: "title".to_string(), text: self.title.to_string(), boost: 1.0 },
            Field { name: "body".to_string(), text: self.body.to_string(), boost: 1.0 },
        ]
    }
}

/// Build an index identical to lunr.js `builder_perf.js` (ref "id", fields
/// title + body, three Cluedo documents).
pub fn cluedo_index() -> lunr::index::Index {
    let docs = cluedo_docs();
    lunr::lunr(move |b| {
        b.ref_field("id");
        b.field("title", Default::default());
        b.field("body", Default::default());
        for d in docs {
            b.add(d);
        }
    })
}
