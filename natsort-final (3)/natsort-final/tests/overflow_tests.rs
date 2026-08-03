//! Overflow-focused tests: integers and floats at or beyond the limits of
//! their underlying Rust representation. See DECISIONS.md #13 for why
//! `Chunk::BigInt` exists.

use natsort_core::{natsort_key, Chunk, Ns};

#[test]
fn i128_min_boundary_parses_exactly() {
    let s = i128::MIN.to_string();
    let key = natsort_key(&s, Ns::SIGNED);
    assert_eq!(key, vec![Chunk::Text(String::new()), Chunk::Int(i128::MIN)]);
}

#[test]
fn one_digit_past_i128_max_triggers_the_bigint_fallback() {
    let one_past_max = "170141183460469231731687303715884105728"; // i128::MAX + 1
    let key = natsort_key(one_past_max, Ns::DEFAULT);
    match &key[1] {
        Chunk::BigInt(neg, digits) => {
            assert!(!*neg);
            assert_eq!(digits.as_slice(), one_past_max.as_bytes());
        }
        other => panic!("expected Chunk::BigInt, got {other:?}"),
    }
}

#[test]
fn bigint_chunks_with_equal_magnitude_are_equal() {
    let a = natsort_key(&"9".repeat(60), Ns::DEFAULT);
    let b = natsort_key(&"9".repeat(60), Ns::DEFAULT);
    assert_eq!(a, b);
}

#[test]
fn negative_bigint_orders_below_positive_bigint() {
    let neg = format!("-{}", "9".repeat(60));
    let pos = "9".repeat(60);
    assert!(natsort_key(&neg, Ns::SIGNED) < natsort_key(&pos, Ns::SIGNED));
}

#[test]
fn more_negative_bigint_orders_lower() {
    // Between two negative numbers, the one with the larger magnitude is
    // the more negative (numerically smaller) value.
    let more_negative = format!("-{}", "9".repeat(61));
    let less_negative = format!("-{}", "9".repeat(60));
    assert!(
        natsort_key(&more_negative, Ns::SIGNED) < natsort_key(&less_negative, Ns::SIGNED)
    );
}

#[test]
fn bigint_always_sorts_above_any_in_range_int() {
    let fits_in_i128 = natsort_key("999999999999999999999999999999999999", Ns::DEFAULT); // 38 nines
    let overflows = natsort_key(&"9".repeat(45), Ns::DEFAULT);
    assert!(fits_in_i128 < overflows);
}

#[test]
fn float_mode_extreme_positive_exponent_saturates_to_infinity() {
    // f64 (like Python's float) saturates to infinity for exponents this
    // large rather than erroring — this mirrors, not diverges from,
    // upstream float-parsing behavior for REAL/FLOAT mode.
    let key = natsort_key("1e400", Ns::FLOAT);
    assert_eq!(key, vec![Chunk::Text(String::new()), Chunk::Float(f64::INFINITY)]);
}

#[test]
fn float_mode_extreme_negative_exponent_underflows_to_zero() {
    let key = natsort_key("1e-400", Ns::FLOAT);
    assert_eq!(key, vec![Chunk::Text(String::new()), Chunk::Float(0.0)]);
}
