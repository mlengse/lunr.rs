use criterion::{criterion_group, criterion_main, Criterion};
use lunr::vector::Vector;

// Deterministic pseudo-random (LCG) so the bench is reproducible across runs,
// mirroring the shape of lunr.js `vector_perf.js` (1000 inserts each).
// `upsert` merges on duplicate index (lunr.js's vector is a map; no panic).
fn fill(v: &mut Vector, a: u64, seed: u64) {
    let mut state = seed;
    for i in 0..1000u32 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(a);
        let jitter = (state % 100) as u32;
        let val = ((state >> 32) % 10000) as f64 / 100.0;
        v.upsert(i.wrapping_add(jitter), val, |_, new| new);
    }
}

fn vector(c: &mut Criterion) {
    let mut v1 = Vector::new();
    let mut v2 = Vector::new();
    fill(&mut v1, 1442695040888963407, 101);
    fill(&mut v2, 1442695040888963407, 202);

    let mut group = c.benchmark_group("vector");
    group.bench_function("magnitude", |b| b.iter(|| v1.magnitude()));
    group.bench_function("dot", |b| b.iter(|| v1.dot(&v2)));
    group.bench_function("similarity", |b| b.iter(|| v1.similarity(&v2)));
    group.finish();
}

criterion_group!(benches, vector);
criterion_main!(benches);
