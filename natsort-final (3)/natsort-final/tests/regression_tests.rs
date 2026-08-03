//! Regression tests: each test here pins down a specific fixed bug (or a
//! previously-undertested public-API guarantee) so it can't silently come
//! back. See DECISIONS.md for the reasoning behind each fix.

use natsort_core::{natsort_key, natsorted, natsorted_by, Chunk, Ns};

#[test]
fn integer_overflowing_i128_no_longer_collapses_to_zero() {
    // Regression for DECISIONS.md #13 (né #2): numbers with more than ~39 digits
    // used to silently parse as 0 (`raw.parse().unwrap_or(0)`), making
    // every such number compare equal regardless of actual magnitude.
    // They now fall back to a magnitude-preserving `Chunk::BigInt`.
    let huge = "9".repeat(50); // far bigger than i128::MAX (39 digits)
    let bigger = "9".repeat(51);
    let key_huge = natsort_key(&huge, Ns::DEFAULT);
    let key_bigger = natsort_key(&bigger, Ns::DEFAULT);
    assert_ne!(
        key_huge, key_bigger,
        "two different huge numbers must not compare equal"
    );
    assert!(
        key_huge < key_bigger,
        "a 50-digit number of 9s must sort before a 51-digit number of 9s"
    );
}

#[test]
fn overflowing_number_still_sorts_correctly_relative_to_ordinary_numbers() {
    let items: Vec<String> = vec![
        "item99".to_string(),
        format!("item{}", "9".repeat(45)),
        "item5".to_string(),
    ];
    let sorted = natsorted(&items);
    assert_eq!(
        sorted,
        vec![
            "item5".to_string(),
            "item99".to_string(),
            format!("item{}", "9".repeat(45)),
        ]
    );
}

#[test]
fn negative_overflowing_number_sorts_before_everything_positive() {
    let items: Vec<String> = vec![
        format!("-{}", "9".repeat(45)),
        "-5".to_string(),
        "5".to_string(),
    ];
    let sorted = natsorted_by(&items, Ns::SIGNED);
    assert_eq!(
        sorted,
        vec![format!("-{}", "9".repeat(45)), "-5".to_string(), "5".to_string()]
    );
}

#[test]
fn overflowing_number_with_leading_zeros_compares_by_true_magnitude() {
    let with_leading_zero = format!("0{}", "9".repeat(45));
    let without = "9".repeat(45);
    assert_eq!(
        natsort_key(&with_leading_zero, Ns::DEFAULT),
        natsort_key(&without, Ns::DEFAULT)
    );
}

#[test]
fn i128_max_boundary_still_uses_the_fast_int_path_not_bigint() {
    // Exactly at the i128 boundary: must still be Chunk::Int, not
    // Chunk::BigInt — only genuine overflow should trigger the fallback.
    let max = i128::MAX.to_string();
    let key = natsort_key(&max, Ns::DEFAULT);
    assert_eq!(key, vec![Chunk::Text(String::new()), Chunk::Int(i128::MAX)]);
}

#[test]
fn leading_plus_sign_is_accepted_under_signed_mode() {
    // Rust's integer FromStr has accepted a leading '+' since well before
    // this crate's edition (rust-lang/rust#28826), so this was already
    // correct, but it wasn't previously covered by a dedicated test.
    let key = natsort_key("+5", Ns::SIGNED);
    assert_eq!(key, vec![Chunk::Text(String::new()), Chunk::Int(5)]);
}
