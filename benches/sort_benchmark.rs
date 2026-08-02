//! Benchmarks comparing the pre-fix comparator-based sort against the
//! current `sort_by_cached_key` + `natsort_keygen` implementation.
//!
//! # Status
//! This benchmark has **not been run** — the environment that generated it
//! has no Rust toolchain and no network access to fetch `criterion`. Run it
//! yourself with:
//!
//! ```text
//! cargo bench
//! ```
//!
//! and paste real results into `BENCHMARK.md`. Do not trust any numbers
//! attributed to this file until they've actually been measured.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use natsort_core::{compare, natsort_keygen, Ns};

/// Deterministic pseudo-random-ish natural-sort-flavored test data: mixes
/// text prefixes with numeric suffixes of varying width, in a shuffled
/// (non-sorted) starting order, so the benchmark isn't measuring a
/// best-case already-sorted input.
fn make_input(n: usize) -> Vec<String> {
    let prefixes = ["file", "img", "track", "chapter", "item", "part"];
    let mut items: Vec<String> = (0..n)
        .map(|i| {
            let prefix = prefixes[i % prefixes.len()];
            // Interleave to avoid a monotonic sequence.
            let number = (i * 2654435761u64 as usize) % (n * 7 + 13);
            format!("{prefix}{number}")
        })
        .collect();
    // Simple deterministic shuffle so we're not benchmarking a sorted or
    // reverse-sorted input, without pulling in a `rand` dependency.
    let len = items.len();
    for i in 0..len {
        let j = (i * 2654435761usize + 12345) % len;
        items.swap(i, j);
    }
    items
}

/// The old approach: a comparator that recomputes both operands' keys on
/// every call. Kept here only as a benchmark baseline — do not reintroduce
/// this pattern in the library.
fn sort_with_comparator(items: &[String], ns: Ns) -> Vec<String> {
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| compare(a, b, ns));
    sorted
}

/// The current approach: compute each key exactly once via
/// `sort_by_cached_key`.
fn sort_with_cached_key(items: &[String], ns: Ns) -> Vec<String> {
    let mut sorted = items.to_vec();
    let keygen = natsort_keygen(ns);
    sorted.sort_by_cached_key(|s| keygen(s));
    sorted
}

fn bench_sort_strategies(c: &mut Criterion) {
    let mut group = c.benchmark_group("natsort_strategies");
    for &n in &[100usize, 1_000, 10_000] {
        let input = make_input(n);

        group.bench_with_input(BenchmarkId::new("comparator (old)", n), &input, |b, input| {
            b.iter(|| sort_with_comparator(black_box(input), Ns::DEFAULT));
        });

        group.bench_with_input(
            BenchmarkId::new("sort_by_cached_key (current)", n),
            &input,
            |b, input| {
                b.iter(|| sort_with_cached_key(black_box(input), Ns::DEFAULT));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_sort_strategies);
criterion_main!(benches);
