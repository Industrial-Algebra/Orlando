// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Performance benchmarks for Orlando transducers.
//!
//! Compare transducer performance against:
//! - Pure iterator chaining (baseline)
//! - Manual for loops

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use orlando_transducers::*;

fn benchmark_map_filter_take(c: &mut Criterion) {
    let mut group = c.benchmark_group("map_filter_take");

    for size in [100, 1_000, 10_000, 100_000].iter() {
        let data: Vec<i32> = (1..=*size).collect();

        group.bench_with_input(BenchmarkId::new("transducer", size), size, |b, _| {
            b.iter(|| {
                let pipeline = Map::new(|x: i32| x * 2)
                    .compose(Filter::new(|x: &i32| x % 3 == 0))
                    .compose(Take::new(10));
                black_box(to_vec(&pipeline, data.clone()))
            });
        });

        group.bench_with_input(BenchmarkId::new("iterator", size), size, |b, _| {
            b.iter(|| {
                let result: Vec<i32> = data
                    .iter()
                    .map(|x| x * 2)
                    .filter(|x| x % 3 == 0)
                    .take(10)
                    .collect();
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("manual", size), size, |b, _| {
            b.iter(|| {
                let mut result = Vec::new();
                for &x in &data {
                    let mapped = x * 2;
                    if mapped % 3 == 0 {
                        result.push(mapped);
                        if result.len() >= 10 {
                            break;
                        }
                    }
                }
                black_box(result)
            });
        });
    }

    group.finish();
}

fn benchmark_complex_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_pipeline");
    let data: Vec<i32> = (1..=10_000).collect();

    group.bench_function("transducer_10_ops", |b| {
        b.iter(|| {
            let pipeline = Map::new(|x: i32| x + 1)
                .compose(Filter::new(|x: &i32| *x % 2 == 0))
                .compose(Map::new(|x: i32| x * 3))
                .compose(Filter::new(|x: &i32| *x > 10))
                .compose(Map::new(|x: i32| x / 2))
                .compose(Filter::new(|x: &i32| *x % 5 == 0))
                .compose(Map::new(|x: i32| x - 1))
                .compose(Take::new(50));
            black_box(to_vec(&pipeline, data.clone()))
        });
    });

    group.bench_function("iterator_10_ops", |b| {
        b.iter(|| {
            let result: Vec<i32> = data
                .iter()
                .map(|x| x + 1)
                .filter(|x| x % 2 == 0)
                .map(|x| x * 3)
                .filter(|x| *x > 10)
                .map(|x| x / 2)
                .filter(|x| x % 5 == 0)
                .map(|x| x - 1)
                .take(50)
                .collect();
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_early_termination(c: &mut Criterion) {
    let mut group = c.benchmark_group("early_termination");
    let data: Vec<i32> = (1..=1_000_000).collect();

    group.bench_function("transducer_take_10", |b| {
        b.iter(|| {
            let pipeline = Take::new(10);
            black_box(to_vec(&pipeline, data.clone()))
        });
    });

    group.bench_function("iterator_take_10", |b| {
        b.iter(|| {
            let result: Vec<i32> = data.iter().copied().take(10).collect();
            black_box(result)
        });
    });

    group.bench_function("transducer_take_while", |b| {
        b.iter(|| {
            let pipeline = TakeWhile::new(|x: &i32| *x < 100);
            black_box(to_vec(&pipeline, data.clone()))
        });
    });

    group.bench_function("iterator_take_while", |b| {
        b.iter(|| {
            let result: Vec<i32> = data.iter().copied().take_while(|x| *x < 100).collect();
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_sum(c: &mut Criterion) {
    let mut group = c.benchmark_group("sum");
    let data: Vec<i32> = (1..=100_000).collect();

    group.bench_function("transducer", |b| {
        b.iter(|| {
            let pipeline = Map::new(|x: i32| x * 2);
            black_box(sum(&pipeline, data.clone()))
        });
    });

    group.bench_function("iterator", |b| {
        b.iter(|| {
            let result: i32 = data.iter().map(|x| x * 2).sum();
            black_box(result)
        });
    });

    group.bench_function("manual", |b| {
        b.iter(|| {
            let mut total = 0;
            for &x in &data {
                total += x * 2;
            }
            black_box(total)
        });
    });

    group.finish();
}

fn benchmark_unique(c: &mut Criterion) {
    let mut group = c.benchmark_group("unique");

    // Data with lots of duplicates
    let data: Vec<i32> = (1..=1000).cycle().take(10_000).collect();

    group.bench_function("transducer", |b| {
        b.iter(|| {
            let pipeline = Unique::<i32>::new();
            black_box(to_vec(&pipeline, data.clone()))
        });
    });

    group.bench_function("iterator_dedup", |b| {
        b.iter(|| {
            let mut result = data.clone();
            result.dedup();
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("scan");
    let data: Vec<i32> = (1..=10_000).collect();

    group.bench_function("transducer", |b| {
        b.iter(|| {
            let pipeline = Scan::new(0, |acc: &i32, x: &i32| acc + x);
            black_box(to_vec(&pipeline, data.clone()))
        });
    });

    group.bench_function("iterator_scan", |b| {
        b.iter(|| {
            let result: Vec<i32> = data
                .iter()
                .scan(0, |acc, &x| {
                    *acc += x;
                    Some(*acc)
                })
                .collect();
            black_box(result)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_map_filter_take,
    benchmark_complex_pipeline,
    benchmark_early_termination,
    benchmark_sum,
    benchmark_unique,
    benchmark_scan,
);

criterion_main!(benches);
