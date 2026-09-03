# Lunr.rs

A Lunr backend implemented in Rust.

## Status

Proof-of-concept stage. All 21 lunr.js modules have been ported. The index
output is wire-compatible with lunr.js v2.3.9 — can be loaded and searched
in JavaScript.

### What works

- **Index building** — tokenization, BM25 scoring, IDF, pipeline processing.
- **Pipeline** — enum-based (trimmer, stopWordFilter, stemmer), clone-safe.
- **Query language** — lexer, parser, field scoping, wildcards, fuzzy matching,
  boost, presence operators (+/-).
- **Search** — full query execution with required/prohibited/optional presence,
  term expansion via TokenSet intersection, asymmetric similarity scoring.
- **Serialization** — JSON index output compatible with lunr.js v2.3.9, searchable
  when loaded in JavaScript.
- **Load** — deserialize JSON index back into a searchable Index.
- **`SearchResult` Serialize** — search results serializable to JSON.
- **`FieldOpts` extractor** — per-field text override with closure.
- **Doc boost** — per-document `boost()` in the `Document` trait.
- **`Builder::ref_field()`** — configurable document ref field (default "id").
- **`Builder::use_plugin()`** — plugin hook for arbitrary builder mutations.
- **NFC normalization** — tokenizer composes decomposed sequences (via `unicode-normalization`).
- **Configurable tokenizer separator** — `Builder::tokenizer_separator()` sets the
  delimiter character set (default `[\s-]+`).
- **Plugin pipeline functions** — `Pipeline::add_custom()` registers arbitrary
  (e.g. non-English) stemmers/functions into the index or search pipeline.
- **Integration tests** — `tests/parity.rs` compares index JSON, search results, and
  roundtrip load-then-search against lunr.js v2.3.9 reference fixtures.
- **94 tests** (84 unit + 3 integration + 2 stemmer + 5 token_set) across all modules.
- **Perf benchmark** — criterion suites under `benches/` mirroring lunr.js
  `perf/*_perf.js`; side-by-side vs lunr.js via root `bench_lunr.py`. Baseline in
  `perf/baseline-<date>.json`. See `CHANGELOG.md` Phase 9 for highlights.

### What's not done

- Nothing — the full README/SPEC feature set for this proof-of-concept is implemented.

## Example

Build an index and print it to stdout:

```bash
cargo run --example simple
```

Load in JavaScript:

```javascript
let idx = lunr.Index.load(JSON.parse('BIG GLOB OF JSON HERE'))
idx.search('life')
```
