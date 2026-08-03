//! Unicode edge cases: digit blocks beyond the original six, mixed-script
//! digit runs, and isolated (non-concatenating) digit characters.

use natsort_core::{natsort_key, natsorted, Chunk, Ns};

fn ch(cp: u32) -> char {
    char::from_u32(cp).expect("valid BMP codepoint")
}

#[test]
fn devanagari_digits_parse_as_numbers() {
    let s = format!("{}{}", ch(0x0967), ch(0x0968)); // "12"
    assert_eq!(
        natsort_key(&s, Ns::DEFAULT),
        vec![Chunk::Text(String::new()), Chunk::Int(12)]
    );
}

#[test]
fn tamil_digits_parse_as_numbers() {
    let s: String = [0x0BE7, 0x0BE8, 0x0BE9].iter().map(|&c| ch(c)).collect(); // "123"
    assert_eq!(
        natsort_key(&s, Ns::DEFAULT),
        vec![Chunk::Text(String::new()), Chunk::Int(123)]
    );
}

#[test]
fn khmer_digits_parse_as_numbers() {
    let s: String = [0x17E5, 0x17E0].iter().map(|&c| ch(c)).collect(); // "50"
    assert_eq!(
        natsort_key(&s, Ns::DEFAULT),
        vec![Chunk::Text(String::new()), Chunk::Int(50)]
    );
}

#[test]
fn mongolian_digits_parse_as_numbers() {
    let s: String = [0x1811, 0x1810].iter().map(|&c| ch(c)).collect(); // "10"
    assert_eq!(
        natsort_key(&s, Ns::DEFAULT),
        vec![Chunk::Text(String::new()), Chunk::Int(10)]
    );
}

#[test]
fn tibetan_digits_parse_as_numbers() {
    let s: String = [0x0F21, 0x0F22].iter().map(|&c| ch(c)).collect(); // "12"
    assert_eq!(
        natsort_key(&s, Ns::DEFAULT),
        vec![Chunk::Text(String::new()), Chunk::Int(12)]
    );
}

#[test]
fn myanmar_and_myanmar_shan_digits_are_separate_but_both_recognized() {
    // Myanmar (U+1040) and Myanmar Shan (U+1090) are distinct Nd blocks
    // in the Unicode Character Database; both are independently
    // registered in DIGIT_BLOCK_STARTS.
    let myanmar_two = ch(0x1042);
    let shan_two = ch(0x1092);
    assert_eq!(
        natsort_key(&myanmar_two.to_string(), Ns::DEFAULT),
        vec![Chunk::Text(String::new()), Chunk::Int(2)]
    );
    assert_eq!(
        natsort_key(&shan_two.to_string(), Ns::DEFAULT),
        vec![Chunk::Text(String::new()), Chunk::Int(2)]
    );
}

#[test]
fn mixed_script_digit_run_concatenates_like_a_single_number() {
    // A digit run mixing ASCII and Arabic-Indic digits concatenates into
    // one multi-digit number — matching how Python's `re` module's `\d+`
    // matches any run of Unicode-decimal-digit characters regardless of
    // script boundaries. Best-effort parity with upstream; not verified
    // against a live Python `natsort` run in this environment (no network
    // access to install it — see DECISIONS.md and the README's "Current
    // state" table).
    let s = format!("1{}", ch(0x0662)); // ASCII '1' + Arabic-Indic '٢' (2)
    assert_eq!(
        natsort_key(&s, Ns::DEFAULT),
        vec![Chunk::Text(String::new()), Chunk::Int(12)]
    );
}

#[test]
fn isolated_digits_do_not_merge_with_a_following_ascii_digit() {
    // "²1" -> superscript 2 (isolated, standalone) then ASCII "1".
    let s = format!("{}1", ch(0x00B2));
    assert_eq!(
        natsort_key(&s, Ns::DEFAULT),
        vec![
            Chunk::Text(String::new()),
            Chunk::Int(2),
            Chunk::Text(String::new()),
            Chunk::Int(1),
        ]
    );
}

#[test]
fn sorting_mixed_unicode_and_ascii_filenames_orders_numerically() {
    let devanagari_ten = format!("file{}{}", ch(0x0967), ch(0x0966)); // "10"
    let items = vec![devanagari_ten.clone(), "file2".to_string(), "file1".to_string()];
    assert_eq!(
        natsorted(&items),
        vec!["file1".to_string(), "file2".to_string(), devanagari_ten]
    );
}

#[test]
fn emoji_and_non_digit_symbols_are_plain_text() {
    let key = natsort_key("😀🎉", Ns::DEFAULT);
    assert_eq!(key, vec![Chunk::Text("😀🎉".to_string())]);
}

#[test]
fn combining_diacritics_are_not_normalized_against_precomposed_forms() {
    // "e" + combining acute (U+0065 U+0301) vs precomposed "é" (U+00E9)
    // are different Chunk::Text values — this crate does not perform
    // Unicode normalization (NFC/NFD), i.e. a plain codepoint-sequence
    // comparison rather than a locale-aware one. This matches upstream
    // natsort's own non-normalizing default, not a divergence introduced
    // by this port.
    let decomposed = "e\u{0301}";
    let precomposed = "\u{00E9}";
    assert_ne!(
        natsort_key(decomposed, Ns::DEFAULT),
        natsort_key(precomposed, Ns::DEFAULT)
    );
}
