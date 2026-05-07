use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use must_cache::hash::compute_hash;
use std::collections::BTreeMap;
use std::path::Path;

fn bench_compute_hash_empty(c: &mut Criterion) {
    let env = BTreeMap::new();
    let flags = BTreeMap::new();
    c.bench_function("compute_hash_empty", |b| {
        b.iter(|| {
            compute_hash(
                black_box("build"),
                black_box("shell"),
                black_box(&[]),
                black_box(&env),
                black_box("rustc 1.85.0"),
                black_box(&flags),
            )
        })
    });
}

fn bench_compute_hash_with_files(c: &mut Criterion) {
    let dir = tempfile::TempDir::new().unwrap();
    let files: Vec<_> = (0..20)
        .map(|i| {
            let p = dir.path().join(format!("file_{i}.rs"));
            std::fs::write(&p, format!("fn main() {{ /* content {i} */ }}")).unwrap();
            p
        })
        .collect();
    let paths: Vec<&Path> = files.iter().map(|f| f.as_path()).collect();

    let mut env = BTreeMap::new();
    env.insert("PROFILE".to_string(), "release".to_string());
    env.insert("TARGET".to_string(), "x86_64-unknown-linux-gnu".to_string());
    let mut flags = BTreeMap::new();
    flags.insert("features".to_string(), "full".to_string());

    c.bench_function("compute_hash_20_files", |b| {
        b.iter(|| {
            compute_hash(
                black_box("build"),
                black_box("rust-bin"),
                black_box(&paths),
                black_box(&env),
                black_box("rustc 1.85.0"),
                black_box(&flags),
            )
        })
    });
}

fn bench_compute_hash_scaling(c: &mut Criterion) {
    let dir = tempfile::TempDir::new().unwrap();
    let mut group = c.benchmark_group("compute_hash_scaling");

    for count in [1, 10, 50, 100] {
        let files: Vec<_> = (0..count)
            .map(|i| {
                let p = dir.path().join(format!("scale_{i}.txt"));
                std::fs::write(&p, format!("content {i}")).unwrap();
                p
            })
            .collect();
        let paths: Vec<&Path> = files.iter().map(|f| f.as_path()).collect();
        let env = BTreeMap::new();
        let flags = BTreeMap::new();

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| {
                compute_hash(
                    black_box("build"),
                    black_box("shell"),
                    black_box(&paths),
                    black_box(&env),
                    black_box("rustc 1.85.0"),
                    black_box(&flags),
                )
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_compute_hash_empty,
    bench_compute_hash_with_files,
    bench_compute_hash_scaling,
);
criterion_main!(benches);
