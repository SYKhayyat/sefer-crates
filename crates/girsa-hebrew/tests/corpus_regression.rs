//! The regression corpus, harvested from the seforim by `examples/harvest.rs`.
//!
//! Every row is two strings that genuinely occur in the corpus and are
//! genuinely the same word written two ways. Nothing here was invented, and
//! nothing here was produced by asking the normalizer what it currently
//! returns — a fixture generated that way passes on the day it is written and
//! keeps passing through any bug introduced consistently.
//!
//! Regenerate with:
//!
//! ```sh
//! cargo run --release -p girsa-hebrew --example harvest -- \
//!     "$OTZARIA/אוצריא" crates/girsa-hebrew/fixtures/corpus-regression.tsv
//! ```

use girsa_hebrew::{normalize, variants, VariantKind};

const FIXTURE: &str = include_str!("../fixtures/corpus-regression.tsv");

struct Case<'a> {
    kind: &'a str,
    left: &'a str,
    right: &'a str,
    left_from: &'a str,
    right_from: &'a str,
}

fn cases() -> Vec<Case<'static>> {
    FIXTURE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|line| {
            let mut f = line.split('\t');
            Some(Case {
                kind: f.next()?,
                left: f.next()?,
                right: f.next()?,
                left_from: f.next().unwrap_or("?"),
                right_from: f.next().unwrap_or("?"),
            })
        })
        .collect()
}

fn of_kind(kind: &str) -> Vec<Case<'static>> {
    cases().into_iter().filter(|c| c.kind == kind).collect()
}

/// Reports every failing row rather than the first, because one broken rule
/// usually breaks a whole class and the count is the diagnosis.
fn assert_all(kind: &str, mut check: impl FnMut(&Case<'_>) -> Result<(), String>) {
    let rows = of_kind(kind);
    assert!(!rows.is_empty(), "the fixture has no {kind} rows");

    let failures: Vec<String> = rows.iter().filter_map(|c| check(c).err()).collect();
    assert!(
        failures.is_empty(),
        "{} of {} {kind} rows failed:\n{}",
        failures.len(),
        rows.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_fixture_is_large_enough_to_mean_something() {
    // W2 asks for 200 rows spanning the transformations. Below that the suite
    // stops being a regression corpus and becomes a handful of examples.
    assert!(
        cases().len() >= 200,
        "only {} rows — regenerate from a wider sample",
        cases().len()
    );
}

#[test]
fn a_word_written_with_nikud_and_without_it_is_one_word() {
    // Berakhot ships fully menukad; Mishnah Berurah has none. The same word is
    // two different strings depending on which sefer you happen to be in, and
    // spec.md §9.1 says the reader never has to care.
    assert_all("nikud", |c| {
        let (l, r) = (normalize(c.left), normalize(c.right));
        if l == r {
            Ok(())
        } else {
            Err(format!(
                "{} ({}) -> {l:?} but {} ({}) -> {r:?}",
                c.left, c.left_from, c.right, c.right_from
            ))
        }
    });
}

#[test]
fn the_two_ways_of_writing_gershayim_are_one_mark() {
    // The fold that looks done and isn't: one sefer writes U+05F4, another an
    // ASCII quote, and an abbreviation found in the first is invisible in the
    // second unless these come together.
    assert_all("gershayim", |c| {
        let (l, r) = (normalize(c.left), normalize(c.right));
        if l == r {
            Ok(())
        } else {
            Err(format!(
                "{} ({}) -> {l:?} but {} ({}) -> {r:?}",
                c.left, c.left_from, c.right, c.right_from
            ))
        }
    });
}

#[test]
fn a_prefixed_word_offers_the_stem_that_is_also_in_the_corpus() {
    assert_all("prefix", |c| {
        let (long, stem) = (normalize(c.left), normalize(c.right));
        if variants(&long)
            .forms_of_kind(VariantKind::PrefixPeeled)
            .any(|f| f == stem)
        {
            Ok(())
        } else {
            Err(format!(
                "{long:?} ({}) does not offer {stem:?} ({}); offers {:?}",
                c.left_from,
                c.right_from,
                variants(&long)
                    .forms_of_kind(VariantKind::PrefixPeeled)
                    .collect::<Vec<_>>()
            ))
        }
    });
}

#[test]
fn a_male_spelling_offers_the_chaser_one_that_is_also_in_the_corpus() {
    assert_all("ktiv", |c| {
        let (male, chaser) = (normalize(c.left), normalize(c.right));
        if variants(&male)
            .forms_of_kind(VariantKind::KtivSwapped)
            .any(|f| f == chaser)
        {
            Ok(())
        } else {
            Err(format!(
                "{male:?} ({}) does not offer {chaser:?} ({})",
                c.left_from, c.right_from
            ))
        }
    });
}

#[test]
fn every_attested_table_entry_expands_both_ways() {
    // These rows are the table checked against reality: both sides were found
    // in real seforim before the row was written.
    assert_all("abbrev", |c| {
        let forward = variants(c.left)
            .forms_of_kind(VariantKind::AbbreviationExpanded)
            .any(|f| normalize(f) == normalize(c.right));
        let backward = variants(c.right)
            .forms_of_kind(VariantKind::AbbreviationExpanded)
            .any(|f| normalize(f) == normalize(c.left));
        match (forward, backward) {
            (true, true) => Ok(()),
            (true, false) => Err(format!("{} -> {} but not back", c.left, c.right)),
            (false, true) => Err(format!("{} -> {} but not forward", c.right, c.left)),
            (false, false) => Err(format!("{} and {} do not meet", c.left, c.right)),
        }
    });
}

// ---------------------------------------------------------------------------
// Coverage — the fixture has to exercise all seven transformations, not four
// ---------------------------------------------------------------------------

#[test]
fn the_fixture_exercises_final_letter_folding() {
    // Not a harvested kind of its own, because a final letter is not an
    // alternative spelling — it is positional. It has to be *exercised*
    // nonetheless, or the fold could be deleted and every test would still pass.
    let touched = cases()
        .iter()
        .filter(|c| c.left.chars().chain(c.right.chars()).any(is_final_form))
        .count();
    assert!(
        touched >= 20,
        "only {touched} rows carry a final letter; the fold is barely tested"
    );
}

#[test]
fn the_fixture_exercises_teamim_and_not_only_nikud() {
    // Te'amim occupy U+0591–U+05AF and appear only in Tanakh and menukad
    // Talmud. A fixture drawn from halacha alone would never see one, and the
    // range could silently narrow to the nikud block.
    let touched = cases()
        .iter()
        .filter(|c| {
            c.left
                .chars()
                .chain(c.right.chars())
                .any(|ch| ('\u{0591}'..='\u{05AF}').contains(&ch))
        })
        .count();
    assert!(touched >= 1, "no row carries a ta'am; te'amim are untested");
}

#[test]
fn every_row_is_a_pair_of_different_strings() {
    // A row where both sides are identical asserts nothing and would quietly
    // pad the count.
    for c in cases() {
        assert_ne!(c.left, c.right, "{} row is the same string twice", c.kind);
    }
}

fn is_final_form(c: char) -> bool {
    matches!(c, 'ך' | 'ם' | 'ן' | 'ף' | 'ץ')
}
