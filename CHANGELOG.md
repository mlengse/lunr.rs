# Changelog

All notable changes to lunr.rs are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [Unreleased]

### Phase 2: Core Infrastructure

**Date:** 2026-08-18

Added all foundational modules needed by the rest of the system: IDF, Set,
Pipeline, MatchData, Trimmer, StopWordFilter, Stemmer, and utils.

#### Added

- **`src/utils.rs`** — `warn()` and `as_string()` helper functions.
- **`src/idf.rs`** — standalone IDF calculation shared between builder and index.
  `idf(posting, document_count) -> f64` using `log(1 + |(N - df + 0.5) / (df + 0.5)|)`.
- **`src/set.rs`** — `Set` enum with `Empty`, `Complete`, `Elements` variants.
  `union()`, `intersect()`, `contains()` methods.
- **`src/pipeline.rs`** — `Pipeline` struct with `run()`, `run_string()`, `reset()`,
  `to_json()`, `load()`. `PipelineResult` enum (`Token`, `Tokens`, `None`).
  Functions stored as `Box<dyn FnMut(Token) -> PipelineResult>`.
- **`src/match_data.rs`** — `MatchData` struct with `add()` and `combine()` for
  merging term/field metadata during search.
- **`src/trimmer.rs`** — Porter-compatible trimmer removing non-word characters from
  token edges. Uses `is_word_char()` with Unicode ranges matching lunr.js.
- **`src/stop_word_filter.rs`** — English stop word list (120 words) with
  `generate_stop_word_filter()` factory and `stop_word_filter()` function.
- **`src/stemmer.rs`** — Porter stemmer ported from lunr.js. 5-step algorithm with
  `step2list`/`step3list` lookup tables.

#### Changed

- **`src/token.rs`** — added `pub fn new()`, `pub fn update()`, and `impl Display`.
  `update()` returns `false` (skip token) if closure produces empty string.
- **`src/builder.rs`** — replaced inline `fn idf()` with `use crate::idf`.
  Removed unused `Posting` import.
- **`src/lib.rs`** — registered all new public modules: `idf`, `match_data`,
  `pipeline`, `set`, `stemmer`, `stop_word_filter`, `trimmer`, `utils`.

### Phase 1 — Batch 4: Dead Code Cleanup

**Date:** 2026-08-18

Removed identity maps, unnecessary imports, shortened field initialization, and
dropped unused lifetime from `Document` trait. No behavioral changes.

#### Changed

- **`Document` trait** (`src/document.rs:1`) — removed unused lifetime `'a`.
  `Document<'a>` → `Document`. Breaking change: all implementations and callers
  must drop the lifetime parameter.
- **`Builder::add()`** (`src/builder.rs:44`) — signature simplified from
  `add<'a, T: Document<'a>>` to `add<T: Document>`.
- **`InvertedIndex` serialization** (`src/inverted_index.rs:37-42`) — removed
  identity `.map(|pair| pair)`, iterate directly.
- **`Posting::new()`** (`src/inverted_index.rs:59`) — `index: index` → `index`
  (field init shorthand).
- **`Posting` serialization** (`src/inverted_index.rs:91-92`) — removed
  unnecessary double-borrow `&field_name` → `field_name`.
- **`FieldPosting` serialization** (`src/inverted_index.rs:121-122`) — same
  double-borrow cleanup.
- **`Index` conversion** (`src/index.rs:28-30`) — removed identity
  `.map(|(k, v)| (k, v))`.
- **`Index` imports** (`src/index.rs:8`) — removed `use std::convert::From`
  (in prelude since edition 2018).
- **Example** (`examples/simple.rs:1`) — removed `extern crate` lines, dropped
  lifetime from `impl Document for Quote`.

### Phase 1 — Batch 3: Validation & FieldRef

**Date:** 2026-08-18

Added document validation, build guard, and FieldRef string parsing. Index
building now panics on invalid input instead of silently producing bad output.

#### Added

