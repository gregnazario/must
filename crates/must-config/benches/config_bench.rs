use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use must_config::load::load_config;
use std::path::Path;

fn generate_config(num_recipes: usize) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::TempDir::new().unwrap();
    let mut toml = String::from("[project]\nname = \"bench\"\n\n[env]\nOPT = \"2\"\n\n");
    for i in 0..num_recipes {
        let deps = if i > 0 {
            format!("deps = [\"r{}\"]", i - 1)
        } else {
            String::new()
        };
        toml.push_str(&format!(
            "[recipe.r{i}]\ntype = \"shell\"\n{deps}\nscript = \"echo {i}\"\n\n"
        ));
    }
    let path = dir.path().join("Mustfile.toml");
    std::fs::write(&path, &toml).unwrap();
    (dir, path)
}

fn bench_parse_small(c: &mut Criterion) {
    let (_dir, path) = generate_config(5);
    c.bench_function("parse_5_recipes", |b| {
        b.iter(|| load_config(black_box(&path)).unwrap())
    });
}

fn bench_parse_medium(c: &mut Criterion) {
    let (_dir, path) = generate_config(50);
    c.bench_function("parse_50_recipes", |b| {
        b.iter(|| load_config(black_box(&path)).unwrap())
    });
}

fn bench_parse_large(c: &mut Criterion) {
    let (_dir, path) = generate_config(200);
    c.bench_function("parse_200_recipes", |b| {
        b.iter(|| load_config(black_box(&path)).unwrap())
    });
}

fn bench_parse_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_scaling");
    for count in [5, 10, 25, 50, 100, 200] {
        let (_dir, path) = generate_config(count);
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| load_config(black_box(&path)).unwrap())
        });
    }
    group.finish();
}

fn bench_parse_polyglot(c: &mut Criterion) {
    let example = Path::new("../../examples/polyglot/Mustfile.toml");
    if !example.exists() {
        return;
    }
    c.bench_function("parse_polyglot_example", |b| {
        b.iter(|| load_config(black_box(example)).unwrap())
    });
}

criterion_group!(
    benches,
    bench_parse_small,
    bench_parse_medium,
    bench_parse_large,
    bench_parse_scaling,
    bench_parse_polyglot,
);
criterion_main!(benches);
