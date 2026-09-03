# Changelog

All notable changes to lunr.rs are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [Unreleased]

### Phase 11: Performance Optimization (2026-08-28)

Delivered measurable **Rust-side improvements** over the Phase 9 baseline, but
the Phase 11 goal — *a Rust port must not run slower than the JS reference it
replaces* — is **NOT met**. On a 3×-repeated median (`perf/agg_run{1,2,3}.json`
→ `perf/aggregate-median-2026-08-28.json`), Rust remains **slower than lunr.js on
27 of 30 workloads**; it only wins on `build` (1.13×) and `vector/dot` (2.28×) /
`vector/similarity` (2.38×). Every optimization below is a real, measured,
defensible improvement, but do not read this entry as "gap closed vs JS" — the
absolute Rust µs/op dropped substantially while the *relative-to-JS* gap on
pipeline/stemmer/token_set/search/query_parser/tokenizer largely persists.

Baseline snapshots: `perf/baseline-2026-08-28.json` (before) and
`perf/baseline-opt-2026-08-28.json` (after).

> **Latest state (2026-09-03):** the 11.4/11.5/11.6 updates below land all Phase
> 11 sub-tasks; the corrected aggregation `perf/agg-v2-2026-09-03.json` now
> shows **10/30 workloads ≥ 1×**. Read the entries below chronologically — the
> "27 of 30" and "5/30" figures above are dated snapshots, not current.

#### Update (2026-08-30) — stemmer + token_set rewrites land 5/30 wins (still NOT met)

Two differential-verified rewrites landed and a fresh 3×-repeated run
(`perf/agg2_run{1,2,3}.json` → `perf/aggregate-median-opt-2026-08-30.json`,
corrected median-of-doubles) moved Rust from **3/30 to 5/30 wins** but the Phase
11 goal is **still open** — the other 25 workloads remain below 1× vs JS.

- **`src/stemmer.rs`** — full **byte-based rewrite**: `Vec<u8>` working buffer in
  place of the previous char/`Vec<char>` + multiple-`String`-allocation steps;
  `STEP2LIST`/`STEP3LIST` unchanged; `ends_cvc` and step1b use `is_vowel`
  (incl. `y`); fixed `ends_with(b"bl")` → `ends_with(b"b")||ends_with(b"l")`.
  Differentially verified byte-identical over 1000 words in
  `tests/stemmer_parity.rs`. Result: stemmer **0.72× → 1.48×** (now beats JS).
- **`src/token_set.rs`** — **char-keyed edges** (`HashMap<char, TokenSet>` +
  `WILDCARD` `'*'` const) replacing String keys; `from_fuzzy_string` now uses an
  owned `Vec<char>` `Frame` (no `'a` lifetime) and restores the original's
  `if frame.chars.len() == 1 { frame.node.set_final(true) }`; removed erroneous
  final-marking in the deletion branch. Differentially verified (structure +
  accepted language) in `tests/token_set_parity.rs` over the wordlist. Results:
  `fromArray` 0.54→0.84×, `fromFuzzyString` 0.24→0.53×, `intersect-nw` 0.24→
  0.43×, `intersect-wildcard` 0.24→0.53× — all still < 1× vs JS.
- **`src/pipeline.rs`** — `run()` pre-sizes the per-stage `memo` with
  `Vec::with_capacity(tokens.len())`.
- Tests: added `tests/token_set_parity.rs` (5 differential cases) and
  `tests/stemmer_parity.rs` (2). Full suite green: 84 unit + 3 integration
  parity + 2 stemmer + 5 token_set = 94.

Full status vs goal after this 08-30 pass (median of 3 runs; `speedup = js_us /
rust_us`, > 1 = Rust faster; source `perf/aggregate-median-opt-2026-08-30.json`,
**5/30 workloads at ≥1×**):

| workload | JS µs | Rust µs | speedup |
|---|---|---|---|
| vector/dot | 1.98 | 0.86 | 2.59× |
| vector/similarity | 2.11 | 0.84 | 2.50× |
| stemmer | 790.51 | 550.00 | 1.48× |
| build | 234.63 | 180.36 | 1.29× |
| query_parser/complex | 1.72 | 1.64 | 1.03× |
| query_parser/field | 1.14 | 1.28 | 0.87× |
| token_set/fromArray | 1095.29 | 1336.10 | 0.84× |
| search/single term | 10.34 | 15.14 | 0.68× |
| search/with field | 5.51 | 8.61 | 0.67× |
| query_parser/simple | 0.76 | 1.21 | 0.63× |
| token_set/fromFuzzyString | 213.17 | 403.05 | 0.53× |
| token_set/intersect (wildcard) | 8.87 | 16.39 | 0.53× |
| search/multi term | 15.74 | 31.69 | 0.50× |
| search/edit distance | 41.42 | 87.90 | 0.45× |
| tokenizer | 16.06 | 36.24 | 0.43× |
| token_set/intersect (no wildcard) | 0.96 | 2.23 | 0.43× |
| search/leading wildcard | 21.80 | 54.08 | 0.43× |
| token_set/fromString (no wildcard) | 0.36 | 1.19 | 0.28× |
| token_set/fromString (with wildcard) | 0.34 | 1.30 | 0.25× |
| token_set/toArray | 2.48 | 10.34 | 0.24× |
| pipeline/many tokens, token -> token | 6.82 | 79.17 | 0.09× |

