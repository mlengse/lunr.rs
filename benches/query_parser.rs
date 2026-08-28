use criterion::{criterion_group, criterion_main, Criterion};
use lunr::query::Query;
use lunr::query_parser::QueryParser;

fn parse(q: &str) {
    let query = Query::new(vec!["title".to_string(), "body".to_string()]);
    let parser = QueryParser::new(q, query);
    let _ = parser.parse();
}

fn query_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_parser");
    group.bench_function("simple", |b| b.iter(|| parse("foo bar")));
    group.bench_function("field", |b| b.iter(|| parse("title:foo bar")));
    group.bench_function("modifier", |b| b.iter(|| parse("foo~2 bar")));
    group.bench_function("complex", |b| b.iter(|| parse("title:foo~2^6 bar")));
    group.finish();
}

criterion_group!(benches, query_parser);
criterion_main!(benches);
