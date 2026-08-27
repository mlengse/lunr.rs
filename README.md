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
- **Integration tests** — `tests/parity.rs` compares index JSON, search results, and
  roundtrip load-then-search against lunr.js v2.3.9 reference fixtures.
- **78 tests** (75 unit + 3 integration) across all modules.

### What's not done

- [ ] NFC normalization (deferred — no `unicode-normalization` dep; tokenizer
      lowercases but does not NFC-normalize)
- [ ] Configurable tokenizer separator (split on whitespace+hyphens is hardcoded)
- [ ] Non-English stemmer plugin architecture

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
