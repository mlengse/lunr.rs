use criterion::{criterion_group, criterion_main, Criterion};
use lunr::token_set::TokenSet;

mod common;

const MONTHS: [&str; 12] = [
    "april", "august", "december", "february", "january", "july", "june",
    "march", "may", "november", "october", "september",
];

fn token_set(c: &mut Criterion) {
    let words = common::words();
    let months: Vec<String> = MONTHS.iter().map(|s| s.to_string()).collect();
    let token_set = TokenSet::from_array(&months);
    let no_wildcard = TokenSet::from_string("september");
    let with_wildcard = TokenSet::from_string("*ber");

    let mut group = c.benchmark_group("token_set");
    group.bench_function("fromArray", |b| b.iter(|| TokenSet::from_array(&words)));
    group.bench_function("fromString (no wildcard)", |b| {
        b.iter(|| TokenSet::from_string("javascript"))
    });
    group.bench_function("fromString (with wildcard)", |b| {
        b.iter(|| TokenSet::from_string("java*cript"))
    });
    group.bench_function("fromFuzzyString", |b| {
        b.iter(|| TokenSet::from_fuzzy_string("javascript", 2))
    });
    group.bench_function("toArray", |b| b.iter(|| token_set.to_array()));
    group.bench_function("intersect (no wildcard)", |b| {
        b.iter(|| token_set.intersect(&no_wildcard))
    });
    group.bench_function("intersect (with wildcard)", |b| {
        b.iter(|| token_set.intersect(&with_wildcard))
    });
    group.finish();
}

criterion_group!(benches, token_set);
criterion_main!(benches);