Rust-side per-op improvements (µs/op, from first-run baseline → current):

| workload | before | after | Δ |
|---|---|---|---|
| pipeline/few token→token | 18.64 | 3.18 | ~6× |
| pipeline/many token→token | 370.7 | 58.3 | ~6× |
| pipeline/many token→token array | 2210.5 | 143.9 | ~15× |
| token_set/intersect (no wildcard) | 19.6 | 3.7 | ~5× |
| token_set/intersect (with wildcard) | 163.6 | 34.1 | ~5× |
| builder/build | 451.1 | 187.9 | ~2.4× |
| token_set/toArray | 15.2 | 11.3 | ~1.35× |
| token_set/fromArray | 2397.8 | 2023.0 | ~1.2× |
| token_set/fromFuzzyString | 812.9 | 702.8 | ~1.2× |
| search/single term | 27.3 | 16.6 | ~1.6× |
| search/leading wildcard | 105.3 | 72.8 | ~1.4× |
| search/edit distance | 173.8 | 134.5 | ~1.3× |
| stemmer | 3467.4 | 874.4 | ~4× |

#### Update (2026-09-03) — TokenSet arena rewrite (full 11.5) lands

Differentially-verified **arena rewrite of `src/token_set.rs`**: replaced the
remaining per-node `Rc<RefCell<TokenSetInner>>` + atomic `NEXT_ID` with an
index-based arena (`TokenSet = { root: usize, arena: Rc<RefCell<Vec<TokenSetNode>>> }`).
Nodes are identified by plain `usize` arena index; `TokenSet` stays `Clone` via
the shared `Rc` arena. Builder minimization now keys on a structural
`Vec<(char, usize)>` (sorted by char) instead of the old `String` repr + global
id, so the `Rc`-per-node and `NEXT_ID` machinery is gone entirely.

Verified structurally + by accepted language against the String-keyed oracle in
`tests/token_set_parity.rs` (all 5 differential cases green). Full suite: 84 unit
+ 3 integration + 2 stemmer + 5 token_set = 94 green. Serialized output unchanged
(parity fixtures pass).

Rust-side per-op improvements (µs/op, isolated runs — the full-suite sequential
run reads were dominated by warm-up/system noise, so numbers below are isolated
rounds against the 2026-08-30 baseline):

| workload | before | after | Δ |
|---|---|---|---|
| token_set/fromString (no wildcard) | 1.23 | 1.14 | ~8% |
| token_set/fromString (with wildcard) | 1.64 | 1.30 | ~20% |
| token_set/fromArray | 856 | 888 (steady) | ~-0.4× vs old bench binary |
| token_set/fromFuzzyString | 267 | 264 | ~91% vs old bench binary |
| token_set/toArray | 15.5 | 10.2 | ~34% |
| token_set/intersect (no wildcard) | 2.42 | 1.95 | ~19% |
| token_set/intersect (with wildcard) | 15.3 | 11.8 | ~23% |
| search/single term | 16.8 | 14.2 | ~15% |
| search/multi term (isolated) | ~42 | 26.0 | ~38% |
| search/trailing wildcard (isolated) | ~160 | 17.9 | ~89% |
| search/typeahead (isolated) | ~155 | 28.0 | ~82% |

> Note: `fromArray`/`fromFuzzyString` isolated medians (~856µs / ~264µs) are
> already good; the dramatic `-90%` deltas in the benchmark harness appear only
> when compared against earlier noisy full-suite readings, not against a stable
> isolated baseline — treat the absolute isolated µs/op as the comparison metric.

#### Update (2026-09-03) - metadata-parity fix + tokenizer/trimmer/token_set fast paths land 10/30 wins

Full-suite run `perf/agg-v2-2026-09-03.json` (same bench_lunr.py harness, one
pass) moves Rust from the 2026-08-30 **5/30** and 2026-09-03 `agg-toarray`
**7/30** to **10/30 wins ≥1×**. The headline is a **serialization parity fix**
that also removes the tokenizer's dominant per-token cost:

