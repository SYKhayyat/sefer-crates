//! The normal form: what goes in the index, and what a query is compared to.
//!
//! This is deliberately the *smallest* transformation that lets two spellings of
//! the same thing compare equal. It removes marks nobody types and folds
//! characters that are one character written two ways — and it stops there.
//!
//! Everything that could destroy a word (peeling `מ` off `משה`, deleting the `ו`
//! from `שור`) lives in [`crate::variants`] instead, because spec.md §9.3 makes
//! the literal mode the default and W12 requires that *nothing beyond
//! nikud-stripping* be applied in it. If the index held peeled stems, there
//! would be no literal mode to default to.

use crate::marks::{fold_final, fold_quote_mark, is_mark, is_word_breaking_punctuation};

/// One word of normalized text, with where it came from in the original.
///
/// The span is into the **input**, not the normalized output, because that is
/// what a highlight needs: the reader is looking at the page as printed, with
/// its nikud, and the match has to land on the right characters of *that*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The normalized word.
    pub text: String,
    /// Byte offset of the token's first character in the input string.
    pub start: usize,
    /// Byte offset one past the token's last character in the input string.
    pub end: usize,
}

/// Reduce a string to its normal form.
///
/// - Nikud and te'amim are removed (spec.md §9.1 — every mode, no toggle).
/// - Maqaf, paseq, sof pasuq and nun hafukha become spaces, because they
///   separate words rather than decorating one.
/// - Final letters fold to their medial form: `ך`→`כ`, `ם`→`מ`, …
/// - Every spelling of geresh and gershayim folds to `'` and `"`.
/// - ASCII letters lowercase, so `Berakhot 2a` and `berakhot 2a` agree.
/// - Anything else — punctuation, HTML angle brackets, em-dashes — becomes a
///   space, and runs of whitespace collapse to one.
///
/// Idempotent, and safe on text that is already bare: nikud coverage across the
/// corpus is inconsistent (Berakhot is fully menukad, Mishnah Berurah has none),
/// so this function meets already-normalized text constantly.
#[must_use]
pub fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    normalize_into(input, &mut out);
    out
}

/// [`normalize`], reusing a caller-owned buffer.
///
/// Ingest normalizes several million segments; allocating a `String` per
/// segment is the difference between an import that finishes over lunch and one
/// that does not.
/// What one character becomes, or `None` if it is a word break.
///
/// `is_mark` is checked by the callers rather than here, because a mark is
/// neither kept nor a break — it vanishes, and a `None` from this function means
/// *break the word*. Folding a te'am into a break would split `מֵאֵימָתַי` into
/// five words.
///
/// The three walkers below all classify characters this way, and until this
/// function existed they each did it with their own copy of the same
/// if-else ladder. The test named `tokenizing_agrees_with_normalizing` guarded
/// that agreement, which was the right instinct and the wrong shape: a test that
/// two copies still say the same thing is a worse tool than one copy.
fn folded(c: char) -> Option<char> {
    if is_word_breaking_punctuation(c) || c.is_whitespace() {
        None
    } else if let Some(folded) = fold_quote_mark(c) {
        Some(folded)
    } else if crate::marks::is_hebrew_letter(c) {
        Some(fold_final(c))
    } else if c.is_ascii_alphanumeric() {
        Some(c.to_ascii_lowercase())
    } else {
        None
    }
}

pub fn normalize_into(input: &str, out: &mut String) {
    out.clear();
    let mut pending_space = false;

    for c in input.chars() {
        if is_mark(c) {
            continue;
        }

        match folded(c) {
            Some(c) => {
                if pending_space && !out.is_empty() {
                    out.push(' ');
                }
                pending_space = false;
                out.push(c);
            }
            None => pending_space = true,
        }
    }
}

