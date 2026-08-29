use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use lunr::pipeline::{Pipeline, PipelineFunction, PipelineResult};
use lunr::token::Token;

// Match lunr.js perf: `new lunr.Token(word)` — empty metadata (no position /
// index HashMap or serde Values), so we measure pipeline-stage cost only, not
// the tokenizer's metadata allocation.
fn build_tokens(count: usize) -> Vec<Token> {
    (0..count).map(|i| Token::new(format!("word{i}"))).collect()
}

fn pipeline(c: &mut Criterion) {
    let few = build_tokens(50);
    let many = build_tokens(1000);

    let mut token_to_token = Pipeline::new();
    token_to_token.add_function(PipelineFunction::Custom {
        label: "t2t".to_string(),
        run: Arc::new(|t: Token| PipelineResult::Token(t)),
    });

    let mut token_to_array = Pipeline::new();
    token_to_array.add_function(PipelineFunction::Custom {
        label: "t2a".to_string(),
        run: Arc::new(|t: Token| PipelineResult::Tokens(vec![t.clone(), t])),
    });

    let mut group = c.benchmark_group("pipeline");
    group.bench_function("few tokens, token -> token", |b| {
        b.iter(|| black_box(token_to_token.run(black_box(few.clone()))));
    });
    group.bench_function("many tokens, token -> token", |b| {
        b.iter(|| black_box(token_to_token.run(black_box(many.clone()))));
    });
    group.bench_function("few tokens, token -> token array", |b| {
        b.iter(|| black_box(token_to_array.run(black_box(few.clone()))));
    });
    group.bench_function("many tokens, token -> token array", |b| {
        b.iter(|| black_box(token_to_array.run(black_box(many.clone()))));
    });
    group.finish();
}

criterion_group!(benches, pipeline);
criterion_main!(benches);