- **`FieldRef::from_string()`** (`src/field_ref.rs:17-23`) — parses `"field/doc"`
  string format. Returns `None` if no `/` separator found. `JOINER` constant
  extracted for serialization consistency.
- **Document validation** (`src/builder.rs:73-78`) — `Builder::add()` now asserts:
  - Document ref is not empty.
  - Document ref is not duplicate (tracks seen refs in `document_refs: HashSet`).
- **Build guard** (`src/builder.rs:89-90`) — `Builder::build()` panics if no
  documents have been added.

#### Changed

- `FieldRef::serialize()` uses `JOINER` constant instead of hardcoded `"/"`.
- `Builder` now tracks `document_refs: HashSet<String>` for duplicate detection.

### Phase 1 — Batch 2: IDF & BM25 Scoring

**Date:** 2026-08-18

Fixed the IDF formula and added BM25 scoring. Field vectors now use proper
BM25 weighting instead of simple tf*idf. Scores vary by term rarity and
document frequency as expected.

#### Fixed

- **IDF formula** (`src/builder.rs:71-75`) — `(1 + N/(1+df)).ln()` (integer
  division, wrong formula) → `(1.0 + |(N - df + 0.5) / (df + 0.5)|).ln()`.
  Now uses float arithmetic and matches SPEC §5.1.
- **BM25 scoring** (`src/builder.rs:60-66`) — simple `tf * idf` → full BM25:
  `idf * (tf * (k1+1)) / (tf + k1*(1 - b + b*fieldLen/avgLen))`.
  `k1=1.2`, `b=0.75` defaults. Score multiplied by field boost (default 1.0).

#### Added

- `Posting::document_count()` (`src/inverted_index.rs:71-77`) — counts distinct
  document refs across all fields. Required for correct IDF calculation.
- `Builder::calculate_average_field_length()` (`src/builder.rs:80-85`) — computes
  mean field length across all fields for BM25 normalization.
- `Builder.k1` and `Builder.b` fields — BM25 parameters with defaults.

### Phase 1 — Batch 1: Foundation Fixes

**Date:** 2026-08-18

Fixed 6 foundational issues required before edition 2021 migration and correct
serialization output. All existing tests pass; example output now matches lunr.js
v2.3.9 format shape.

#### Fixed

- **Bare trait object** (`src/token.rs:42`) — `Box<Serialize>` → `Box<dyn Serialize>`.
  Required for edition 2021; was a warning in 2015, a hard error in 2021+.
- **Version string** (`src/index.rs:26`) — `"2.1.3"` → `"2.3.9"`.
  Output now matches lunr.js v2.3.9 version field.
- **serialize_struct count** (`src/index.rs:42`) — `serialize_struct("Index", 3)`
  → `serialize_struct("Index", 5)`. Was serializing 5 fields with count 3
  (serde ignores the count, but it was technically incorrect).
- **fields ordering** (`src/builder.rs`, `src/index.rs`) — `HashSet<String>` →
  `Vec<String>` with dedup check. Fields now serialize in insertion order,
  matching lunr.js behavior.
- **Rust edition** (`Cargo.toml`) — Added `edition = "2021"`.
- **extern crate** (`src/lib.rs`) — Removed `extern crate` lines (not needed
  in edition 2018+).

#### Changed

- All intra-module `use` statements updated to `crate::` prefix across
  `builder.rs`, `index.rs`, `inverted_index.rs` (required by edition 2021
  module system).

---

## [0.1.0] — 2026-08-17

### Initial

First working version. Proof-of-concept index building and JSON serialization.

#### Added

- `Builder` — add documents via `Document` trait, build index.
- `Index` — serialize to JSON via serde.
- `FieldRef` — `"field/doc"` serialization format.
- `InvertedIndex` — `BTreeMap<Term, Posting>` with `_index` field.
- `Token` — term + metadata HashMap.
- `Vector` — sparse `BTreeMap<u32, f64>`, flat `[index, score, ...]` serialization.
- `Document` trait — `id()` + `fields()` for document abstraction.
- Example (`examples/simple.rs`) — builds index from two quotes, prints JSON.