/// Walk the normalized words of `input`, without allocating one per word.
///
/// Every word is handed to `f` as `(normalized text, start, end)`, where the
/// span is into the **input as printed** — the menukad word on the page, not the
/// bare form `f` is shown. The `&str` borrows a buffer this function reuses, so
/// it is only valid for the call; a caller that wants to keep the words wants
/// [`tokenize`], which is this function plus a `to_string`.
///
/// This exists because the caller that dominates the cost does not want the
/// words at all. `SearchIndex::marks` tokenizes a hit's text to find *where* the
/// matched words sit and then throws every string away, keeping the spans — and
/// a hit can be a 10,000-character segment, so that is thousands of allocations
/// per row, per keystroke of a live search, all discarded.
pub fn for_each_token(input: &str, mut f: impl FnMut(&str, usize, usize)) {
    let mut current = String::new();
    let mut start = 0usize;

    for (offset, c) in input.char_indices() {
        if is_mark(c) {
            continue;
        }

        match folded(c) {
            Some(k) => {
                if current.is_empty() {
                    start = offset;
                }
                current.push(k);
            }
            None => {
                if !current.is_empty() {
                    f(&current, start, offset);
                    current.clear();
                }
            }
        }
    }

    if !current.is_empty() {
        f(&current, start, input.len());
    }
}

/// Split text into normalized words, keeping each one's span in the input.
#[must_use]
pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    for_each_token(input, |text, start, end| {
        tokens.push(Token {
            text: text.to_string(),
            start,
            end,
        });
    });
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_point_back_at_the_text_as_printed() {
        let input = "מֵאֵימָתַי קוֹרִין";
        let tokens = tokenize(input);
        assert_eq!(tokens.len(), 2);
        // The span must cover the menukad word on the page, not the bare one.
        assert_eq!(&input[tokens[0].start..tokens[0].end], "מֵאֵימָתַי");
        assert_eq!(tokens[0].text, "מאימתי");
    }

    #[test]
    fn tokenizing_agrees_with_normalizing() {
        // Two walkers, one classifier. They cannot drift any more — `folded`
        // is the only thing either of them asks — but they still assemble what
        // it says differently (one string versus spans), and *that* is what
        // this checks. If they disagreed, a highlight would land on text that
        // was never matched.
        for input in [
            "בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ׃",
            "אֶת־הַשָּׁמַיִם",
            "וכל כונת הש\"י הוא רק לטובתנו",
            "",
            "   ",
        ] {
            let joined = tokenize(input)
                .into_iter()
                .map(|t| t.text)
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(joined, normalize(input), "disagreement on {input:?}");
        }
    }

    #[test]
    fn walking_the_tokens_says_what_collecting_them_says() {
        // `tokenize` is `for_each_token` plus a `to_string`, so this cannot
        // fail without one of them being rewritten — which is the point of
        // pinning it: the next person to optimize the walker has a fence.
        for input in ["מֵאֵימָתַי קוֹרִין אֶת־שְׁמַע", "Berakhot 2a — קוֹרִין", "  ", "א"]
        {
            let mut walked = Vec::new();
            for_each_token(input, |t, s, e| walked.push((t.to_string(), s, e)));
            let collected: Vec<_> = tokenize(input)
                .into_iter()
                .map(|t| (t.text, t.start, t.end))
                .collect();
            assert_eq!(walked, collected, "disagreement on {input:?}");
        }
    }

    #[test]
    fn the_word_buffer_is_one_buffer() {
        // What the whole change is for: the words are handed out of a buffer
        // that is reused, so a caller that only wants spans allocates nothing
        // per word. Observable as the pointer staying put once the buffer has
        // grown past the longest word — which it has by the third of these.
        let text = "אחד שנים שלושה ארבעה חמישה שישה שבעה שמונה תשעה עשרה";
        let mut seen = Vec::new();
        for_each_token(text, |t, _, _| seen.push(t.as_ptr() as usize));
        let tail = &seen[3..];
        assert!(
            tail.iter().all(|p| *p == tail[0]),
            "the buffer moved: {seen:?}"
        );
    }

    #[test]
    fn html_angle_brackets_do_not_glue_a_heading_to_its_text() {
        // The importer strips tags (W7); the normalizer must not make things
        // worse in the meantime.
        assert_eq!(normalize("<h2>דף ב.</h2>"), "h2 דפ ב h2");
    }
}
