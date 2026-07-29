//! Benchmark suite for apmw.
//!
//! Run with: cargo bench

use criterion::criterion_group;
use criterion::criterion_main;

fn bench_version(c: &mut criterion::Criterion) {
    c.bench_function("version", |b| b.iter(apmw::version));
}

criterion_group!(benches, bench_version);
criterion_main!(benches);
