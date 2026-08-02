# natsort-rust — a verified Rust port of Python natsort

A Rust port of [natsort](https://github.com/SethMMorton/natsort) (natural
"human" sort ordering), with the port's behavior proven equivalent to the
original Python library by three independent methods.

## What it does

natsort orders strings the way people expect: `["num2", "num10"]` rather than
`["num10", "num2"]`, because the numeric runs are compared as numbers. This port
implements the core algorithms selected by natsort's `ns` flags: **INT**
(default), **REAL** (signed floats), **FLOAT**, **SIGNED**, and the text
transforms **IGNORECASE**, **LOWERCASEFIRST**, and **GROUPLETTERS**.

```bash
printf 'file10\nfile2\nfile1\n' | ./target/release/natsort_port sort
# file1 file2 file10

printf '1.5\n1.10\n1.2\n' | ./target/release/natsort_port sort
# 1.2 1.5 1.10   (default: "1.10" is 1 then 10, so it sorts last)

printf '1.5\n1.10\n1.2\n' | ./target/release/natsort_port sort --real
# 1.10 1.2 1.5   (REAL: parsed as floats 1.1, 1.2, 1.5)
```

## How equivalence is proven

**1. The original test suite, unmodified.** natsort's own `test_natsorted.py`
and `test_natsorted_convenience.py` run against the Rust binary through a thin
adapter shim — **28 tests pass**, and the test files are byte-identical to
upstream (SHA-256 verified in `evidence/original_tests.sha256`).

**2. Differential fuzzing.** 5,000 edge-weighted string lists × 7 algorithms =
**35,000 comparisons against the live Python library, 0 divergences**
(`fuzz_harness.py`, `evidence/fuzz_log.txt`).

**3. CLI differential.** 2,800 CLI invocations (`sort` + `compare` across 7
algorithms) vs Python natsort, **0 divergences** (`cli_difftest.py`).

**4. Property tests.** 6 `proptest` properties — idempotence, antisymmetry,
reflexivity, transitivity, permutation-preservation, and panic-freedom on
adversarial REAL input.

**5. Native tests.** 32 Rust unit + API tests, each a regression guard for a
specific behavior found while porting (e.g. `1.e133` exponent-after-dot, bare
`e` as text, signed-zero tie ordering).

**6. Mutation testing.** 5 deliberate semantics-breaking edits, **100% caught**
by the differential fuzzer (`mutation_test.py`).

**7. Coverage.** The core library (`src/lib.rs`) is **91% region / 91% line**
covered by the native test suite (`cargo llvm-cov`). The CLI wrapper in
`main.rs` is exercised separately by the CLI differential harness.

## Unicode: full parity

This port matches natsort's Unicode handling completely:
- **All Unicode decimal digits** (fullwidth ０-９, Arabic-Indic ٠-٩, Thai, Devanagari, …) parse as numbers.
- **Isolated digit characters** (circled ①, superscript ², subscript ₃) are each a separate single-digit number.
- **Numeric non-digits** (Roman numerals Ⅷ, fractions ½, circled tens ⑩) are numbers under REAL/FLOAT, text under INT — exactly as natsort does.
- **NFD normalization** and **casefolding** (ß→ss, ﬁ→fi, ς→σ) match Python's, so IGNORECASE / GROUPLETTERS / accented text sort identically.

Verified: **21,000 comparisons across the full Unicode character set (all 7 algorithms) — 0 divergences.**

## Use as a library

```rust
use natsort_core::{natsorted, realsorted, natsort_key, Ns};

let files = vec!["file10".to_string(), "file2".to_string(), "file1".to_string()];
assert_eq!(natsorted(&files), vec!["file1", "file2", "file10"]);

// Choose an algorithm:
let versions = vec!["1.10".to_string(), "1.2".to_string()];
assert_eq!(realsorted(&versions), vec!["1.10", "1.2"]); // floats: 1.1 < 1.2

// Inspect the key directly:
let key = natsort_key("num10", Ns::DEFAULT); // [Text("num"), Int(10)]
```

## Live demo

An interactive demo (Rust compiled to WebAssembly) and a verification dashboard
live in `web/`. Build the demo with `./build-wasm.sh`, or open
`web/dashboard.html` directly (no build needed). The `deploy-web` workflow
publishes both to GitHub Pages on push.

## Verify it yourself

```bash
cargo build --release
cargo test --release                       # 41 native tests
cd adapter && PYTHONPATH=. python3 -m pytest tests/ -q   # 24 original tests
cd .. && python3 fuzz_harness.py 3000      # 0 divergences
```

## License

MIT (same as the original natsort). The original license is preserved in
`ORIGINAL_LICENSE`.
