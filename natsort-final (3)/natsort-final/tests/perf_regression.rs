//! Coarse performance regression guard.
//!
//! This is NOT a benchmark — see `benches/sort_benchmark.rs` and
//! `BENCHMARK.md` for real measurements via `criterion`, which have not
//! been run in this offline environment (see the README's "Current
//! state" table). This file exists only to catch an accidental
//! reintroduction of quadratic-or-worse behavior (e.g. swapping
//! `sort_by_cached_key` back for a naive `sort_by` comparator that
//! recomputes both operands' keys on every comparison — see
//! DECISIONS.md #4) by asserting that sorting a large input finishes
//! within a generous time budget. The threshold is intentionally loose
//! to avoid flakiness on slow or loaded CI runners: it's a tripwire, not
//! a speed claim.

use natsort_core::{compare, natsorted_by, Ns};
use std::time::{Duration, Instant};

fn make_input(n: usize) -> Vec<String> {
    let prefixes = ["file", "img", "track", "chapter", "item", "part"];
    (0..n)
        .map(|i| {
            let prefix = prefixes[i % prefixes.len()];
            let number = (i * 2654435761usize) % (n * 7 + 13);
            format!("{prefix}{number}")
        })
        .collect()
}

#[test]
fn sorting_100k_items_completes_within_a_generous_time_budget() {
    let input = make_input(100_000);
    let start = Instant::now();
    let sorted = natsorted_by(&input, Ns::DEFAULT);
    let elapsed = start.elapsed();
    assert_eq!(sorted.len(), input.len());
    // A correct O(n log n) sort of 100k short strings should take well
    // under a second on any reasonable machine; 10s leaves enormous
    // headroom for slow CI while still catching an accidental quadratic
    // regression.
    assert!(
        elapsed < Duration::from_secs(10),
        "sorting 100k items took {elapsed:?}, expected well under 10s — \
         check for an accidental quadratic regression (see DECISIONS.md #4)"
    );
}

#[test]
fn cached_key_sort_agrees_with_a_naive_comparator_sort_on_correctness() {
    // A correctness cross-check between the two strategies compared in
    // benches/sort_benchmark.rs — speed comparison is what that benchmark
    // is for, this just confirms they produce the same order.
    let input = make_input(500);
    let mut naive = input.clone();
    naive.sort_by(|a, b| compare(a, b, Ns::DEFAULT));
    let cached = natsorted_by(&input, Ns::DEFAULT);
    assert_eq!(naive, cached);
}
