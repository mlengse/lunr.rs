use criterion::{criterion_group, criterion_main, Criterion};
use lunr::stemmer::stemmer as apply_stemmer;
use lunr::token::Token;

mod common;

fn stemmer_bench(c: &mut Criterion) {
    let tokens: Vec<Token> = common::words().into_iter().map(Token::new).collect();
    c.bench_function("stemmer", |b| {
        b.iter(|| {
            for t in &tokens {
                std::hint::black_box(apply_stemmer(t.clone()));
            }
        });
    });
}

criterion_group!(benches, stemmer_bench);
criterion_main!(benches);
