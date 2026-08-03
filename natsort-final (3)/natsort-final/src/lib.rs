//! natsort_core — natural ("human") sort ordering, ported from Python's
//! `natsort`.
//!
//! The core idea: split a string into alternating runs of text and numbers,
//! compare text runs lexicographically and number runs numerically. That
//! makes `"file2"` sort before `"file10"`, unlike a plain byte-wise compare.
//!
//! # Scope / honesty note
//! This is a working port, not a claim of full parity with upstream
//! `natsort`. Known gaps vs. the full Python library (see `DECISIONS.md`
//! for the reasoning behind each):
//! - Unicode decimal-digit *runs* support ASCII plus 26 Unicode
//!   decimal-digit (Nd) blocks (see [`DIGIT_BLOCK_STARTS`]) — a real
//!   expansion over ASCII-only, but still short of the ~65+ Nd blocks in
//!   the full Unicode Character Database (a handful of BMP blocks past
//!   Balinese, and every supplementary-plane/astral block, are not yet
//!   covered). Isolated digit characters (circled ①, superscript ²,
//!   subscript ₃) are handled separately — see [`isolated_digit_value`] —
//!   each as its own standalone single-digit number, never merged with a
//!   neighbor. Numeric *non-digits* (Roman numerals, vulgar fractions ½)
//!   are still not handled.
//! - Integers are parsed into `i128` for the common case; a number with
//!   more digits than fit in an `i128` (39+ decimal digits) falls back to
//!   [`Chunk::BigInt`], which preserves correct magnitude-based ordering
//!   without pulling in an arbitrary-precision arithmetic dependency. This
//!   replaces an earlier version of this port where overflow silently
//!   produced `0` — see `DECISIONS.md` #13.
//! - `GROUPLETTERS` and `LOWERCASEFIRST` are implemented as reasonable
//!   approximations of natsort's behavior, not verified character-for-
//!   character against upstream (no network access to install Python
//!   `natsort` and diff against it in the environment this port was
//!   written in).

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

/// One piece of a natsort key: a text run, a parsed number that fits in
/// `i128`, an arbitrary-magnitude integer that didn't, or a float.
///
/// # Why `BigInt` carries `(bool, Vec<u8>)` instead of a bignum crate
/// `bool` is the sign (`true` = negative) and `Vec<u8>` is the decimal
/// digits in ASCII (`b'0'..=b'9'`), most-significant first, with no
/// leading zero (except the impossible-in-practice case of magnitude
/// zero, which never reaches this variant — `0` always fits in `i128`).
/// Comparing two such values only needs digit-count then lexicographic
/// byte comparison — no arithmetic, no bignum dependency, and it keeps
/// the crate's zero-runtime-dependency property (`DECISIONS.md` #2, #7).
#[derive(Debug, Clone, PartialEq)]
pub enum Chunk {
    Text(String),
    Int(i128),
    BigInt(bool, Vec<u8>),
    Float(f64),
}

impl Eq for Chunk {}

