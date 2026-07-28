//! Which characters are marks, which are letters, and which are punctuation.
//!
//! spec.md §9.1 says nikud and te'amim are stripped in every mode, no toggle,
//! and gives the range `U+0591–U+05C7`. That range is not all marks, and the
//! difference matters:
//!
//! | Code point | What it is | What we do |
//! |---|---|---|
//! | `U+05BE` ־ maqaf | joins two words, like a hyphen | **space** |
//! | `U+05C0` ׀ paseq | a divider between words | **space** |
//! | `U+05C3` ׃ sof pasuq | ends a verse, like a full stop | **space** |
//! | `U+05C6` ׆ nun hafukha | a scribal bracket around a passage | **space** |
//! | everything else in range | combining marks | **deleted** |
//!
//! Deleting maqaf instead of replacing it would turn `אֶת־הַשָּׁמַיִם` into the single
//! token `אתהשמים`, and searching for `השמים` would then not find the second
//! verse of the Torah. That is the exact failure mode §9.2 exists to prevent, so
//! the four punctuation marks break words rather than vanishing.

/// The block Hebrew combining marks live in, per spec.md §9.1.
const MARK_BLOCK: core::ops::RangeInclusive<char> = '\u{0591}'..='\u{05C7}';

/// Hebrew punctuation that sits inside the mark block but separates words.
const WORD_BREAKING: [char; 4] = [
    '\u{05BE}', // ־ MAQAF
    '\u{05C0}', // ׀ PASEQ
    '\u{05C3}', // ׃ SOF PASUQ
    '\u{05C6}', // ׆ NUN HAFUKHA
];

/// A nikud point, te'amim accent, or other Hebrew combining mark — something
/// that is stripped because nobody types it.
#[must_use]
pub fn is_mark(c: char) -> bool {
    MARK_BLOCK.contains(&c) && !WORD_BREAKING.contains(&c)
}

/// Hebrew punctuation that separates two words rather than decorating one.
#[must_use]
pub fn is_word_breaking_punctuation(c: char) -> bool {
    WORD_BREAKING.contains(&c)
}

/// A Hebrew consonant, `א` through `ת`, including the five final forms.
///
/// Deliberately excludes the Yiddish ligatures `װ ױ ײ` (`U+05F0`–`U+05F2`) and
/// the geresh/gershayim punctuation above them, which are handled separately.
#[must_use]
pub fn is_hebrew_letter(c: char) -> bool {
    ('\u{05D0}'..='\u{05EA}').contains(&c)
}

/// Final letters, paired with the medial form they fold to.
///
/// Which form a letter takes is decided by where it sits in the word, not by
/// what it means, so folding them costs nothing and buys a great deal: it is
/// what lets `מלך` be found inside `מלכים`, and what keeps a peeled prefix from
/// leaving a final letter stranded in the middle of a stem.
pub(crate) const FINAL_FORMS: [(char, char); 5] = [
    ('\u{05DA}', '\u{05DB}'), // ך → כ
    ('\u{05DD}', '\u{05DE}'), // ם → מ
    ('\u{05DF}', '\u{05E0}'), // ן → נ
    ('\u{05E3}', '\u{05E4}'), // ף → פ
    ('\u{05E5}', '\u{05E6}'), // ץ → צ
];

/// Fold a final letter to its medial form. Any other character is returned
/// unchanged.
#[must_use]
pub(crate) fn fold_final(c: char) -> char {
    match FINAL_FORMS.iter().find(|(final_form, _)| *final_form == c) {
        Some((_, medial)) => *medial,
        None => c,
    }
}

/// Every character the corpus uses for a geresh — the mark on `גְּמָ׳` and `ה'`.
///
/// There are this many because the corpus genuinely uses them all: Berakhot
/// writes `גְּמָ׳` with `U+05F3`, while Mishnah Berurah writes `ה'` with an ASCII
/// apostrophe, and text pasted from a word processor arrives with curly quotes.
/// They are the same mark and must fold together, or an abbreviation found in
/// one sefer is invisible in another.
const GERESH_FORMS: [char; 4] = [
    '\u{05F3}', // ׳ HEBREW PUNCTUATION GERESH
    '\'',       // ' APOSTROPHE
    '\u{2018}', // ' LEFT SINGLE QUOTATION MARK
    '\u{2019}', // ' RIGHT SINGLE QUOTATION MARK
];

/// Every character the corpus uses for gershayim — the mark inside `שו"ע`.
const GERSHAYIM_FORMS: [char; 4] = [
    '\u{05F4}', // ״ HEBREW PUNCTUATION GERSHAYIM
    '"',        // " QUOTATION MARK
    '\u{201C}', // " LEFT DOUBLE QUOTATION MARK
    '\u{201D}', // " RIGHT DOUBLE QUOTATION MARK
];

/// The single character every geresh folds to.
pub(crate) const CANONICAL_GERESH: char = '\'';
/// The single character every gershayim folds to.
pub(crate) const CANONICAL_GERSHAYIM: char = '"';

/// Fold any of the geresh or gershayim spellings to its canonical character.
///
/// They are *folded*, not removed. The mark says "this is an abbreviation", and
/// that is worth keeping: it is what tells [`crate::variants`] to look `שו"ע` up
/// in the abbreviation table. Removing it is offered separately, as
/// [`crate::VariantKind::GershayimDropped`].
#[must_use]
pub(crate) fn fold_quote_mark(c: char) -> Option<char> {
    if GERESH_FORMS.contains(&c) {
        Some(CANONICAL_GERESH)
    } else if GERSHAYIM_FORMS.contains(&c) {
        Some(CANONICAL_GERSHAYIM)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_punctuation_marks_in_the_range_are_not_treated_as_marks() {
        for c in WORD_BREAKING {
            assert!(
                !is_mark(c),
                "{c:?} (U+{:04X}) must not be stripped",
                c as u32
            );
            assert!(is_word_breaking_punctuation(c));
        }
    }

    #[test]
    fn nikud_and_teamim_are_marks() {
        for c in ['\u{05B0}', '\u{05B7}', '\u{05BC}', '\u{0591}', '\u{05C7}'] {
            assert!(is_mark(c), "U+{:04X} must be stripped", c as u32);
        }
    }

    #[test]
    fn every_spelling_of_a_quote_mark_folds_to_one_character() {
        for c in GERESH_FORMS {
            assert_eq!(fold_quote_mark(c), Some(CANONICAL_GERESH));
        }
        for c in GERSHAYIM_FORMS {
            assert_eq!(fold_quote_mark(c), Some(CANONICAL_GERSHAYIM));
        }
        assert_eq!(fold_quote_mark('א'), None);
    }

    #[test]
    fn final_letters_fold_and_ordinary_letters_do_not() {
        assert_eq!(fold_final('ך'), 'כ');
        assert_eq!(fold_final('ץ'), 'צ');
        assert_eq!(fold_final('א'), 'א');
    }
}
