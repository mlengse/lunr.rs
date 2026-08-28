use criterion::{criterion_group, criterion_main, Criterion};
use lunr::pipeline::{Pipeline, PipelineResult};
use lunr::token::Token;

fn build_tokens(count: usize) -> Vec<Token> {
    lunr::token::Tokens::from(common_mock_words(count)).into_iter().collect()
}

// A small fixed vocabulary so pipeline benches don't read the corpus file.
fn common_mock_words(count: usize) -> String {
    (0..count).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" ")
}

fn pipeline(c: &mut Criterion) {
    let few = build_tokens(50);
    let many = build_tokens(1000);

    let mut token_to_token = Pipeline::new();
    token_to_token.add_custom("t2t", |t: Token| PipelineResult::Token(t));

    let mut token_to_array = Pipeline::new();
    token_to_array.add_custom("t2a", |t: Token| {
        PipelineResult::Tokens(vec![t.clone(), t])
    });

    let mut group = c.benchmark_group("pipeline");
    group.bench_function("few tokens, token -> token", |b| {
        b.iter(|| token_to_token.run(few.clone()));
    });
    group.bench_function("many tokens, token -> token", |b| {
        b.iter(|| token_to_token.run(many.clone()));
    });
    group.bench_function("few tokens, token -> token array", |b| {
        b.iter(|| token_to_array.run(few.clone()));
    });
    group.bench_function("many tokens, token -> token array", |b| {
        b.iter(|| token_to_array.run(many.clone()));
    });
    group.finish();
}

criterion_group!(benches, pipeline);
criterion_main!(benches);
