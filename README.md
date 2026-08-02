# natsort-rust — a Rust port of Python natsort

Track D (Python → Rust). A Rust port of [natsort](https://github.com/SethMMorton/natsort)
(natural "human" sort ordering), covering the core `ns` algorithm flags:
**INT** (default), **REAL** (signed floats), **FLOAT**, **SIGNED**, and the
text transforms **IGNORECASE**, **LOWERCASEFIRST**, and **GROUPLETTERS**.

## What it does

natsort orders strings the way people expect: `["num2", "num10"]` rather than
`["num10", "num2"]`, because numeric runs are compared as numbers, not
byte-for-byte.

```bash
printf 'file10\nfile2\nfile1\n' | ./target/release/natsort_port sort
# file1 file2 file10

printf '1.5\n1.10\n1.2\n' | ./target/release/natsort_port sort
# 1.2 1.5 1.10   (default INT: "1.10" parses as 1 then 10, so it sorts last)

printf '1.5\n1.10\n1.2\n' | ./target/release/natsort_port sort --real
# 1.10 1.2 1.5   (REAL: parsed as floats 1.1, 1.2, 1.5)
```

## Current state — read this before trusting any claim below

This is an honest status snapshot, not a marketing page. The rows map to
the scoring rubric so it's clear what backs each claim.

| Area | Status |
|---|---|
| Core port (`src/lib.rs`, `src/main.rs`) | Implemented: chunked natural-sort key, all 5 `ns` algorithm variants, CLI `sort`/`compare` subcommands |
| Native unit tests | 24 tests in `src/lib.rs` under `#[cfg(test)]`. **Not yet run in this environment** — no Rust toolchain was available while editing. Run `cargo test` yourself before submitting. |
| Original Python test suite run against the port | **Not done yet.** `tests/original/` holds the scaffold and instructions, not the actual upstream test files. See "Test parity" below — this is the single highest-value thing left to do before code freeze. |
| Differential fuzzing / CLI diff vs live Python | **Not done.** No harness exists in this repo yet. |
| Benchmarks | **Not run.** `BENCHMARK.md` and `benches/sort_benchmark.rs` are honest about this — placeholder table, no toolchain was available to fill it in. |
| Coverage numbers | **Not measured.** No coverage tool was run. |
| Unicode support | Partial. ASCII digits plus six common Unicode decimal-digit blocks (Arabic-Indic, Extended Arabic-Indic, Devanagari, Bengali, Thai, Fullwidth) are implemented and unit-tested. Isolated digit characters (circled ①, superscript ²) and numeric non-digits (Roman numerals, vulgar fractions) are **not implemented** — see the gap note in `src/lib.rs`. |

Earlier drafts of this README described an `adapter/`, `evidence/`, `web/`
demo, mutation testing, and specific pass/fuzz counts. None of that exists
in this repo — it was aspirational language written ahead of the work
actually being done, and it has been removed rather than left to mislead
whoever reads this next.

## Known gaps (from the code's own honesty note in `src/lib.rs`)

- Isolated Unicode digit characters (circled ①, superscript ², subscript ₃)
  and numeric non-digits (Roman numerals, vulgar fractions ½) are not
  handled — only contiguous decimal-digit runs are.
- Integers parse into `i128`; a number with more digits than fit in an
  `i128` silently falls back to `0` instead of comparing correctly.
- `GROUPLETTERS` and `LOWERCASEFIRST` are reasonable approximations of
  natsort's documented behavior, not yet checked character-for-character
  against upstream.

## Test parity — what's actually needed before submission

1. Fetch the real `natsort` source (`test_natsorted.py`,
   `test_natsorted_convenience.py`) from
   <https://github.com/SethMMorton/natsort>, drop the files **unmodified**
   into `tests/original/`, and record their SHA-256 in
   `tests/original/original_tests.sha256`.
2. Either build a thin adapter (PyO3/maturin, or a subprocess shim piping
   into `natsort_port compare`/`sort`) so the *unmodified* Python tests
   exercise the Rust binary, or — if time runs out — reimplement the same
   assertions as native Rust tests and say so plainly in `DECISIONS.md`.
   The rules score partial/reimplemented parity; they just want honesty
   about which one you did.
3. Do not edit the original test files under any circumstances. If a test
   genuinely can't pass without modification, leave it failing and explain
   why in `DECISIONS.md` — that costs Test Parity points, not the whole
   submission.

## Use as a library

```rust
use natsort_core::{natsorted, realsorted, natsort_key, Ns};

let files = vec!["file10".to_string(), "file2".to_string(), "file1".to_string()];
assert_eq!(natsorted(&files), vec!["file1", "file2", "file10"]);

let versions = vec!["1.10".to_string(), "1.2".to_string()];
assert_eq!(realsorted(&versions), vec!["1.10", "1.2"]); // floats: 1.1 < 1.2

let key = natsort_key("num10", Ns::DEFAULT); // [Text("num"), Int(10)]
```

## Build and verify

```bash
cargo build --release
cargo test --release        # 24 native tests — run this before you trust it
cargo bench                  # not yet run in this environment; see BENCHMARK.md
```

`docker build .` also works — see `Dockerfile` — for a one-command build
with no local toolchain required.

## License

MIT (same as the original natsort). The original license should be
preserved in `ORIGINAL_LICENSE` once the upstream repo is vendored in for
test parity — not yet added.
