# Benchmarks

## Status: still not yet run

Same constraint as the pass before this one, re-verified rather than
assumed: this pass's environment also has no Rust toolchain
(`rustc`/`cargo` not on `PATH`) and no network access (`crates.io` is
blocked by an egress allowlist, and `apt-get install rustc cargo` fails
with `403 Forbidden` on every configured mirror). **The numbers in this
file are still placeholders showing the expected table shape, not real
measurements.** Do not cite them as evidence — run the harness yourself
and replace this table.

In the meantime, `tests/perf_regression.rs` (new this pass) adds a coarse
"does sorting 100k items finish in well under 10 seconds" tripwire test —
not a substitute for real benchmark numbers, but at least a guard against
an accidental quadratic regression slipping in unnoticed before someone
runs `cargo bench` for real.

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
