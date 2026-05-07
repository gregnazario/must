use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use must_graph::dag::Dag;
use std::collections::HashMap;

fn bench_topo_sort_linear(c: &mut Criterion) {
    let mut recipes = HashMap::new();
    for i in 0..100 {
        let deps = if i > 0 {
            vec![format!("r{}", i - 1)]
        } else {
            vec![]
        };
        recipes.insert(format!("r{i}"), deps);
    }
    let dag = Dag::new(recipes);

    c.bench_function("topo_sort_linear_100", |b| {
        b.iter(|| black_box(&dag).topo_sort().unwrap())
    });
}

fn bench_waves_wide(c: &mut Criterion) {
    let mut recipes = HashMap::new();
    for i in 0..200 {
        recipes.insert(format!("r{i}"), vec![]);
    }
    let dag = Dag::new(recipes);

    c.bench_function("waves_200_independent", |b| {
        b.iter(|| black_box(&dag).waves().unwrap())
    });
}

fn bench_waves_diamond(c: &mut Criterion) {
    let mut recipes = HashMap::new();
    recipes.insert("root".to_string(), vec![]);
    for i in 0..50 {
        recipes.insert(format!("mid_{i}"), vec!["root".to_string()]);
    }
    let mut leaf_deps: Vec<String> = (0..50).map(|i| format!("mid_{i}")).collect();
    recipes.insert("leaf".to_string(), leaf_deps.clone());
    leaf_deps.push("root".to_string());
    recipes.insert("leaf2".to_string(), leaf_deps);
    let dag = Dag::new(recipes);

    c.bench_function("waves_diamond_52_nodes", |b| {
        b.iter(|| black_box(&dag).waves().unwrap())
    });
}

fn bench_topo_sort_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("topo_sort_scaling");

    for count in [10, 50, 100, 500, 1000] {
        let mut recipes = HashMap::new();
        for i in 0..count {
            let deps = if i > 0 && i % 3 == 0 {
                vec![format!("r{}", i - 1), format!("r{}", i - 2)]
            } else if i > 0 {
                vec![format!("r{}", i - 1)]
            } else {
                vec![]
            };
            recipes.insert(format!("r{i}"), deps);
        }
        let dag = Dag::new(recipes);

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(&dag).topo_sort().unwrap())
        });
    }
    group.finish();
}

fn bench_reachable_from(c: &mut Criterion) {
    let mut recipes = HashMap::new();
    recipes.insert("root".to_string(), vec![]);
    for i in 0..50 {
        recipes.insert(format!("mid_{i}"), vec!["root".to_string()]);
    }
    for i in 0..20 {
        recipes.insert(format!("leaf_{i}"), vec!["mid_0".to_string(), format!("mid_{}", i % 50)]);
    }
    let dag = Dag::new(recipes);

    c.bench_function("reachable_from_mid0", |b| {
        b.iter(|| black_box(&dag).reachable_from("mid_0"))
    });
}

criterion_group!(
    benches,
    bench_topo_sort_linear,
    bench_waves_wide,
    bench_waves_diamond,
    bench_topo_sort_scaling,
    bench_reachable_from,
);
criterion_main!(benches);
