//! Parser edge cases: malformed, boundary, and unusual input shapes.
//! Nothing here was previously broken (that's what `regression_tests.rs`
//! is for) — these pin down behavior that's easy to get wrong and wasn't
//! previously exercised by a dedicated test.

use natsort_core::{natsort_key, natsorted_by, Chunk, Ns};

#[test]
fn leading_zeros_do_not_affect_integer_value() {
    assert_eq!(natsort_key("007", Ns::DEFAULT), natsort_key("7", Ns::DEFAULT));
}

#[test]
fn only_digits_no_text_produces_a_leading_empty_text_chunk() {
    let key = natsort_key("42", Ns::DEFAULT);
    assert_eq!(key, vec![Chunk::Text(String::new()), Chunk::Int(42)]);
}

#[test]
fn double_sign_only_the_one_directly_before_a_digit_counts() {
    // '+' isn't itself a number start (no digit immediately after it,
    // '-' is not a digit) so it's ordinary text; "-5" is the number.
    let key = natsort_key("+-5", Ns::SIGNED);
    assert_eq!(
        key,
        vec![Chunk::Text("+".to_string()), Chunk::Int(-5)]
    );
}

#[test]
fn float_mode_lone_decimal_point_is_text() {
    let key = natsort_key(".", Ns::FLOAT);
    assert_eq!(key, vec![Chunk::Text(".".to_string())]);
}

#[test]
fn float_mode_trailing_dot_is_included_in_the_number() {
    let key = natsort_key("5.", Ns::FLOAT);
    assert_eq!(key, vec![Chunk::Text(String::new()), Chunk::Float(5.0)]);
}

#[test]
fn float_mode_leading_dot_digit_is_a_valid_number() {
    let key = natsort_key(".5", Ns::FLOAT);
    assert_eq!(key, vec![Chunk::Text(String::new()), Chunk::Float(0.5)]);
}

#[test]
fn signed_mode_hyphen_directly_before_text_is_not_a_number() {
    let key = natsort_key("a-b", Ns::SIGNED);
    assert_eq!(key, vec![Chunk::Text("a-b".to_string())]);
}

#[test]
fn empty_string_is_a_single_empty_text_chunk() {
    assert_eq!(natsort_key("", Ns::DEFAULT), vec![Chunk::Text(String::new())]);
}

#[test]
fn whitespace_only_string_is_pure_text() {
    assert_eq!(
        natsort_key("   ", Ns::DEFAULT),
        vec![Chunk::Text("   ".to_string())]
    );
}

#[test]
fn very_long_text_run_with_no_digits_is_a_single_chunk() {
    let long_text = "a".repeat(10_000);
    let key = natsort_key(&long_text, Ns::DEFAULT);
    assert_eq!(key, vec![Chunk::Text(long_text)]);
}

#[test]
fn many_short_alternating_chunks_round_trip_through_the_parser() {
    let s = "a1b2c3d4e5f6g7h8i9j0";
    let key = natsort_key(s, Ns::DEFAULT);
    // 10 text runs + 10 number runs, alternating, starting with text.
    assert_eq!(key.len(), 20);
    assert_eq!(key[0], Chunk::Text("a".to_string()));
    assert_eq!(key[1], Chunk::Int(1));
    assert_eq!(key[18], Chunk::Text("j".to_string()));
    assert_eq!(key[19], Chunk::Int(0));
}

#[test]
fn replacement_character_is_treated_as_ordinary_text() {
    // &str is always valid UTF-8, so "invalid input" in this domain means
    // the replacement character U+FFFD rather than a byte-level encoding
    // error — it's ordinary (non-digit) text.
    let key = natsort_key("a\u{FFFD}b", Ns::DEFAULT);
    assert_eq!(key, vec![Chunk::Text("a\u{FFFD}b".to_string())]);
}

#[test]
fn stability_holds_when_many_items_share_a_key() {
    // All items compare equal (identical text, no numbers), so a stable
    // sort must leave their relative order untouched.
    let items: Vec<String> = (0..50).map(|_| "same".to_string()).collect();
    let sorted = natsorted_by(&items, Ns::DEFAULT);
    assert_eq!(sorted, items);
}

#[test]
fn stability_holds_for_distinct_strings_with_equal_keys_under_ignorecase() {
    let items = vec![
        "Apple".to_string(),
        "APPLE".to_string(),
        "apple".to_string(),
        "ApPlE".to_string(),
    ];
    let ns = Ns::DEFAULT.with_ignorecase();
    let sorted = natsorted_by(&items, ns);
    // All four keys are equal under ignorecase, so original relative
    // order must be preserved exactly.
    assert_eq!(sorted, items);
}

#[test]
fn groupletters_and_ignorecase_can_be_combined() {
    let ns = Ns::DEFAULT.with_groupletters().with_ignorecase();
    // Shouldn't panic and should still fold case for comparison purposes.
    let input = vec!["Banana".to_string(), "apple".to_string()];
    let sorted = natsorted_by(&input, ns);
    assert_eq!(sorted, vec!["apple".to_string(), "Banana".to_string()]);
}
