//! natsort_core — natural ("human") sort ordering, ported from Python's
//! `natsort`.
//!
//! The core idea: split a string into alternating runs of text and numbers,
//! compare text runs lexicographically and number runs numerically. That
//! makes `"file2"` sort before `"file10"`, unlike a plain byte-wise compare.
//!
//! # Scope / honesty note
//! This is a first working implementation, not a claim of full parity with
//! upstream `natsort`. Known gaps vs. the full Python library:
//! - Unicode decimal-digit support covers ASCII plus a handful of common
//!   scripts (Arabic-Indic, Extended Arabic-Indic, Devanagari, Bengali,
//!   Thai, Fullwidth). Isolated digit characters (circled ①, superscript ²)
//!   and numeric non-digits (Roman numerals, vulgar fractions) are not yet
//!   handled.
//! - Integers are parsed into `i128`; a number with more digits than fit in
//!   an `i128` will not compare correctly against another such number
//!   (falls back to 0). Arbitrary-precision integers are a follow-up.
//! - `GROUPLETTERS` and `LOWERCASEFIRST` are implemented as reasonable
//!   approximations of natsort's behavior, not verified against upstream.

use std::cmp::Ordering;

/// Sort algorithm flags, mirroring natsort's `ns` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ns {
    /// Parse numbers as floating point (decimal point + exponent) instead
    /// of plain integers.
    pub float: bool,
    /// Recognize a leading `+`/`-` immediately before a number as its sign.
    pub signed: bool,
    /// Case-insensitive comparison of text runs.
    pub ignorecase: bool,
    /// Sort lowercase letters before uppercase (inverts default ASCII order).
    pub lowercasefirst: bool,
    /// Group letters by case-folded value first, original case as tiebreak.
    pub groupletters: bool,
}

impl Ns {
    pub const DEFAULT: Ns = Ns {
        float: false,
        signed: false,
        ignorecase: false,
        lowercasefirst: false,
        groupletters: false,
    };

    pub const SIGNED: Ns = Ns {
        signed: true,
        ..Ns::DEFAULT
    };

    pub const FLOAT: Ns = Ns {
        float: true,
        ..Ns::DEFAULT
    };

    pub const REAL: Ns = Ns {
        float: true,
        signed: true,
        ..Ns::DEFAULT
    };

    pub fn with_ignorecase(mut self) -> Ns {
        self.ignorecase = true;
        self
    }

    pub fn with_lowercasefirst(mut self) -> Ns {
        self.lowercasefirst = true;
        self
    }

    pub fn with_groupletters(mut self) -> Ns {
        self.groupletters = true;
        self
    }
}

/// One piece of a natsort key: either a text run or a parsed number.
#[derive(Debug, Clone, PartialEq)]
pub enum Chunk {
    Text(String),
    Int(i128),
    Float(f64),
}

impl Eq for Chunk {}

