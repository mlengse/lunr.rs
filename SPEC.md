# SPEC — lunr.rs

| | |
|---|---|
| **Crate** | `lunr` (library) |
| **Target** | lunr.js v2.3.9 compatible index output |
| **Rust edition** | 2021 |
| **License** | MIT |
| **Status** | Proof of concept — Phases 1-7 complete, 69 tests |

This document is the **source of truth** for lunr.rs behavior. All implementation
decisions must conform to this spec. Deviations require spec updates first.

---

## 1. Goals

lunr.rs is a Rust port of [lunr.js](https://lunrjs.com/) — a client-side
full-text search engine. The primary objective is **wire-format compatibility**:
an index built by lunr.rs must be loadable and searchable by lunr.js.

1. **Index building** — documents are tokenized, processed through a pipeline,
   and stored in an inverted index with BM25 field vectors.
2. **Serialization** — the index serializes to JSON matching lunr.js v2.3.9 format.
3. **Search** — query language with wildcard, fuzzy, boost, field-scoping, presence.
4. **Pipeline** — configurable text processing chain (trimmer, stop-word filter, stemmer).

---

## 2. Scope

This spec covers:

- Module architecture and public API.
- Scoring model (IDF, BM25, document scoring).
- Serialization format and `load` validation.
- Query language syntax and semantics.
- Pipeline system and token processing.
- Internal data structures (`TokenSet`, `Set`, `Vector`).

Out of scope:

- Non-English stemmers (use plugin architecture for those).
- Server-side / distributed search.
- WASM compilation target (pure Rust library).

---

## 3. Architecture & Module Structure

```
src/
├── lib.rs               # Module declarations + lunr() factory
├── utils.rs             # warn, as_string, clone
├── field_ref.rs         # FieldRef (joiner "/", fromString, serialize, Display)
├── set.rs               # Set (Empty, Complete, union, intersect, contains)
├── idf.rs               # idf(posting, document_count) -> f64
├── token.rs             # Token (term, metadata, update, clone) + Tokens
├── tokenizer.rs         # tokenizer(obj, metadata) -> Vec<Token>
├── pipeline.rs          # Pipeline + PipelineFunction enum (Trimmer, StopWordFilter, Stemmer)
├── vector.rs            # Vector (BTreeMap, insert, upsert, dot, magnitude, similarity)
├── stemmer.rs           # Porter stemmer (English, default)
├── stop_word_filter.rs  # Stop word filter (English, default)
├── trimmer.rs           # Trimmer + wordCharacters constant
├── token_set.rs         # TokenSet DAG (fromString, fromArray, intersect, toArray) + Builder
├── inverted_index.rs    # InvertedIndex + Posting + FieldPosting
├── match_data.rs        # MatchData (add, combine)
├── query.rs             # Query + Clause + ClauseOptions + Presence + wildcard constants
├── query_parse_error.rs # QueryParseError (name, message, start, end)
├── query_lexer.rs       # QueryLexer (state-machine, Option<StateId> termination)
├── query_parser.rs      # QueryParser (recursive-descent, field validation)
├── builder.rs           # Builder + FieldOpts (add, field, build, BM25, b/k1, search_pipeline)
├── index.rs             # Index + SearchResult (search, query, load, search_pipeline)
└── document.rs          # Document trait + Field struct
```

---

## 4. Pipeline & Tokenization

### 4.1 Tokenizer

- Input `None` / `null` → empty `Vec`.
- Input array → each element `to_string().normalize(NFC).to_lowercase()` becomes a `Token`.
- Input string → `normalize(NFC).to_lowercase()`, split on separator `[\s-]+`.
- Empty tokens discarded.
- Position metadata: `{"position": [start, length], "index": token_index}`.
- `tokenizer.separator` is configurable.

### 4.2 Pipeline

- **Index pipeline** (used during `Builder::add`): `trimmer → stopWordFilter → stemmer`.
- **Search pipeline** (used during `Index::query`): `stemmer`.
- Pipeline is a `PipelineFunction` enum (`Trimmer`, `StopWordFilter`, `Stemmer`).
- Enum dispatch for Clone-safe, no trait objects needed.
- Returning `None` skips the token; returning `Vec<Token>` expands (term expansion).
- Registered functions have labels for serialization.

### 4.3 Trimmer

- Trims non-word characters from token edges.
- `wordCharacters` constant: `"A-Za-z\xAA\xBA\xC0-\xD6\xD8-\xF6\xF8-\u02FF\u0370-\u037D\u037F-\u1FFF\u200C-\u200D\u2070-\u218F\u2C00-\u2FEF\u3001-\uD7FF\uF900-\uFDCF\uFDF0-\uFFFD"`.
- Regex built per call: `^[^{wordChars}0-9_]+` and `[^{wordChars}0-9_]+$`.

### 4.4 Stemmer

- Porter stemmer (English), ported from lunr.js `lib/stemmer.js`.
- Registered as `"stemmer"`.

### 4.5 Stop-word Filter

- Static English stop-word list (171 words).
- Registered as `"stopWordFilter"`.
- `generate_stop_word_filter(words)` for custom lists.

---

## 5. Scoring Model

### 5.1 IDF

```rust
fn idf(posting: &Posting, document_count: usize) -> f64 {
    let documents_with_term = posting.document_count();  // sum of doc refs across all fields
    let x = (document_count as f64 - documents_with_term as f64 + 0.5)
          / (documents_with_term as f64 + 0.5);
    (1.0 + x.abs()).ln()
}
```

- `df` = count of distinct document refs across all fields (ignore `_index`).
- Shared between `Builder.create_field_vectors` and `Index.query`.

### 5.2 BM25 Field Score

```rust
score = idf * (tf * (k1 + 1.0))
           / (tf + k1 * (1.0 - b + b * field_length / average_field_length))
```

- `k1` default `1.2` (negative clamped to `0`).
- `b` default `0.75` (clamped to `0..=1`).
- Per-field scoring with `field_term_frequencies` and `field_lengths`.
- Score multiplied by `field_boost` and `doc_boost` (both default `1`).
- Final score rounded to 3 decimal places: `(score * 1000.0).round() / 1000.0`.

### 5.3 Document Score

- One query `Vector` per field (values = boost per term, upserted cumulatively).
- For each matching field: `field_vector.dot(query_vector) / field_vector.magnitude()`.
- Sum across all fields → document score.
- Results sorted descending by score.
- **Note:** This is NOT cosine similarity — asymmetric (only divides by document magnitude).

---

## 6. Internal Data Structures

### 6.1 TokenSet (Minimal DAG)

- Nodes: `{ final: bool, edges: HashMap<char, TokenSet>, id: usize }`.
- `TokenSet::from_array(sorted_words)` — build minimized DAG with suffix sharing.
- `TokenSet::from_string(str)` — single string, `*` creates self-edge (wildcard).
- `TokenSet::from_clause(clause)` — from query clause (fuzzy or exact).
- `TokenSet::from_fuzzy_string(str, edit_distance)` — edit distance DFA.
- `intersect(other)` — no memoization; nodes shared only on same character edge from same output node.
- `to_array()` — extracts words; **throws** if wildcards present.
- Builder: `insert(word)` + `finish()` for suffix sharing.

### 6.2 Set

- `Set::Empty` — contains nothing; `intersect(self)` = self, `union(other)` = other.
- `Set::Complete` — contains everything; `intersect(other)` = other, `union(self)` = self.
- `Set::Elements(HashSet<String>)` — concrete set with `intersect` and `union`.

### 6.3 Vector

- Sparse: `elements: BTreeMap<u32, f64>` (sorted by key).
- `insert(index, val)` — no duplicates (throws `"duplicate index"`).
- `upsert(index, val, fn)` — insert or merge with function.
- `position_for_index(index)` — binary search.
- `dot(other)` — dot product (merge-join on sorted keys).
- `magnitude()` — lazy-cached with `Cell<Option<f64>>` for interior mutability (`&self` on `similarity()`).
- `similarity(other)` — `self.dot(other) / self.magnitude() || 0.0`.

---

## 7. Public API

### 7.1 Factory

```rust
pub fn lunr(config: impl FnOnce(&mut Builder)) -> Index
```

Creates a `Builder`, adds default pipeline (`trimmer → stopWordFilter → stemmer`),
sets search pipeline (`stemmer`), calls `config`, then `build()`.

### 7.2 Builder

```rust
impl Builder {
    pub fn new() -> Builder;
    pub fn ref_field(&mut self, field: &str);           // default "id"
    pub fn field(&mut self, name: &str, opts: FieldOpts); // boost, extractor
    pub fn add<T: Document>(&mut self, doc: T);         // validate + tokenize + pipeline
    pub fn build(&mut self) -> Index;                   // calculateAverageFieldLengths + createFieldVectors + createTokenSet
    pub fn b(&mut self, n: f64);                        // BM25 b param (clamped 0..=1)
    pub fn k1(&mut self, n: f64);                      // BM25 k1 param (clamped >= 0)
    pub fn use_plugin(&mut self, fn: impl Fn(&mut Builder)); // plugin
    pub fn pipeline: Pipeline;                          // index pipeline
    pub fn search_pipeline: Pipeline;                   // search pipeline
    pub fn tokenizer: fn(...) -> Vec<Token>;            // configurable tokenizer
}
```

### 7.3 Index

```rust
impl Index {
    pub fn search(&mut self, query: &str) -> Vec<SearchResult>;          // parse query → query
    pub fn query(&mut self, query: Query) -> Vec<SearchResult>;          // programmatic query
    pub fn to_json(&self) -> String;                                     // serialize to lunr.js format
    pub fn load(json: &serde_json::Value) -> Result<Index, String>;      // deserialize + validate
    pub fields: Vec<String>;
    pub pipeline: Pipeline;
    pub search_pipeline: Pipeline;
}
```

### 7.4 SearchResult

```rust
pub struct SearchResult {
    pub ref_: String,
    pub score: f64,
    pub match_data: MatchData,
}
```

### 7.5 Query

```rust
impl Query {
    pub fn new(all_fields: Vec<String>) -> Query;
    pub fn clause(&mut self, clause: Clause);
    pub fn term(&mut self, term: &str, opts: TermOpts);
    pub fn is_negated(&self) -> bool;
}

pub struct Clause {
    pub fields: Vec<String>,
    pub boost: f64,
    pub edit_distance: Option<u32>,
    pub use_pipeline: bool,
    pub wildcard: u8,
    pub presence: Presence,
}

pub enum Presence { Optional, Required, Prohibited }

pub mod wildcard {
    pub const NONE: u8 = 0;
    pub const LEADING: u8 = 1;
    pub const TRAILING: u8 = 2;
}
```

---

## 8. Query Language

| Syntax | Meaning |
|---|---|
| `foo` | term, optional, stemmed |
| `foo bar` | OR (documents with both rank higher) |
| `foo*`, `*oo*`, `f*o` | wildcard (pipeline disabled) |
| `foo~2` | fuzzy, edit distance ≤ 2 |
| `foo^5`, `foo^1.5` | boost (must be positive; decimal supported) |
| `title:foo` | field-scoped |
| `+foo` / `-foo` | presence REQUIRED / PROHIBITED |
| `\~`, `\^`, `\:` | escape special character |

Parser rules:

- Term is lowercased; pipeline-expanded terms use same clause options.
- Unregistered field → `QueryParseError("unrecognised field '<f>', possible fields: ...")`.
- Edit distance must be non-negative integer.
- Boost must be positive number.
- REQUIRED presence with no matches → empty result (early break).

---

## 9. Serialization Format

### 9.1 `toJSON` Output

```json
{
  "version": "2.3.9",
  "fields": ["title", "body"],
  "fieldVectors": [
    ["title/docRef", [0, 1.234, 2, 0.567]],
    ["body/docRef", [0, 0.891, 3, 1.456]]
  ],
  "invertedIndex": [
    ["term", {
      "_index": 0,
      "title": { "docRef": { "position": [[0, 5]] } },
      "body": { "docRef": { "position": [[12, 5]] } }
    }]
  ],
  "pipeline": ["trimmer", "stopWordFilter", "stemmer"],
  "searchPipeline": ["stemmer"]
}
```

- `version`: always `"2.3.9"`.
- `fields`: ordered `Vec<String>` (insertion order).
- `fieldVectors`: `Vec<(FieldRef, Vector)>` serialized as flat `[index, value, ...]`.
- `invertedIndex`: sorted by term, `Vec<(Term, Posting)>`.
- `pipeline`: labels of registered functions in the index pipeline.
- `searchPipeline`: labels of registered functions in the search pipeline.

### 9.2 `Index::load` Validation

- `fieldVectors`, `invertedIndex`, `fields`, `pipeline` must be arrays.
- `searchPipeline` is optional (defaults to empty).
- Each `fieldVectors` entry must be `[string, array]`.
- Each `invertedIndex` entry must be `[string, object]`.
- Version mismatch → warning (not error).
- Malformed shape → `Error("malformed serialized index, ...")`.

---

## 10. Error Handling

| Condition | Error |
|---|---|
| `Builder::add(None/null)` | `"cannot add a undefined or null document to the index"` |
| document missing ref field | `"cannot add a document without a '<ref>' field to the index"` |
| duplicate document ref | `"cannot add a document with a duplicate ref '<ref>'"` |
| `build()` with no documents | `"cannot build index with no documents"` |
| `FieldRef::from_string` no joiner | `Error("malformed field ref string")` |
| `Vector::insert` duplicate | `Error("duplicate index")` |
| `TokenSet::to_array` with wildcard | `Error("cannot convert a TokenSet containing wildcards to an array")` |
| negative edit distance | `QueryParseError("edit distance must be a non-negative integer")` |
| non-positive boost | `QueryParseError("boost must be a positive number")` |
| unregistered field in query | `QueryParseError("unrecognised field '<f>', possible fields: ...")` |
| malformed serialized index | `Error("malformed serialized index, ...")` |

---

## 11. Build & Test

```bash
cargo check          # fast compile check
cargo test           # all tests
cargo run --example simple   # print lunr-compatible JSON index
```

### 11.1 Test Strategy

- Unit tests per module (match lunr.js test coverage).
- Integration test: build index from known documents, serialize, compare against
  lunr.js reference JSON snapshot.
- Search test: query index, verify results match lunr.js output.
- Roundtrip test: `Index::load(serialized)` → search → verify.

### 11.2 Test Parity Target

lunr.js has 540 tests across: `builder`, `field_ref`, `idf`, `index`, `lunr`,
`match_data`, `pipeline`, `query`, `query_lexer`, `query_parser`,
`query_parse_error`, `search`, `serialization`, `set`, `stemmer`,
`stop_word_filter`, `token`, `tokenizer`, `token_set`, `trimmer`, `utils`, `vector`.

---

## 12. Design Constraints

- **Minimal dependencies**: `serde`, `serde_json` only. `erased-serde` listed but unused.
- **No `unsafe`** — pure safe Rust.
- **Wire compatibility** — output must be loadable by lunr.js v2.3.9.
- **Error types** — use `thiserror` or manual `Display`/`Error` impls.
- **Pipeline** — `PipelineFunction` enum (Trimmer, StopWordFilter, Stemmer), Clone-safe via enum dispatch.
- **NFC normalization** — in tokenizer, not pipeline.
- **`TokenSet::intersect`** — no memoization (soundness requirement, see lunr.js audit).
- **`Vector::magnitude`** — `Cell<Option<f64>>` cache, interior mutability for `&self` on `similarity()`.
- **All maps iterated** — use `BTreeMap` or `HashMap` (no `Object.create(null)` equivalent needed; Rust types are safe).

---

## 13. Implementation Status

| Module | Status | Notes |
|---|---|---|
| `field_ref` | **OK** | `fromString`, `JOINER` const, `Display` impl, `to_ref_string()` |
| `token` | **OK** | `update()`, `clone()`, `clone_with()`, `Display`. `Metadata` = `serde_json::Value` |
| `tokenizer` | Partial | Split on whitespace+hyphens with position metadata. No NFC (deferred), no separator config |
| `vector` | **OK** | `BTreeMap` sparse vector with `insert`, `upsert`, `dot`, `magnitude` (Cell-cached), `similarity` (asymmetric). `from_elements` for deserialization |
| `inverted_index` | **OK** | Correct shape, `document_count()`, `terms()`, `field_posting()`, `add_posting_raw()` for load. `FieldPosting` public |
| `builder` | **OK** | IDF + BM25 correct, shared idf module, validation added, pipeline wired. `field()` with `FieldOpts`, `b()`/`k1()` setters, `search_pipeline` |
| `index` | **OK** | Serialization + `search()`, `query()`, `load()`, TokenSet integration, `search_pipeline`. Builds token_set from terms |
| `lunr` (factory) | **OK** | `lunr()` convenience API with default pipeline (trimmer → stopWordFilter → stemmer) and searchPipeline (stemmer) |
| `document` | OK | Lifetime `'a` removed |
| `set` | **OK** | `Empty`, `Complete`, `Elements` variants; `union()`, `intersect()`, `contains()` |
| `idf` | **OK** | Standalone `idf(posting, document_count)`, shared by builder & index |
| `pipeline` | **OK** | `Pipeline` with `PipelineFunction` enum (`Trimmer`, `StopWordFilter`, `Stemmer`). `run()`, `run_string()`, `to_json()`. Clone-safe via enum dispatch |
| `stemmer` | **OK** | Porter stemmer ported from lunr.js, 5-step algorithm |
| `stop_word_filter` | **OK** | English stop words, `generate_stop_word_filter()` factory |
| `trimmer` | **OK** | Non-word char trimming with Unicode ranges |
| `token_set` | **OK** | `TokenSet` with `from_string`, `from_array`, `from_clause`, `from_fuzzy_string`, `intersect`, `to_array` |
| `token_set_builder` | **OK** | `Builder` with `insert`, `finish`, minimization via suffix sharing |
| `match_data` | **OK** | `MatchData` with `add()`, `combine()` |
| `query` | **OK** | `Query`, `Clause`, `ClauseOptions`, presence/wildcard constants, `term()`, `clause()`, `is_negated()` |
| `query_lexer` | **OK** | State-machine lexer with `FieldType`, `TERM`, `EDIT_DISTANCE`, `BOOST`, `PRESENCE` tokens. Whitespace + hyphen separators |
| `query_parser` | **OK** | Recursive-descent parser: `parseClause`, `parsePresence`, `parseField`, `parseTerm`, `parseEditDistance`, `parseBoost`. Field validation, wildcard disables pipeline |
| `query_parse_error` | **OK** | `QueryParseError` with `name`, `message`, `start`, `end`. `Display` and `Error` impls |
| `utils` | **OK** | `warn()`, `as_string()` |