impl PartialOrd for Chunk {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Convert a signed `i128` into `(is_negative, magnitude_digits)` form, so
/// it can be compared against a [`Chunk::BigInt`] using the same
/// magnitude-comparison logic.
fn int_to_signed_magnitude(v: i128) -> (bool, Vec<u8>) {
    (v < 0, v.unsigned_abs().to_string().into_bytes())
}

/// Compare two decimal-digit ASCII byte strings (no leading zeros, at
/// least one digit) purely by magnitude: a longer digit string is always
/// numerically larger, and equal-length strings compare byte-lexically
/// (equivalent to numeric order once lengths match and there's no leading
/// zero).
fn cmp_digit_magnitude(a: &[u8], b: &[u8]) -> Ordering {
    match a.len().cmp(&b.len()) {
        Ordering::Equal => a.cmp(b),
        other => other,
    }
}

/// Total order over signed `(sign, magnitude)` pairs — the shared
/// comparison logic behind `Chunk::Int`-vs-`Chunk::BigInt` and
/// `Chunk::BigInt`-vs-`Chunk::BigInt`.
fn signed_magnitude_cmp(neg_a: bool, mag_a: &[u8], neg_b: bool, mag_b: &[u8]) -> Ordering {
    match (neg_a, neg_b) {
        (false, true) => Ordering::Greater,
        (true, false) => Ordering::Less,
        (false, false) => cmp_digit_magnitude(mag_a, mag_b),
        // Both negative: the larger magnitude is the smaller (more
        // negative) number, so the magnitude order is reversed.
        (true, true) => cmp_digit_magnitude(mag_a, mag_b).reverse(),
    }
}

impl Ord for Chunk {
    fn cmp(&self, other: &Self) -> Ordering {
        use Chunk::*;
        match (self, other) {
            (Text(a), Text(b)) => a.cmp(b),
            (Int(a), Int(b)) => a.cmp(b),
            (Float(a), Float(b)) => a.total_cmp(b),
            (BigInt(neg_a, mag_a), BigInt(neg_b, mag_b)) => {
                signed_magnitude_cmp(*neg_a, mag_a, *neg_b, mag_b)
            }
            (Int(a), BigInt(neg_b, mag_b)) => {
                let (neg_a, mag_a) = int_to_signed_magnitude(*a);
                signed_magnitude_cmp(neg_a, &mag_a, *neg_b, mag_b)
            }
            (BigInt(neg_a, mag_a), Int(b)) => {
                let (neg_b, mag_b) = int_to_signed_magnitude(*b);
                signed_magnitude_cmp(*neg_a, mag_a, neg_b, &mag_b)
            }
            // Chunk sequences produced by `natsort_key` always alternate
            // Text/Number starting with Text, so a Text-vs-Number
            // comparison shouldn't occur when comparing two well-formed
            // keys of the same length at the same position. Keep the
            // order total anyway (e.g. for hand-built or differently-
            // shaped `Vec<Chunk>` values): Text always sorts before any
            // numeric chunk.
            (Text(_), _) => Ordering::Less,
            (_, Text(_)) => Ordering::Greater,
            // Cross Int/Float/BigInt without a Text partner likewise
            // shouldn't occur within one natsort_key call (float-ness is
            // fixed by `Ns` for the whole string), but stays total and
            // consistent with the existing Int-before-Float convention.
            (Int(_), Float(_)) => Ordering::Less,
            (Float(_), Int(_)) => Ordering::Greater,
            (BigInt(_, _), Float(_)) => Ordering::Less,
            (Float(_), BigInt(_, _)) => Ordering::Greater,
        }
    }
}

/// Contiguous Unicode decimal-digit (Nd) block start codepoints this crate
/// recognizes, each block being 10 consecutive codepoints (digit 0
/// through 9), plus the script name for documentation and testing.
///
/// Compiled from the Unicode "Decimal Number" (Nd) category listing
/// (<https://www.compart.com/en/unicode/category/Nd>, cross-referenced
/// against the Unicode Character Database) covering every Basic
/// Multilingual Plane Nd block through Balinese (U+1B50), plus Fullwidth
/// (U+FF10). ASCII is handled separately in [`digit_value`] rather than
/// listed here.
///
/// **This is not the complete Nd category** — flagged explicitly rather
/// than silently omitted, per this crate's policy against overstating
/// Unicode support (see the crate-level "Scope / honesty note"). Not yet
/// covered: a handful of BMP blocks past Balinese (Sundanese, Lepcha, Ol
/// Chiki, Vai, Saurashtra, Javanese, Cham, Meetei Mayek, and others), and
/// every supplementary-plane (astral) Nd block (Osmanya, Brahmi, Sora
/// Sompeng, Chakma, Adlam, Wancho, and others).
///
/// Kept sorted ascending by start codepoint — [`digit_value`] relies on
/// this for its binary search.
pub const DIGIT_BLOCK_STARTS: &[(u32, &str)] = &[
    (0x0660, "Arabic-Indic"),
    (0x06F0, "Extended Arabic-Indic"),
    (0x07C0, "NKo"),
    (0x0966, "Devanagari"),
    (0x09E6, "Bengali"),
    (0x0A66, "Gurmukhi"),
    (0x0AE6, "Gujarati"),
    (0x0B66, "Oriya"),
    (0x0BE6, "Tamil"),
    (0x0C66, "Telugu"),
    (0x0CE6, "Kannada"),
    (0x0D66, "Malayalam"),
    (0x0DE6, "Sinhala Lith"),
    (0x0E50, "Thai"),
    (0x0ED0, "Lao"),
    (0x0F20, "Tibetan"),
    (0x1040, "Myanmar"),
    (0x1090, "Myanmar Shan"),
    (0x17E0, "Khmer"),
    (0x1810, "Mongolian"),
    (0x1946, "Limbu"),
    (0x19D0, "New Tai Lue"),
    (0x1A80, "Tai Tham Hora"),
    (0x1A90, "Tai Tham Tham"),
    (0x1B50, "Balinese"),
    (0xFF10, "Fullwidth"),
];

/// Numeric value of `c` if it's an ASCII digit or a member of one of the
/// [`DIGIT_BLOCK_STARTS`] Unicode decimal-digit blocks, else `None`.
///
/// # Performance
/// Blocks are checked via binary search (`DIGIT_BLOCK_STARTS` is sorted
/// and non-overlapping — each block is 10 codepoints and the next entry's
/// start is always past the previous one's end), so this is `O(log n)` in
/// the number of registered blocks rather than a linear scan.
fn digit_value(c: char) -> Option<u32> {
    if c.is_ascii_digit() {
        return Some(c as u32 - '0' as u32);
    }
    let cp = c as u32;
    let idx = DIGIT_BLOCK_STARTS.partition_point(|&(start, _)| start <= cp);
    if idx == 0 {
        return None;
    }
    let (start, _name) = DIGIT_BLOCK_STARTS[idx - 1];
    if cp < start + 10 {
        Some(cp - start)
    } else {
        None
    }
}

/// Isolated Unicode digit characters that always form a **standalone
/// single-digit number**, never concatenated with a neighboring digit —
/// unlike the contiguous decimal-digit runs [`digit_value`] feeds into.
/// Covers superscript, subscript, and circled digit characters, e.g.
/// `"x²"` parses as `[Text("x"), Int(2)]` and `"①②"` parses as
/// `[Text(""), Int(1), Text(""), Int(2)]` (two separate one-digit numbers),
/// not `Int(12)`.
fn isolated_digit_value(c: char) -> Option<u32> {
    match c {
        '\u{2070}' => Some(0),                                   // ⁰ SUPERSCRIPT ZERO
        '\u{00B9}' => Some(1),                                   // ¹ SUPERSCRIPT ONE
        '\u{00B2}' => Some(2),                                   // ² SUPERSCRIPT TWO
        '\u{00B3}' => Some(3),                                   // ³ SUPERSCRIPT THREE
        '\u{2074}'..='\u{2079}' => Some(c as u32 - 0x2074 + 4),  // ⁴-⁹
        '\u{2080}'..='\u{2089}' => Some(c as u32 - 0x2080),      // ₀-₉ subscript
        '\u{2460}'..='\u{2468}' => Some(c as u32 - 0x2460 + 1),  // ①-⑨ circled
        '\u{24EA}' => Some(0),                                   // ⓪ CIRCLED DIGIT ZERO
        _ => None,
    }
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

/// Apply the `Ns` text transforms to a text chunk, taking ownership of the
/// already-collected chunk `text` rather than a borrow.
///
/// # Performance
/// When none of `lowercasefirst`/`groupletters`/`ignorecase` are set (the
/// common default case), this returns `text` unchanged with no
/// allocation, instead of unconditionally cloning it first.
fn apply_text_transform(text: String, ns: Ns) -> String {
    if !ns.lowercasefirst && !ns.groupletters && !ns.ignorecase {
        return text;
    }
    let mut s = text;
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
    if isolated_digit_value(chars[i]).is_some() {
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

/// Strip leading zero digits from an ASCII digit byte slice, always
/// leaving at least one digit (so `"000"` becomes `"0"`, not `""`).
fn strip_leading_zeros(digits: &[u8]) -> &[u8] {
    let mut i = 0;
    while i + 1 < digits.len() && digits[i] == b'0' {
        i += 1;
    }
    &digits[i..]
}

/// Parse a number run starting at `chars[i]`. Returns the chunk and the
/// index just past the consumed run.
fn parse_number(chars: &[char], i: usize, ns: Ns) -> (Chunk, usize) {
    let n = chars.len();

    // Isolated digit characters (circled, superscript, subscript) are
    // always a standalone single-digit number — consume exactly this one
    // character, never extending into a neighboring digit of any kind.
    if digit_value(chars[i]).is_none() {
        if let Some(d) = isolated_digit_value(chars[i]) {
            let chunk = if ns.float {
                Chunk::Float(d as f64)
            } else {
                Chunk::Int(d as i128)
            };
            return (chunk, i + 1);
        }
    }

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
        match raw.parse::<i128>() {
            Ok(value) => return (Chunk::Int(value), j),
            Err(_) => {
                // More digits than fit in an i128 (39+ decimal digits).
                // Fall back to a magnitude-preserving BigInt chunk instead
                // of silently comparing as 0 — see DECISIONS.md #13.
                let neg = raw.starts_with('-');
                let digits_start = usize::from(neg || raw.starts_with('+'));
                let digits = strip_leading_zeros(raw[digits_start..].as_bytes()).to_vec();
                return (Chunk::BigInt(neg, digits), j);
            }
        }
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

    // A number this extreme (e.g. `1e400`) overflows `f64` to +/-infinity,
    // or underflows to 0.0 for extreme negative exponents — the same
    // saturating behavior Python's own `float()` has, not a divergence
    // introduced by this port.
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
        chunks.push(Chunk::Text(apply_text_transform(text, ns)));

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

    // --- Isolated Unicode digit characters ---

    #[test]
    fn circled_digits_are_separate_single_digit_numbers() {
        // Adjacent circled digits do NOT concatenate into a multi-digit
        // number, unlike adjacent ASCII digits.
        let key = natsort_key("\u{2460}\u{2461}", Ns::DEFAULT); // "①②"
        assert_eq!(
            key,
            vec![
                Chunk::Text(String::new()),
                Chunk::Int(1),
                Chunk::Text(String::new()),
                Chunk::Int(2),
            ]
        );
    }

    #[test]
    fn superscript_digit_attaches_to_preceding_text() {
        let key = natsort_key("x\u{00B2}", Ns::DEFAULT); // "x²"
        assert_eq!(key, vec![Chunk::Text("x".to_string()), Chunk::Int(2)]);
    }

    #[test]
    fn subscript_digit_sorts_numerically_against_plain_digit() {
        // "x₂" (isolated 2) should sort before "x10" (plain 10).
        let input = v(&["x10", "x\u{2082}", "x1"]);
        assert_eq!(natsorted(&input), v(&["x1", "x\u{2082}", "x10"]));
    }

    #[test]
    fn circled_digit_zero_is_recognized() {
        let key = natsort_key("\u{24EA}", Ns::DEFAULT); // "⓪"
        assert_eq!(key, vec![Chunk::Text(String::new()), Chunk::Int(0)]);
    }

    #[test]
    fn isolated_digit_respects_float_flag() {
        let key = natsort_key("\u{00B9}", Ns::REAL); // "¹" under REAL
        assert_eq!(key, vec![Chunk::Text(String::new()), Chunk::Float(1.0)]);
    }

    // --- New Unicode digit blocks (expanded from 6 to 26) ---

    #[test]
    fn every_registered_digit_block_maps_its_full_0_to_9_range() {
        for &(start, name) in DIGIT_BLOCK_STARTS {
            for offset in 0..10u32 {
                let c = char::from_u32(start + offset).unwrap();
                assert_eq!(
                    digit_value(c),
                    Some(offset),
                    "block {name} (U+{start:04X}) offset {offset} did not round-trip"
                );
            }
            // The codepoint immediately past this block's last digit must
            // not itself be read as digit 9 of this block.
            if let Some(past) = char::from_u32(start + 10) {
                assert_ne!(
                    digit_value(past),
                    Some(9),
                    "block {name} boundary leaked into the next codepoint"
                );
            }
        }
    }

    // --- BigInt overflow fallback (fixes the earlier "overflow silently
    // becomes 0" gap — see DECISIONS.md #13) ---

    #[test]
    fn signed_magnitude_cmp_matches_intuitive_integer_order() {
        assert_eq!(signed_magnitude_cmp(false, b"5", false, b"10"), Ordering::Less);
        assert_eq!(signed_magnitude_cmp(true, b"5", true, b"10"), Ordering::Greater);
        assert_eq!(signed_magnitude_cmp(true, b"5", false, b"1"), Ordering::Less);
        assert_eq!(signed_magnitude_cmp(false, b"5", false, b"5"), Ordering::Equal);
    }

    #[test]
    fn strip_leading_zeros_keeps_at_least_one_digit() {
        assert_eq!(strip_leading_zeros(b"000"), b"0");
        assert_eq!(strip_leading_zeros(b"007"), b"7");
        assert_eq!(strip_leading_zeros(b"700"), b"700");
        assert_eq!(strip_leading_zeros(b"0"), b"0");
    }

    #[test]
    fn overflowing_integer_becomes_bigint_not_zero() {
        let huge = "9".repeat(45); // far past i128::MAX (39 digits)
        let key = natsort_key(&huge, Ns::DEFAULT);
        assert_eq!(
            key,
            vec![Chunk::Text(String::new()), Chunk::BigInt(false, huge.into_bytes())]
        );
    }

    #[test]
    fn bigint_orders_above_every_in_range_int() {
        let small = natsort_key("999999999999999999999999999999999999", Ns::DEFAULT); // 38 nines
        let big = natsort_key(&"9".repeat(45), Ns::DEFAULT);
        assert!(small < big);
    }
}