impl PartialOrd for Chunk {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Chunk {
    fn cmp(&self, other: &Self) -> Ordering {
        use Chunk::*;
        match (self, other) {
            (Text(a), Text(b)) => a.cmp(b),
            (Int(a), Int(b)) => a.cmp(b),
            (Float(a), Float(b)) => a.total_cmp(b),
            // Chunk sequences always alternate Text/Number starting with
            // Text, so mismatched variants shouldn't occur in practice.
            // Keep the comparison total anyway.
            (Text(_), _) => Ordering::Less,
            (_, Text(_)) => Ordering::Greater,
            (Int(_), Float(_)) => Ordering::Less,
            (Float(_), Int(_)) => Ordering::Greater,
        }
    }
}

/// Contiguous Unicode decimal-digit (Nd) block start codepoints we support.
/// Each block is 10 consecutive codepoints, digit 0 through 9.
const DIGIT_BLOCK_STARTS: &[u32] = &[
    0x0660, // Arabic-Indic
    0x06F0, // Extended Arabic-Indic (Persian)
    0x0966, // Devanagari
    0x09E6, // Bengali
    0x0E50, // Thai
    0xFF10, // Fullwidth
];

fn digit_value(c: char) -> Option<u32> {
    if c.is_ascii_digit() {
        return Some(c as u32 - '0' as u32);
    }
    let cp = c as u32;
    for &start in DIGIT_BLOCK_STARTS {
        if cp >= start && cp < start + 10 {
            return Some(cp - start);
        }
    }
    None
}

fn swap_ascii_case(c: char) -> char {
    if c.is_ascii_uppercase() {
        c.to_ascii_lowercase()
    } else if c.is_ascii_lowercase() {
        c.to_ascii_uppercase()
    } else {
        c
    }
}

fn apply_text_transform(text: &str, ns: Ns) -> String {
    let mut s = text.to_string();
    if ns.lowercasefirst {
        s = s.chars().map(swap_ascii_case).collect();
    }
    if ns.groupletters {
        s = s
            .chars()
            .flat_map(|c| {
                let mut v: Vec<char> = c.to_lowercase().collect();
                v.push(c);
                v
            })
            .collect();
    }
    if ns.ignorecase {
        s = s.to_lowercase();
    }
    s
}

fn is_number_start(chars: &[char], i: usize, ns: Ns) -> bool {
    let n = chars.len();
    if digit_value(chars[i]).is_some() {
        return true;
    }
    if ns.signed && (chars[i] == '+' || chars[i] == '-') {
        if i + 1 < n && digit_value(chars[i + 1]).is_some() {
            return true;
        }
        if ns.float && i + 1 < n && chars[i + 1] == '.' && i + 2 < n && digit_value(chars[i + 2]).is_some()
        {
            return true;
        }
    }
    if ns.float && chars[i] == '.' && i + 1 < n && digit_value(chars[i + 1]).is_some() {
        return true;
    }
    false
}

/// Parse a number run starting at `chars[i]`. Returns the chunk and the
/// index just past the consumed run.
fn parse_number(chars: &[char], i: usize, ns: Ns) -> (Chunk, usize) {
    let n = chars.len();
    let mut j = i;
    let mut raw = String::new();

    if ns.signed && j < n && (chars[j] == '+' || chars[j] == '-') {
        raw.push(chars[j]);
        j += 1;
    }

    let mut has_digits = false;
    while j < n {
        if let Some(d) = digit_value(chars[j]) {
            raw.push(std::char::from_digit(d, 10).unwrap());
            has_digits = true;
            j += 1;
        } else {
            break;
        }
    }

    if !ns.float {
        let value: i128 = raw.parse().unwrap_or(0);
        return (Chunk::Int(value), j);
    }

    // Decimal point.
    if j < n && chars[j] == '.' {
        let next_is_digit = j + 1 < n && digit_value(chars[j + 1]).is_some();
        if next_is_digit || has_digits {
            raw.push('.');
            j += 1;
            while j < n {
                if let Some(d) = digit_value(chars[j]) {
                    raw.push(std::char::from_digit(d, 10).unwrap());
                    j += 1;
                } else {
                    break;
                }
            }
        }
    }

    // Exponent.
    if j < n && (chars[j] == 'e' || chars[j] == 'E') {
        let mut k = j + 1;
        let mut exp_raw = String::new();
        if k < n && (chars[k] == '+' || chars[k] == '-') {
            exp_raw.push(chars[k]);
            k += 1;
        }
        let exp_start = exp_raw.len();
        while k < n {
            if let Some(d) = digit_value(chars[k]) {
                exp_raw.push(std::char::from_digit(d, 10).unwrap());
                k += 1;
            } else {
                break;
            }
        }
        // Only accept the exponent if it has at least one digit after the sign.
        if exp_raw.len() > exp_start {
            raw.push('e');
            raw.push_str(&exp_raw);
            j = k;
        }
    }

    let value: f64 = raw.parse().unwrap_or(0.0);
    (Chunk::Float(value), j)
}

/// Build the natural-sort key for a string: alternating Text/Number chunks,
/// always starting with a (possibly empty) Text chunk.
pub fn natsort_key(s: &str, ns: Ns) -> Vec<Chunk> {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut chunks = Vec::new();
    let mut i = 0;

    loop {
        let start = i;
        while i < n && !is_number_start(&chars, i, ns) {
            i += 1;
        }
        let text: String = chars[start..i].iter().collect();
        chunks.push(Chunk::Text(apply_text_transform(&text, ns)));

        if i >= n {
            break;
        }

        let (chunk, next_i) = parse_number(&chars, i, ns);
        chunks.push(chunk);
        i = next_i;

        if i >= n {
            break;
        }
    }

    chunks
}

/// Compare two strings using natural sort ordering.
///
/// # Performance
/// This recomputes a full [`natsort_key`] for `a` and `b` on every call. It
/// is the right tool for a one-off comparison (e.g. the CLI's `compare`
/// subcommand), but **do not** use it as a sort comparator over a
/// collection — see [`natsort_keygen`] and [`natsorted_by`] for why.
pub fn compare(a: &str, b: &str, ns: Ns) -> Ordering {
    natsort_key(a, ns).cmp(&natsort_key(b, ns))
}

/// Build a reusable natural-sort key function for the given algorithm
/// flags, mirroring Python natsort's `natsort_keygen(alg)`.
///
/// The returned closure is `Copy` (it only captures the `Copy` [`Ns`]
/// flags), so it can be passed around, cloned implicitly, and called many
/// times without any allocation of its own — the allocation happens inside
/// each call, when it builds that item's [`Chunk`] key.
///
/// # Why this exists
/// Passing this to [`slice::sort_by_cached_key`] guarantees the key is
/// computed **exactly once per element**, regardless of how many
/// comparisons the sort algorithm performs. Compare that to sorting with a
/// raw comparator built on [`compare`] (or on [`natsort_key`] called inline
/// in a `sort_by` closure): a comparison sort performs `O(n log n)`
/// comparisons, and a naive comparator recomputes both operands' keys on
/// *every* comparison, so the same string's key can be rebuilt `O(log n)`
/// times over the course of one sort. With `n` items of average length
/// `m`:
///
/// | Approach | Key computations | Total work |
/// |---|---|---|
/// | `sort_by` + inline `natsort_key` | `O(n log n)` | `O(n · m · log n)` |
/// | `sort_by_cached_key` + `natsort_keygen` | `O(n)` | `O(n · m + n log n)` |
///
/// # Examples
/// ```
/// use natsort_core::{natsort_keygen, Ns};
///
/// let keygen = natsort_keygen(Ns::DEFAULT);
/// let mut items = vec!["file10", "file2", "file1"];
/// items.sort_by_cached_key(|s| keygen(s));
/// assert_eq!(items, vec!["file1", "file2", "file10"]);
/// ```
pub fn natsort_keygen(ns: Ns) -> impl Fn(&str) -> Vec<Chunk> + Copy {
    move |s: &str| natsort_key(s, ns)
}

/// Sort a slice of natural-sort-key-comparable items using the given
/// algorithm flags, returning a new sorted `Vec`. Input is not mutated.
///
/// Generic over any `S: AsRef<str> + Clone` so it works for `String`,
/// `&str`, `Cow<str>`, or any newtype wrapping a string, not just `String`.
///
/// Uses [`slice::sort_by_cached_key`] with [`natsort_keygen`] internally —
/// each item's key is computed exactly once. See [`natsort_keygen`] for
/// the complexity argument.
///
/// The sort is stable (same guarantee as Python's `sorted`): items that
/// compare equal retain their relative input order.
pub fn natsorted_by<S: AsRef<str> + Clone>(items: &[S], ns: Ns) -> Vec<S> {
    let mut sorted: Vec<S> = items.to_vec();
    let keygen = natsort_keygen(ns);
    sorted.sort_by_cached_key(|s| keygen(s.as_ref()));
    sorted
}

/// Sort using the default (INT) algorithm.
pub fn natsorted<S: AsRef<str> + Clone>(items: &[S]) -> Vec<S> {
    natsorted_by(items, Ns::DEFAULT)
}

/// Sort using the REAL (signed float) algorithm.
pub fn realsorted<S: AsRef<str> + Clone>(items: &[S]) -> Vec<S> {
    natsorted_by(items, Ns::REAL)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn default_int_sort_matches_readme_example() {
        let input = v(&["file10", "file2", "file1"]);
        assert_eq!(natsorted(&input), v(&["file1", "file2", "file10"]));
    }

    #[test]
    fn default_int_sort_of_decimal_looking_strings() {
        // "1.10" is parsed as [1, ".", 10] under INT, so it sorts by the
        // trailing 10 and ends up last.
        let input = v(&["1.5", "1.10", "1.2"]);
        assert_eq!(natsorted(&input), v(&["1.2", "1.5", "1.10"]));
    }

    #[test]
    fn real_sort_treats_strings_as_floats() {
        let input = v(&["1.5", "1.10", "1.2"]);
        assert_eq!(natsorted_by(&input, Ns::REAL), v(&["1.10", "1.2", "1.5"]));
    }

    #[test]
    fn signed_negative_numbers_compare_numerically() {
        let input = v(&["item-3", "item-10", "item2"]);
        assert_eq!(
            natsorted_by(&input, Ns::SIGNED),
            v(&["item-10", "item-3", "item2"])
        );
    }

    #[test]
    fn ignorecase_folds_text_chunks() {
        let input = v(&["Banana", "apple", "Cherry"]);
        let ns = Ns::DEFAULT.with_ignorecase();
        assert_eq!(natsorted_by(&input, ns), v(&["apple", "Banana", "Cherry"]));
    }

    #[test]
    fn lowercasefirst_inverts_ascii_case_order() {
        let input = v(&["Apple", "apple"]);
        let default_order = natsorted(&input);
        // Default: uppercase 'A' (65) sorts before lowercase 'a' (97).
        assert_eq!(default_order, v(&["Apple", "apple"]));

        let ns = Ns::DEFAULT.with_lowercasefirst();
        let lcf_order = natsorted_by(&input, ns);
        assert_eq!(lcf_order, v(&["apple", "Apple"]));
    }

    #[test]
    fn fullwidth_unicode_digits_parse_as_numbers() {
        // U+FF11 U+FF10 = fullwidth "10"
        let fullwidth_ten = "\u{FF11}\u{FF10}";
        let input = v(&[fullwidth_ten, "2", "1"]);
        assert_eq!(natsorted(&input), v(&["1", "2", fullwidth_ten]));
    }

    #[test]
    fn key_alternates_starting_with_text_chunk() {
        let key = natsort_key("num10", Ns::DEFAULT);
        assert_eq!(key, vec![Chunk::Text("num".to_string()), Chunk::Int(10)]);
    }

    #[test]
    fn empty_string_produces_single_empty_text_chunk() {
        let key = natsort_key("", Ns::DEFAULT);
        assert_eq!(key, vec![Chunk::Text(String::new())]);
    }

    #[test]
    fn compare_matches_natsorted_ordering() {
        assert_eq!(compare("file2", "file10", Ns::DEFAULT), Ordering::Less);
        assert_eq!(compare("file10", "file10", Ns::DEFAULT), Ordering::Equal);
    }

    // --- Additional coverage: regression guards for behavior already
    // implemented but not previously exercised by a dedicated test. ---

    #[test]
    fn signed_flag_does_not_treat_hyphen_as_sign_without_digit() {
        let key = natsort_key("item-", Ns::SIGNED);
        assert_eq!(key, vec![Chunk::Text("item-".to_string())]);
    }

    #[test]
    fn real_mode_parses_exponent_notation() {
        let ns = Ns::REAL;
        let input = v(&["1e3", "1e10", "1e2"]);
        assert_eq!(natsorted_by(&input, ns), v(&["1e2", "1e3", "1e10"]));
    }

    #[test]
    fn real_mode_rejects_dangling_exponent_sign() {
        let key = natsort_key("1e", Ns::REAL);
        assert_eq!(
            key,
            vec![
                Chunk::Text(String::new()),
                Chunk::Float(1.0),
                Chunk::Text("e".to_string())
            ]
        );
    }

    #[test]
    fn groupletters_sorts_case_insensitively_first_then_by_case() {
        let ns = Ns::DEFAULT.with_groupletters();
        let input = v(&["b", "B", "a", "A"]);
        let sorted = natsorted_by(&input, ns);
        assert_eq!(sorted, v(&["A", "a", "B", "b"]));
    }

    #[test]
    fn natsorted_is_stable_on_equal_keys() {
        let ns = Ns::DEFAULT.with_ignorecase();
        let input = v(&["Apple", "APPLE", "apple"]);
        assert_eq!(natsorted_by(&input, ns), v(&["Apple", "APPLE", "apple"]));
    }

    #[test]
    fn arabic_indic_digits_parse_as_numbers() {
        let arabic_twelve = "\u{0661}\u{0662}";
        let input = v(&[arabic_twelve, "3", "1"]);
        assert_eq!(natsorted(&input), v(&["1", "3", arabic_twelve]));
    }

    #[test]
    fn empty_input_slice_sorts_to_empty() {
        let input: Vec<String> = v(&[]);
        assert_eq!(natsorted(&input), v(&[]));
    }

    #[test]
    fn single_item_slice_is_unchanged() {
        let input = v(&["only"]);
        assert_eq!(natsorted(&input), v(&["only"]));
    }
}
