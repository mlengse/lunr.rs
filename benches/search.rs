use criterion::{criterion_group, criterion_main, Criterion};
use lunr::query::{ClauseOptions, Query, WILDCARD_TRAILING};

mod common;

fn search(c: &mut Criterion) {
    let mut index = common::cluedo_index();

    let mut group = c.benchmark_group("search");
    group.bench_function("single term", |b| b.iter(|| index.search("green")));
    group.bench_function("multi term", |b| b.iter(|| index.search("green plant")));
    group.bench_function("trailing wildcard", |b| b.iter(|| index.search("pl*")));
    group.bench_function("leading wildcard", |b| b.iter(|| index.search("*ant")));
    group.bench_function("contained wildcard", |b| b.iter(|| index.search("p*t")));
    group.bench_function("with field", |b| b.iter(|| index.search("title:plant")));
    group.bench_function("edit distance", |b| b.iter(|| index.search("plint~2")));
    group.bench_function("typeahead", |b| {
        b.iter(|| {
            let mut q = Query::new(vec!["title".to_string(), "body".to_string()]);
            q.term(
                "pl",
                Some(ClauseOptions {
                    boost: Some(100.0),
                    use_pipeline: Some(true),
                    ..Default::default()
                }),
            );
            q.term(
                "pl",
                Some(ClauseOptions {
                    boost: Some(10.0),
                    use_pipeline: Some(false),
                    wildcard: Some(WILDCARD_TRAILING),
                    ..Default::default()
                }),
            );
            q.term(
                "pl",
                Some(ClauseOptions {
                    edit_distance: Some(1),
                    ..Default::default()
                }),
            );
            index.query(q);
        });
    });
    group.bench_function("negated query", |b| b.iter(|| index.search("-plant")));
    group.bench_function("required term", |b| b.iter(|| index.search("green +plant")));
    group.finish();
}

criterion_group!(benches, search);
criterion_main!(benches);
