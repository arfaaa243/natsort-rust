# Benchmarks

## Status: not yet run

The environment used to write this crate this session has no Rust
toolchain (`rustc`/`cargo` not found) and no network access (so `criterion`
can't be fetched even if `cargo` were present). **The numbers in this file
are placeholders showing the expected table shape, not real measurements.**
Do not cite them as evidence — run the harness yourself and replace this
table.

## How to run it

```bash
cargo bench
```

This runs `benches/sort_benchmark.rs`, which compares:
- `comparator (old)` — the pre-fix `sort_by` + inline `compare()` approach
- `sort_by_cached_key (current)` — the `natsort_keygen` + `sort_by_cached_key` approach

across `n ∈ {100, 1_000, 10_000}` items of natural-sort-flavored test data
(mixed text prefixes + numeric suffixes, deterministically shuffled so the
input isn't pre-sorted).

Criterion writes an HTML report to `target/criterion/report/index.html` and
prints a text summary to stdout, e.g.:

```
natsort_strategies/comparator (old)/1000
                        time:   [XX.XX ms XX.XX ms XX.XX ms]
natsort_strategies/sort_by_cached_key (current)/1000
                        time:   [XX.XX ms XX.XX ms XX.XX ms]
```

## Results (placeholder — replace after running)

| n | comparator (old) | sort_by_cached_key (current) | Speedup |
|---|---|---|---|
| 100 | *(run `cargo bench`)* | *(run `cargo bench`)* | *(fill in)* |
| 1,000 | *(run `cargo bench`)* | *(run `cargo bench`)* | *(fill in)* |
| 10,000 | *(run `cargo bench`)* | *(run `cargo bench`)* | *(fill in)* |

## Performance graph (Markdown/ASCII placeholder)

Once real numbers exist, a simple relative-time bar chart in Markdown
(no image dependency, renders anywhere):

```
n = 10,000
comparator (old)              ████████████████████████████  (fill in ms)
sort_by_cached_key (current)  ████████                       (fill in ms)
```

## What "success" looks like

Given the complexity argument in `PERFORMANCE.md` (`O(n·m·log n)` vs.
`O(n·m + n log n)`), the expected shape is: roughly constant or mildly
growing ratio at small `n`, widening gap as `n` grows past a few thousand,
most pronounced on longer average string lengths. If the measured numbers
*don't* show that shape, that's a signal something in the analysis or the
implementation is off — worth investigating rather than editing the
narrative to fit, per this project's own standard of not asserting
unverified claims.
