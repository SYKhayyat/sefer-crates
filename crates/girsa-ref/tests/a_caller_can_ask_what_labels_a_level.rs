//! The list of words that *label* a level is asked for, not copied.
//!
//! # Why this is public at all
//!
//! A caller reading citations out of a file — an importer looking at
//! `שו"ת מהר"י בן לב חלק א סימן א` and deciding whether that line opens a
//! section — has to make exactly the distinction this crate makes internally:
//! is `חלק` a word that labels the number after it, or is it part of a name?
//!
//! Answering that with a second list is the failure mode `SECTION_WORD_SET`
//! already has a comment about. When the internal list drifted from its
//! normalized copy, `סעיף` fell through to the numeral reader and became 220,
//! so `שוע אוח סימן א סעיף א` resolved to `160:1:220:1` — four levels, all
//! wrong, and it *resolved*. A copy of the list in another repository is the
//! same bug with a longer fuse: nothing would fail here when a word is added
//! there, and nothing would fail there when a word is added here.
//!
//! So the list is asked for. This test pins that it can be.

use girsa_ref::resolve::is_section_word;

#[test]
fn the_words_a_schema_labels_a_level_with_are_answered_for() {
    // The ones an importer meets first, in the order it meets them.
    for word in ["חלק", "סימן", "כלל", "שער", "פרק", "סעיף", "דף", "עמוד"]
    {
        assert!(is_section_word(word), "{word} labels a level");
    }
}

#[test]
fn a_word_that_names_a_sefer_is_not_a_label() {
    // The five deliberately absent, and the reason is in the const's own
    // comment: a title is matched before an address, and skipping these turns
    // the rest of a name into nothing.
    for word in ["תורה", "תלמוד", "ספר", "תפילה", "מדרש"] {
        assert!(!is_section_word(word), "{word} is how a sefer is called");
    }
    // And ordinary words, which is nearly everything.
    for word in ["ברכות", "אמת", "משה", "הרשבא"] {
        assert!(!is_section_word(word), "{word} is not a label");
    }
}

#[test]
fn the_answer_is_the_normalized_one_so_a_final_letter_does_not_miss() {
    // `סעיף` normalizes to `סעיפ`. Asking with either spelling has to give the
    // same answer, or a caller normalizing first and a caller not normalizing
    // get different libraries.
    assert!(is_section_word("סעיף"));
    assert!(is_section_word("סעיפ"));
    // Nikud is not a different word either.
    assert!(is_section_word("סִימָן"));
}
