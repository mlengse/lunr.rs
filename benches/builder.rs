use criterion::{criterion_group, criterion_main, Criterion};

mod common;

fn build_index() -> lunr::index::Index {
    lunr::lunr(|builder| {
        builder.ref_field("id");
        builder.field("title", Default::default());
        builder.field("body", Default::default());
        for d in common::cluedo_docs() {
            builder.add(d);
        }
    })
}

fn builder(c: &mut Criterion) {
    c.bench_function("build", |b| {
        b.iter(|| {
            std::hint::black_box(build_index());
        });
    });
}

criterion_group!(benches, builder);
criterion_main!(benches);
