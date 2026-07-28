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
pub fn normalize_into(input: &str, out: &mut String) {
    out.clear();
    let mut pending_space = false;

    for c in input.chars() {
        if is_mark(c) {
            continue;
        }

        let kept = if is_word_breaking_punctuation(c) || c.is_whitespace() {
            None
        } else if let Some(folded) = fold_quote_mark(c) {
            Some(folded)
        } else if crate::marks::is_hebrew_letter(c) {
            Some(fold_final(c))
        } else if c.is_ascii_alphanumeric() {
            Some(c.to_ascii_lowercase())
        } else {
            None
        };

        match kept {
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

/// Split text into normalized words, keeping each one's span in the input.
#[must_use]
pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut start = 0usize;

    for (offset, c) in input.char_indices() {
        if is_mark(c) {
            continue;
        }

        let kept = if is_word_breaking_punctuation(c) || c.is_whitespace() {
            None
        } else if let Some(folded) = fold_quote_mark(c) {
            Some(folded)
        } else if crate::marks::is_hebrew_letter(c) {
            Some(fold_final(c))
        } else if c.is_ascii_alphanumeric() {
            Some(c.to_ascii_lowercase())
        } else {
            None
        };

        match kept {
            Some(k) => {
                if current.is_empty() {
                    start = offset;
                }
                current.push(k);
            }
            None => {
                if !current.is_empty() {
                    tokens.push(Token {
                        text: core::mem::take(&mut current),
                        start,
                        end: offset,
                    });
                }
            }
        }
    }

    if !current.is_empty() {
        tokens.push(Token {
            text: current,
            start,
            end: input.len(),
        });
    }

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
        // Two code paths, one answer. If they drift, a highlight lands on text
        // that was never matched.
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
    fn html_angle_brackets_do_not_glue_a_heading_to_its_text() {
        // The importer strips tags (W7); the normalizer must not make things
        // worse in the meantime.
        assert_eq!(normalize("<h2>דף ב.</h2>"), "h2 דפ ב h2");
    }
}
