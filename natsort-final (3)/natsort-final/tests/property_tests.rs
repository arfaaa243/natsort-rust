//! Hand-rolled property-based tests.
//!
//! This crate ships zero runtime dependencies by design (DECISIONS.md #2,
//! #7), and the environment these tests were written in has no network
//! access to fetch `proptest` from crates.io and no Rust toolchain
//! available to confirm it would even compile here (the same caveat
//! BENCHMARK.md already states for `cargo bench` / `criterion`). Rather
//! than add an unverified dev-dependency, these tests use a small
//! deterministic linear-congruential generator (LCG) — no external
//! randomness source, fully reproducible from the seed in each test, in
//! the same spirit as `benches/sort_benchmark.rs`'s hand-rolled shuffle.
//! If/when `proptest` becomes available in the build environment,
//! swapping this file for real `proptest!` blocks would be a reasonable,
//! clearly-scoped follow-up.

use natsort_core::{natsort_key, natsorted_by, Ns};
use std::collections::HashMap;

/// Deterministic LCG (Numerical Recipes constants) so every test run
/// explores the same input space and any failure is reproducible.
struct Lcg(u64);

impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn next_range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
}

/// Characters covering ASCII letters/digits, punctuation relevant to
/// SIGNED/FLOAT parsing, a few Unicode digit scripts, isolated digit
/// characters, and non-digit Unicode (CJK, emoji) to fuzz text handling.
const CHAR_POOL: &[char] = &[
    'a', 'b', 'c', 'A', 'B', '0', '1', '2', '9', '-', '+', '.', ' ', '_', 'e', 'E',
    '\u{0661}', '\u{FF11}', '\u{00B2}', '\u{2460}', '文', '😀',
];

fn random_string(rng: &mut Lcg, max_len: usize) -> String {
    let len = rng.next_range(max_len + 1);
    (0..len)
        .map(|_| CHAR_POOL[rng.next_range(CHAR_POOL.len())])
        .collect()
}

fn random_ns(rng: &mut Lcg) -> Ns {
    Ns {
        float: rng.next_range(2) == 0,
        signed: rng.next_range(2) == 0,
        ignorecase: rng.next_range(2) == 0,
        lowercasefirst: rng.next_range(2) == 0,
        groupletters: rng.next_range(2) == 0,
    }
}

fn multiset(items: &[String]) -> HashMap<&str, usize> {
    let mut m = HashMap::new();
    for i in items {
        *m.entry(i.as_str()).or_insert(0) += 1;
    }
    m
}

#[test]
fn natsorted_is_always_a_permutation_of_its_input() {
    let mut rng = Lcg(0xC0FFEE);
    for trial in 0..200 {
        let n = rng.next_range(12);
        let items: Vec<String> = (0..n).map(|_| random_string(&mut rng, 8)).collect();
        let ns = random_ns(&mut rng);
        let sorted = natsorted_by(&items, ns);
        assert_eq!(
            multiset(&items),
            multiset(&sorted),
            "trial {trial}: sorting changed the multiset of items (ns={ns:?})"
        );
    }
}

#[test]
fn sorting_twice_is_idempotent() {
    let mut rng = Lcg(0xBEEF);
    for trial in 0..200 {
        let n = rng.next_range(12);
        let items: Vec<String> = (0..n).map(|_| random_string(&mut rng, 8)).collect();
        let ns = random_ns(&mut rng);
        let once = natsorted_by(&items, ns);
        let twice = natsorted_by(&once, ns);
        assert_eq!(
            once, twice,
            "trial {trial}: sorting an already-sorted list changed it (ns={ns:?})"
        );
    }
}

#[test]
fn natsort_key_never_panics_on_arbitrary_input() {
    let mut rng = Lcg(0x5EED);
    for _ in 0..500 {
        let s = random_string(&mut rng, 16);
        for ns in [Ns::DEFAULT, Ns::SIGNED, Ns::FLOAT, Ns::REAL] {
            let _ = natsort_key(&s, ns); // must not panic
        }
    }
}

#[test]
fn key_ordering_agrees_with_full_sort_ordering() {
    // For random pairs, the natsort_key `Ord` relation must agree with
    // where natsorted_by actually places them.
    let mut rng = Lcg(0x1234_5678);
    for trial in 0..300 {
        let a = random_string(&mut rng, 8);
        let b = random_string(&mut rng, 8);
        let ns = random_ns(&mut rng);
        let key_a = natsort_key(&a, ns);
        let key_b = natsort_key(&b, ns);
        let pair = vec![a.clone(), b.clone()];
        let sorted = natsorted_by(&pair, ns);

        if key_a < key_b {
            assert_eq!(
                sorted,
                vec![a, b],
                "trial {trial}: key says a<b but sort disagreed (ns={ns:?})"
            );
        } else if key_b < key_a {
            assert_eq!(
                sorted,
                vec![b, a],
                "trial {trial}: key says b<a but sort disagreed (ns={ns:?})"
            );
        }
        // Equal keys: a stable sort of a 2-element equal-comparing input
        // always preserves original order — nothing extra to assert.
    }
}

#[test]
fn reversing_the_input_order_never_changes_the_sorted_result_for_distinct_keys() {
    // Sorting is a function of the multiset of keys, not of input order,
    // as long as no two items share a key (ties are where input order
    // matters, and that's covered by the stability tests instead).
    let mut rng = Lcg(0x9E3779B9);
    for trial in 0..100 {
        let n = rng.next_range(10) + 1;
        // Build items with distinct numeric suffixes so keys can't tie.
        let items: Vec<String> = (0..n).map(|i| format!("item{i}")).collect();
        let mut reversed = items.clone();
        reversed.reverse();
        let ns = random_ns(&mut rng);
        assert_eq!(
            natsorted_by(&items, ns),
            natsorted_by(&reversed, ns),
            "trial {trial}: sort result depended on input order for distinct keys (ns={ns:?})"
        );
    }
}