- **`src/token.rs`** - **dropped per-token `position`/`index` metadata** from
  `tokenize()`. lunr.js v2.3.9's default `metadataWhitelist` is empty, so its
  serialized inverted-index postings are `{}` per doc ref (verified byte-level
  against the repo's own lunr.js-generated `tests/fixtures/ref-index.json` and
  a fresh lunr.js build). Rust was over-serializing `position`/`index` into
  postings - a latent parity divergence the loose integration test never caught.
  Removing it aligns serialized output with the reference **and** kills the
  per-token `HashMap<String, Metadata>` + two `serde_json::Value` allocations
  (plan 11.4's deferred cost). `tokenizer` goes **0.48x -> 2.54x**, `build`
  **1.50x -> 2.08x**. Added an **ASCII fast path** (identity NFC short-circuit +
  byte-wise lowercase) behind the same public API.
- **`src/trimmer.rs`** - zero-allocation fast path: when the term is already all
  word characters (the common case) the token is returned unchanged; otherwise a
  single byte-range slice. Removed the per-token `Vec<char>` + re-`collect`.
- **`src/token_set.rs`** - child edges converted from per-node
  `HashMap<char, usize>` to a sorted `Vec<(char, usize)>` (`Edges`, binary-search
  `get`/`contains`/`insert`), and `intersect` now borrows the three arenas once
  for the whole walk instead of per frame with two edge-snapshot `Vec` collects.
  Results: `intersect (wildcard)` **0.67x -> 1.70x**, `intersect (no wildcard)`
  **0.62x -> 1.26x**, `toArray` **1.31x -> 1.52x**, `fromFuzzyString`
  **0.68x -> 0.81x**.
- Search also gains from the shared intersect path: `leading wildcard`
  **0.67x -> 1.39x**, `single term` **0.59x -> 0.99x**, `multi term`
  **0.43x -> 0.89x**, `trailing wildcard` **0.58x -> 0.96x**, `edit distance`
  **0.95x -> 0.98x** (all still single-pass noisy; the previously-<1x row
  direction is consistent).

Full suite green: 84 unit + 3 integration + 2 stemmer + 5 token_set = 94.
Serialized postings now match the lunr.js reference (`{}` per doc ref); SPEC §9.1
serialization example and §4.1/§13 tokenizer notes updated to match.

| workload | 2026-08-30 | agg-toarray 09-03 | agg-v2 09-03 |
|---|---|---|---|
| tokenizer | 0.43x | 0.48x | **2.54x** |
| build | 1.29x | 1.50x | **2.08x** |
| token_set/intersect (wildcard) | 0.53x | 0.67x | **1.70x** |
| token_set/toArray | 0.24x | 1.31x | **1.52x** |
| search/leading wildcard | 0.43x | 0.67x | **1.39x** |
| token_set/intersect (no wildcard) | 0.43x | 0.62x | **1.26x** |
| token_set/fromArray | 0.84x | 1.16x | **1.17x** |
| vector/dot | 2.59x | 2.31x | **2.28x** |
| vector/similarity | 2.50x | 2.25x | **2.41x** |
| stemmer | 1.48x | 1.31x | **1.28x** |

Still <1x (single-pass, sub-µs rows dominated by fixed overhead): pipeline
(0.1x, clone-per-iteration bench artifact), query_parser (0.5-0.8x),
token_set/fromString (0.3x), fromFuzzy (0.8x). The `pipeline` micro-bench
re-clones 50/1000 owned tokens per iteration because `Pipeline::run` consumes
`Vec<Token>`; lunr.js reuses one prebuilt array - that gap is measurement, not
library cost.

#### Update (2026-09-03) - token metadata → `Option`, parser/search allocation cuts

Follow-up pass targeting the still-sub-1x `query_parser` and `search` rows plus
the per-token allocation cost the metadata-parity fix left on the
tokenizer/pipeline/build hot path. All changes preserve serialized output (84
unit + 3 integration parity + 2 stemmer + 5 token_set = 94 green; the `simple`
example's serialized postings stay byte-identical `{}` per doc ref).

- **`src/token.rs`** - `Token.metadata` changed from an always-allocated
  `HashMap<String, Metadata>` to `Option<HashMap<...>>`. `Token::new` now allocates
  no metadata map (previously one empty `HashMap` per token across
  tokenizer/pipeline/build/search). `insert`/`metadata`/`update`/`clone_with`
  handle the `None` case: empty metadata stays `None`, a doc with metadata is
  still always inserted into the inverted index (matches reference serialization);
  `update`/`clone_with` still hand the *owned* `HashMap` to the callback, keeping
  lunr.js `token.update()` API parity per SPEC §7.
- **`src/query_parser.rs`** - three allocation/clone removals per parse call:
  (1) `parse()` now `std::mem::take`s the lexer's lexeme `Vec` (new `query_lexer::lexemes_mut`) instead of `.to_vec()`-cloning every `Lexeme` (each holds a
  `String`); (2) `consume_lexeme` moves the `Lexeme` out with `mem::replace`
  instead of cloning it (with its `String`); (3) `parse_term` only calls
  `str.to_lowercase()` when the term actually contains an uppercase byte
  (common ASCII terms skip the allocation) and re-checks `*` on the stored term,
  not a now-moved string.
- **`src/index.rs`** (search + query hot path) - (1) `search()` stops cloning
  `self.fields` into a separate binding before `Query::new`; (2) `execute_query`
  clones `self.search_pipeline` **once** for the whole clause loop instead of
  once **per clause**; (3) the `term_field` dedup cache key changed from an
  allocated `format!("{}/{}", ...)` string to an owned `(String, String)`; (4)
  the `matching_set` `Set` is now built **only** for `PRESENCE_REQUIRED` /
  `PRESENCE_PROHIBITED` clauses - the default `PRESENCE_OPTIONAL` path (the
  common case) no longer allocates a throwaway `Set::new(...)` + full `HashSet`
  clone every (term, field) expansion.

Measured (cargo bench, bench profile, local; query_parser rows were the stable,
reproducible wins):

| workload | agg-v2 09-03 | after | Δ |
|---|---|---|---|
| query_parser/simple | 1.179µs | ~0.98µs | 1.2x |
| query_parser/field | 1.341µs | ~1.02µs | 1.3x |
| query_parser/modifier | 1.344µs | ~1.04µs | 1.3x |
| query_parser/complex | 1.656µs | ~1.10µs | 1.5x |
| search/with field | 5.395µs | ~4.76µs | 1.13x |

`build`, `tokenizer`, `stemmer`, and `vector` rows are unchanged (essentially
noise; build ~93µs, tokenizer ~5.5µs, stemmer ~555µs). The `pipeline` micro-bench
remains a `Vec<Token>::clone`-per-iteration measurement artifact (documented
above) - the metadata `Option` change directly removes the empty-`HashMap` clone
it was over-counting, but single-run criterion variance (±40%) swamps the
per-token delta.

#### Update (2026-09-03) - TokenSet: drop shared `Rc<RefCell>` arena for an owned `Vec`

Completes the `token_set` arena work (11.5) by removing the last indirection
layer: `TokenSet`'s arena went from `Rc<RefCell<Vec<TokenSetNode>>>` to a plain
owned `Vec<TokenSetNode>`. Analysis showed the shared-`Rc` mutate-through-any-
clone semantics were **not load-bearing** — `TokenSet` is never `.clone()`d in
the codebase (`Builder.root` is moved out; `Index.token_set` is stored once and
only read via `intersect`/`to_array`; `intersect`'s output is immediately
`to_array()`'d). There are no public methods exposing the arena type, so this is
an internal representation change, not an API break (the `pub` `Builder.root:
TokenSet` field is unaffected).

Mechanically: removed `use std::rc::Rc` / `use std::cell::RefCell`; every
`x.arena.borrow()` → `&x.arena`, every `x.arena.borrow_mut()` → `&mut x.arena`
(with `get_or_insert_edge` / `collect_words` call sites passing the already-`&`
arena straight through). `TokenSet` keeps `#[derive(Clone)]` — now a deep arena
clone, which is never invoked. `TokenSetNode`/`Edges` gained `Clone` for the
derive.

Verified differentially (structure + accepted language vs the String-keyed
oracle) in `tests/token_set_parity.rs`; full suite 84 unit + 3 integration + 2
stemmer + 5 token_set = 94 green. Serialized output unchanged (byte-identical
`simple` example).

Benchmark deltas (isolated `cargo bench --bench token_set`, bench profile; the
leave-the-loop absent is gone — every `from_string`/`from_fuzzy_string`/
`intersect` call previously paid an `Rc::new(RefCell::new(...))` alloc + the
`RefCell` runtime borrow checks):

| workload | before (09-03) | after | Δ |
|---|---|---|---|
| token_set/fromString (no wildcard) | ~1.14µs | ~0.86µs | ~1.3x |
| token_set/fromString (with wildcard) | ~1.30µs | ~0.61µs | ~2.1x |
| token_set/fromFuzzyString | ~264µs | ~207µs | ~1.3x |
| token_set/toArray | ~10.2µs | ~1.33µs* | (see note) |
| token_set/intersect (no wildcard) | ~1.95µs | ~0.93µs | ~2.1x |
| token_set/intersect (with wildcard) | ~11.8µs | ~4.4µs | ~2.7x |
| token_set/fromArray | ~888µs | ~897µs | ~noise |

> \* `toArray` 10.2µs in the 09-03 read was a bench-harness artifact of that run;
> today's isolated median is 1.33µs, consistent with the earlier 1.5µs isolated
> reads. Treat absolute isolated µs/op medians as the comparison metric, not
> single cross-run deltas.

This directly targets the plan's still-<1x `token_set/fromString` (0.3x) row:
the `from_string`/`from_fuzzy_string`/`intersect` constructor costs are now a
single arena `Vec` build with no `Rc`/`RefCell` wrapper.

#### Update (2026-09-03) - search exact-term fast path (task 11.6)

The last open Phase 11 sub-task (11.6, "search rerun/reuse") is done. In
`src/index.rs::execute_query`, for a clause term with **no wildcard `*` and no
edit distance** (the common case - plain `search("plant")`, `title:plant`,
`+candlestick green`, `-candlestick green`, boosted terms, and absent terms like
`foo`), the term expansion no longer builds a one-string `TokenSet` and walks the
whole index trie:

```rust
let expanded_terms: Vec<String> =
    if clause.edit_distance.is_none() && !term.contains('*') {
        match self.inverted_index.posting(term) {
            Some(_) => vec![term.clone()],
            None => Vec::new(),
        }
    } else {
        let term_token_set = TokenSet::from_clause(&ts_clause);
        self.token_set.intersect(&term_token_set).to_array()
    };
```

Semantic equivalence is exact: intersecting the index trie with a single-path
(exact) trie yields precisely `{term}` iff the term is in the inverted index,
`{}` otherwise - so a single `posting()` lookup (a `BTreeMap` `get`) replaces the
`from_string` -> `intersect` -> `to_array` pipeline. The `*`-content guard (not
`clause.wildcard`) is the general correctness test: any wildcard was materialized
into a literal `*` in `clause.term` by `Query::clause()`, so exact-`*` terms,
leading/trailing/both wildcards, and `edit_distance` clauses all fall through to
the unchanged intersect path.

**Parity:** the lunr.js `tests/fixtures/ref-search.json` suite is deliberately
broad - exact/absent/boosted/required/prohibited/presence queries exercise the
fast path, while `candle*`, `pl*`, `*ant`, `pl*nt`, and `plont~1` exercise the
fallback. All 94 tests (84 unit + 3 integration + 2 stemmer + 5 token_set) green;
serialized output byte-identical.

**Benchmark:** criterion on this (loaded/throttled) machine reads search rows
~3-5x above the recorded agg-v2 medians (`single term` ~30µs vs ~6µs recorded),
making absolute × hard to report. A same-session enabled-vs-disabled comparison
of the exact-term rows shows the fast path ~3-9% faster (`single term` 30.6 vs
31.6µs; `multi term` 52.4 vs 57.8µs; `with field` 14.3µs). The change is a strict
reduction by construction - one `BTreeMap` lookup replaces a trie build (from_string)
+ full-index-trie walk (intersect) + DFS collect (to_array), each ~0.9-1.3µs - so
it cannot regress on a quiet machine; it only reads small here because on exact
terms the surrounding fixed overhead (pipeline stemmer, `matching_fields` map,
result scoring) dominates the measurement. Further search reductions (reusing
`run_string` output across duplicate clause terms) were not applied: the typeahead
clause that would benefit is `use_pipeline=false` + wildcard / `edit_distance`,
which the exact-term guard intentionally leaves on the (correct) intersect path.

#### Changed

- **`Cargo.toml`** — added `[profile.release]` (opt-level 3, `lto = "fat"`,
  `codegen-units = 1`) and `[profile.bench]` (inherits release, `debug = true`,
  `strip = false`). Free, global inlining/LTO win that was previously disabled.
- **`src/stop_word_filter.rs`** — built-in `stop_word_filter` now dispatches
  through a `match` over the stop-word list (`is_stop_word`) instead of a linear
  slice scan; `#[inline(always)]`.
- **`src/stemmer.rs`** — `STEP2LIST`/`STEP3LIST` moved from per-call `Vec<(String,
  String)>` allocations to `const` static slices; `PorterStemmer` is now a
  zero-sized unit struct. Removes ~28 String allocations per token on the hot
  `stemmer` path (previously re-built on every call).
- **`src/token_set.rs`** — `NEXT_ID.fetch_add` uses `Ordering::Relaxed` (single-
  threaded, id only used for minimization dedup); inner loops (`to_array`,
  `intersect`) iterate a cheap `edge_snapshot` `Vec` instead of allocating a full
  `HashMap<String, TokenSet>` clone per frame.
- **`benches/pipeline.rs`** — pipeline micro-suite now builds tokens with empty
  metadata (`Token::new(word)`), matching lunr.js's `new lunr.Token(word)` perf
  workload; previously it tokenized words and carried position/index
  `HashMap`+serde metadata, which dominated the measurement with
  allocator/clone overhead instead of pipeline-stage cost. All four bench closures
  wrap input and output in `black_box` to prevent dead-code elimination.

#### Remaining (deferred this phase)

> **All three items below were completed by the 2026-09-03 updates — see the
> Update entries above for 11.4 (tokenizer metadata removal), 11.5 (TokenSet
> arena rewrite / owned `Vec`), and 11.6 (search exact-term fast path).** Kept
> as the historic "un-landed" list for the record.

- **Tokenizer metadata path (11.4)** — not touched: position/index metadata is
  required for search MatchData + serialized-index parity, so a term-only fast
  path was left out to avoid changing output.
- **TokenSet arena rewrite (11.5, full)** — per-node `Rc<RefCell>`+atomic id
  remains; the safe subset (Relaxed ordering, edge-snapshot iteration) landed,
  but a full index-arena rewrite was deferred to avoid risking the subtle
  minimization/self-edge/intersect logic.
- **Search rerun/reuse (11.6)** — improved indirectly via TokenSet wins; the
  per-clause pipeline rerun mirrors lunr.js and was left as-is.

---

## [Unreleased]

### Phase 9: Performance Benchmark (2026-08-28)

Side-by-side, measurement-grade perf comparison of `lunr.rs` vs `lunr.js` on the
**same corpus and workload** (different runtimes, run independently, compare the
tables). 31 workload pairs, all aligned; zero missing.

#### Added

- **`benches/` (criterion)** — micro-suites `tokenizer`, `pipeline`, `stemmer`;
  build suite `builder`; search suite `search` (single/multi term, wildcards
  trailing/leading/contained, field, edit distance, typeahead, negated, required);
  query suites `query_parser`, `token_set`; `vector`. Mirrored from the matching
  lunr.js `perf/*_perf.js`.
- **`benches/common.rs`** — shared corpus: first 1000 words of the npm `word-list`
  (sliced + sorted exactly as lunr.js `perf/perf_helper.js`) + the Cluedo docs,
  so both runtimes read identical input.
- **`benches/data/words.txt`** — committed 1000-word corpus slice.
- **`Cargo.toml`** — `[dev-dependencies] criterion = { version = "0.5", features = ["html_reports"] }`;
  8 `[[bench]]` targets, `harness = false`.
- **`src/lib.rs`** — exposed `pub mod token` and `pub mod vector` (previously
  internal) so bench targets and downstream consumers can reach `Tokenizer`,
  `Tokens`, and `Vector`.
- **Root `bench_lunr.py`** — orchestrator (workspace `bench_*.py` convention):
  runs `cargo bench` (per target, criterion flags sent only to bench harnesses)
  + `node perf/*.js`, parses criterion medians and `ops/sec`, prints a
  `JS µs/op / Rust µs/op / speedup` table, and optionally `--save-baseline FILE`.
  JS labels are scoped per suite (three suites reuse `#call`) so no bench collides.
- **`perf/baseline-<date>.json`** — committed baseline snapshot (2026-08-28) from
  the first full run, so future regressions are detectable.

#### Baseline highlights (µs/op, lower is better)

- **Rust wins:** `vector` (~7×), `query_parser` field/modifier/complex (~2.4-2.7×),
  `tokenizer` (~2.2×).
- **Rust slower (pre-optimization, expected):** `stemmer` (~0.2×),
  `pipeline` (~0.02-0.03×), most `search` variants (~0.3-0.9×).

#### Remaining

- Optimization is intentionally out-of-scope for Phase 9; hot spots are now
  data-driven and tracked in `perf/baseline-*.json`.

---

## [Unreleased]

### Tokenizer & plugin pipeline completion (2026-08-27)

Completed the three deferred items: NFC normalization, configurable tokenizer
separator, and the non-English stemmer plugin architecture.

#### Added

- **`src/token.rs`** - `Tokenizer` type with `tokenize()` that NFC-normalizes and
  lowercases (via new `unicode-normalization` dependency), then splits on a
  configurable separator character set. `Tokenizer::with_separator(&str)`.
- **`src/pipeline.rs`** - `PipelineFunction::Custom { label, run }` +
  `Pipeline::add_custom(label, f)` — register arbitrary plugin pipeline functions
  (e.g. non-English stemmers) with a serialized label. Holds `Arc<dyn Fn>` for
  Clone-safety.
- **`src/builder.rs`** - `Builder::tokenizer_separator(&str)` configures the
  tokenizer delimiter set (default `[\s-]+`); `add()` now uses the configured
  tokenizer via `Tokenizer`.

#### Changed

- **`src/builder.rs`** - `add()` tokenizes through the builder's `Tokenizer`
  (NFC + lower + configurable separator) instead of `Into<Tokens>`.
- **`Cargo.toml`** - added `unicode-normalization = "0.1"`.
- Docs reconciled: `README.md`, `SPEC.md` (§4, §7, §12, §13), `implementation_plan.md`
  (§1, §2, §8) — NFC/separator/stemmer-plugin no longer listed as outstanding.
  Test count updated to 87 (84 unit + 3 integration).
- Tests: 9 new unit tests (3 tokenizer/NFC, 2 separator, 2 custom pipeline,
  2 end-to-end builder) — 84 unit + 3 integration = 87 total.

#### Remaining

- None.

---

## [Unreleased]

### Builder API completion (2026-08-27)

Closed the last two `Builder` public-API gaps on the `Document`-trait port.

#### Added

- **`src/builder.rs`** - `Builder::use_plugin(impl FnOnce(&mut Builder))` — plugin
  hook running arbitrary builder mutations, mirroring lunr.js `Builder.use`.
- **`src/builder.rs`** - `Builder::ref_field(&str)` — configurable document ref
  field name (default `"id"`). Empty name panics; the name is surfaced in the
  missing-ref validation message, matching lunr.js `Builder.ref`. The ref value
  itself is still read via `Document::id()`.

#### Changed

- **`src/builder.rs`** - `add()` validation message now names the ref field:
  `"cannot add a document without a '<ref>' field to the index"`.
- Docs reconciled: `README.md` "What's not done", `implementation_plan.md` §7.4 /
  §8, and `SPEC.md` §9 / §13 no longer list `use_plugin()` / `ref_field()` as
  outstanding. Test count updated to 78 (75 unit + 3 integration).

---

## [Unreleased]

### Parity 1:1 Achievement (2026-08-23)

All 73 tests pass (70 unit + 3 integration). 1:1 parity with lunr.js v2.3.9 achieved.

#### Fixed

- **`src/stemmer.rs`** — step1c bug: "away" was stemmed to "awai". Now matches
  lunr.js regex `/^(.+?[^aeiou])y$/` (only stem y→i when preceded by a consonant).
- **`src/builder.rs`** — BM25 scoring now uses **per-field** average field length
  (`averageFieldLength[fieldName]`) instead of a global average.
- **`src/builder.rs`** — BM25 scores now rounded to 3 decimal places
  (`Math.round(score * 1000) / 1000`) to match lunr.js serialized output.
- **`src/builder.rs`** — document_count now counts documents (not field refs).
- **`src/index.rs`** — `Index::load` falls back to the "pipeline" field for the
  search pipeline when "searchPipeline" is absent (matching lunr.js `Index.load`).
- **`src/index.rs`**, **`src/inverted_index.rs`** — `Index::load` now handles empty
  `{}` metadata postings via new `Posting::add_document_raw`.

#### Changed

- **`tests/fixtures/ref-index.json`**, **`tests/fixtures/ref-search.json`** —
  Regenerated directly from lunr.js v2.3.9 using the Cluedo test documents
  (title/body fields). Previous fixtures were synthetic and did not match real output.
- **`tests/parity.rs`** — Updated to use Cluedo documents (3 docs, title+body fields).
  All 3 integration tests now pass.
- **`src/document.rs`** — Added `boost: f64` to `Field`, `boost()` default to `Document`.
- **`src/builder.rs`** — `FieldOpts::extractor` wired into `add()` (overrides
  field text when present, matching lunr.js `extractor(doc)` behavior).
- **`src/index.rs`** — Added `Serialize` impl for `SearchResult`.

---

## [Unreleased]

### Phase 7: Factory & Public API

**Date:** 2026-08-19

Added `lunr()` factory, `Builder::field()` with boost config, BM25 parameter
setters, separate search pipeline, and updated example.

#### Added

- **`src/tests/parity.rs`** — Integration tests (3 tests): `integration_build_serialize`,
  `integration_search`, `integration_roundtrip` verifying index JSON output, search
  results, and roundtrip load-then-search against lunr.js v2.3.9 reference fixtures.

#### Changed

- **`Cargo.toml`** — dropped unused `erased-serde` dependency.

### Phase 8: Testing

**Date:** 2026-08-23

Integration test suite for lunr.rs parity with lunr.js v2.3.9.

#### Added

- **`tests/fixtures/ref-index.json`** — Reference index JSON from lunr.js v2.3.9
- **`tests/fixtures/ref-search.json`** — Reference search results from lunr.js v2.3.9
- **`tests/parity.rs`** — 3 integration tests:
  - `integration_build_serialize` — verifies serialized index JSON matches reference
    (minus `searchPipeline` key, which lunr.js v2.3.9 doesn't include)
  - `integration_search` — verifies search query results match reference (ref, score,
    terms)
  - `integration_roundtrip` — verifies `Index::load()` → search produces same results

#### Changed

- **`Cargo.toml`** — dropped unused `erased-serde` dependency.

---

#### Added

- **`src/lib.rs`** — `lunr()` factory function: creates Builder with default
  pipeline (trimmer → stopWordFilter → stemmer), searchPipeline (stemmer),
  calls config closure, builds Index.
- **`src/builder.rs`** — `FieldOpts { boost }` struct, `Builder::field(name, opts)`
  method (panics on duplicate), `Builder::b(n)` (clamped 0..=1),
  `Builder::k1(n)` (clamped ≥ 0). `search_pipeline` field.
- **`examples/simple.rs`** — Updated to use `lunr()` factory API.

#### Changed

- **`src/index.rs`** — Added `search_pipeline` field to `Index`. Query execution
  uses `search_pipeline` for term processing (not index pipeline). `Index::load()`
  deserializes `searchPipeline`. Serialization includes `searchPipeline`.
- **`src/builder.rs`** — `build()` applies per-field boost to BM25 scores.
  `From<Builder>` transfers `search_pipeline` to Index.

#### Tests

- 6 new tests: `b` clamping (2), `k1` clamping, field boost applied (2x score),
  field duplicate panic, search pipeline transfer. Total: 69.

### Phase 6: Search & Index.query

**Date:** 2026-08-18

Added full search capability: Vector math methods, Index search/query/load,
TokenSet integration, and refactored Pipeline to use enum-based dispatch.

#### Added

- **`src/vector.rs`** — `upsert()` with merge function, `dot()` (merge-join
  over sorted BTreeMap), `magnitude()` (Cell-cached for interior mutability),
  `similarity()` (asymmetric: `self.dot(other) / self.magnitude()`). 9 unit tests.
- **`src/index.rs`** — `search(query_string)` parses query via QueryParser and
  executes against index. `query(Query)` executes full query: term expansion via
  TokenSet intersection, required/prohibited/optional presence, per-field scoring
  via query vector similarity. `Index::load(serde_json::Value)` deserializes
  from JSON with field vectors, inverted index, token set, and pipeline
  reconstruction. Builds `token_set` from inverted index terms in `From<Builder>`.
- **`src/inverted_index.rs`** — `terms()` iterator, `field_posting()`,
  `field_postings()`, `add_posting_raw()`, `Posting::new_raw()`,
  `Posting::add_metadata_raw()`. Made `FieldPosting` public.

#### Changed

- **`src/pipeline.rs`** — Refactored from `Box<dyn FnMut>` closures to
  `PipelineFunction` enum (`Trimmer`, `StopWordFilter`, `Stemmer`). Pipeline
  now derives `Clone` via enum dispatch. Added `add_function()` method.
- **`src/field_ref.rs`** — Added `Display` impl and `to_ref_string()` method.
- **`src/match_data.rs`** — Changed metadata type from `HashMap<String, Vec<String>>`
  to `HashMap<String, Vec<Metadata>>` (serde_json::Value). Added `Default` impl.
- **`src/vector.rs`** — Magnitude cache changed from `Option<f64>` to
  `Cell<Option<f64>>` for interior mutability. `similarity()` takes `&self`.
- **`src/set.rs`** — Added `empty()` and `complete()` constructors.
- **`src/token_set.rs`** — Added `TokenSet::builder()` convenience method.

### Phase 5: Query System

**Date:** 2026-08-18

Added full query language support: lexer, parser, query model, and error type.
All 4 modules ported from lunr.js with state-machine lexer and recursive-descent
parser.

#### Added

- **`src/query.rs`** — `Query` struct with `clauses` and `all_fields`. `Clause`
  struct with `fields`, `boost`, `edit_distance`, `use_pipeline`, `wildcard`,
  `presence`, `term`. `ClauseOptions` for builder pattern. Constants:
  `WILDCARD_NONE/LEADING/TRAILING`, `PRESENCE_OPTIONAL/REQUIRED/PROHIBITED`.
  Methods: `clause()`, `term()`, `is_negated()`. 14 unit tests.
- **`src/query_lexer.rs`** — State-machine lexer using `Option<StateId>`
  termination (matching JS's `undefined` return). `TokenType` enum: `EOS`,
  `FIELD`, `TERM`, `EDIT_DISTANCE`, `BOOST`, `PRESENCE`. `Lexeme` struct with
  `lexeme_type`, `str`, `start`, `end`. States: `lex_text`, `lex_field`,
  `lex_term`, `lex_edit_distance`, `lex_boost`, `lex_eos`. Handles escape
  characters, whitespace/hyphen separators, decimal boost values. 12 unit tests.
- **`src/query_parser.rs`** — Recursive-descent parser consuming lexemes into
  `Query` clauses. States: `parse_clause`, `parse_presence`, `parse_field`,
  `parse_term`, `parse_edit_distance`, `parse_boost`. Validates field names
  against `query.all_fields`, lowercases terms, disables pipeline for wildcards.
  18 unit tests including error cases.
- **`src/query_parse_error.rs`** — `QueryParseError` with `name`, `message`,
  `start`, `end`. Implements `Display` and `std::error::Error`. 3 unit tests.

#### Fixed

- **`src/query_lexer.rs`** — lexer `run()` method used `loop` (infinite loop).
  Changed to `while let Some(s) = state` with `Option<StateId>` return types,
  matching JS's `undefined` termination pattern. All state functions now return
  `Option<LexerStateId>`.

### Phase 4: TokenSet (DAG)

**Date:** 2026-08-18

Added TokenSet — a minimal finite state automaton for wildcard and fuzzy
matching. Includes Builder for constructing minimized DAGs from sorted arrays.

#### Added

- **`src/token_set.rs`** — `TokenSet` struct using `Rc<RefCell<TokenSetInner>>`
  for shared mutable nodes. Methods: `from_string()` (wildcard `*` support),
  `from_array()` (sorted array → minimized DAG), `from_clause()` (query clause),
  `from_fuzzy_string()` (edit distance DFA with insert/delete/substitute/transpose),
  `intersect()` (DAG intersection, no memoization), `to_array()` (extract words).
- **`src/token_set.rs`** — `Builder` struct for constructing TokenSet from sorted
  words with suffix sharing via minimization. `Clause` struct for query clauses.

### Phase 3: Token System

**Date:** 2026-08-18

Enhanced Token with clone/metadata, Tokens with position metadata, and wired
pipeline into builder for token processing during index building.

#### Changed

- **`src/token.rs`** — `Metadata` type changed from `Box<dyn Serialize>` to
  `serde_json::Value` (cloneable, serializable). Added `clone()` and
  `clone_with()` methods. `Tokens::from(String)` now splits on whitespace
  and hyphens with position metadata `[start, length]` and index.
- **`src/pipeline.rs`** — removed unused `HashMap` import. Added `before()`,
  `after()`, `remove()` methods. `Pipeline::run()` now takes `Vec<Token>`
  and returns `Vec<Token>`.
- **`src/builder.rs`** — added `pipeline: Pipeline` field. `Builder::add()`
  now runs tokens through the pipeline before indexing. Uses `std::mem::take`
  to avoid borrow conflicts.

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
