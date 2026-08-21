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
- **Serialization** — JSON output matching lunr.js v2.3.9 format.
- **Load** — deserialize JSON index back into a searchable Index.
- **TokenSet** — DAG for wildcard and fuzzy matching with minimization.
- **69 unit tests** across all modules.

### What's not done

- [ ] `Builder::use_plugin()` plugin system
- [ ] `Builder::ref_field()` for custom document ref field
- [ ] Integration tests (build → serialize → load → search → compare with lunr.js)
- [ ] `SearchResult` Serialize impl for JSON output
- [ ] NFC normalization (deferred, no `unicode-normalization` dep)
- [ ] Configurable tokenizer separator
- [ ] `FieldOpts` extractor for nested documents
- [ ] Doc boost in Document trait
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
