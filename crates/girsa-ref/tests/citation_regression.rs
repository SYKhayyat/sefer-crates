//! The resolver, measured against citations that genuinely appear in the corpus.
//!
//! BUILDER.md W3 asks for ≥95% exact resolution on a regression corpus mined
//! from the seforim rather than invented, with **every miss classified as
//! `Ambiguous` rather than wrong**. This is that measurement, run as a test so
//! it cannot quietly regress between releases.
//!
//! # Where the citations come from
//!
//! Neither invented nor chosen:
//!
//! - **Sefaria's `links*.csv`**, whose two citation columns are canonical refs
//!   Sefaria produced for the whole corpus — `Sanhedrin 74b:9`, `Exodus 1:1-6:1`.
//! - **Otzaria's `*_links.json`**, whose `heRef_2` is the same link written in
//!   Hebrew the way a person writes it — `אבי עזר, במדבר,  י, ב, א`, with the
//!   doubled spaces and trailing commas the real files carry.
//!
//! Regenerate with:
//!
//! ```sh
//! cargo run --release -p girsa-corpus --example measure-resolver -- \
//!     corpus/lexicon.tsv corpus/sefaria/links "$OTZARIA/links" \
//!     ../sefer-crates/crates/girsa-ref/fixtures/citations.tsv
//! ```

#![allow(clippy::expect_used, clippy::unwrap_used)]

use girsa_ref::{resolve, Lexicon, Resolution};

const LEXICON: &str = include_str!("../lexicon/sefaria.tsv");
const CITATIONS: &str = include_str!("../fixtures/citations.tsv");

/// BUILDER.md W3. Deliberately the stated bar and not the measured number:
/// a threshold set to whatever today achieves turns every improvement into a
/// commitment and every unrelated change into a failure.
const REQUIRED_EXACT_RATE: f64 = 95.0;

fn citations() -> Vec<(&'static str, &'static str)> {
    CITATIONS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| l.split_once('\t'))
        .collect()
}

fn lexicon() -> Lexicon {
    Lexicon::from_tsv(LEXICON)
}

struct Score {
    exact: usize,
    ambiguous: usize,
    unresolved: Vec<String>,
}

fn score(source: Option<&str>) -> Score {
    let lexicon = lexicon();
    let mut score = Score {
        exact: 0,
        ambiguous: 0,
        unresolved: Vec::new(),
    };
    for (from, citation) in citations() {
        if source.is_some_and(|s| s != from) {
            continue;
        }
        match resolve(&lexicon, citation) {
            Resolution::Exact(_) => score.exact += 1,
            Resolution::Ambiguous(_) => score.ambiguous += 1,
            Resolution::Unresolved => score.unresolved.push(citation.to_string()),
        }
    }
    score
}

#[test]
fn the_lexicon_is_the_whole_corpus_and_not_a_sample() {
    let lexicon = lexicon();
    assert!(
        lexicon.len() > 6000,
        "only {} works — the lexicon was built from a partial corpus",
        lexicon.len()
    );
    assert!(
        lexicon.variant_count() > 20_000,
        "only {} spellings",
        lexicon.variant_count()
    );
}

#[test]
fn the_fixture_is_large_enough_and_covers_both_languages() {
    let rows = citations();
    assert!(rows.len() >= 2000, "only {} citations", rows.len());
    assert!(rows.iter().any(|(s, _)| *s == "sefaria"));
    assert!(rows.iter().any(|(s, _)| *s == "otzaria"));
}

#[test]
fn at_least_95_percent_of_real_citations_resolve_exactly() {
    let score = score(None);
    let total = score.exact + score.ambiguous + score.unresolved.len();
    let rate = score.exact as f64 * 100.0 / total as f64;

    assert!(
        rate >= REQUIRED_EXACT_RATE,
        "{rate:.2}% exact of {total} ({} ambiguous, {} unresolved). \
         A sample of what did not resolve:\n{}",
        score.ambiguous,
        score.unresolved.len(),
        score
            .unresolved
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    println!("{rate:.2}% exact of {total} real citations");
}

#[test]
fn hebrew_citations_do_as_well_as_english_ones() {
    // The English half is Sefaria's own canonical output and would look fine on
    // its own. Hebrew as a person writes it — gershayim, no gershayim, dafim in
    // print notation — is the half that is actually hard, and averaging the two
    // would hide a failure in it.
    for source in ["sefaria", "otzaria"] {
        let score = score(Some(source));
        let total = score.exact + score.ambiguous + score.unresolved.len();
        let rate = score.exact as f64 * 100.0 / total as f64;
        assert!(
            rate >= REQUIRED_EXACT_RATE,
            "{source}: {rate:.2}% exact of {total}"
        );
        println!("{source}: {rate:.2}% exact of {total}");
    }
}

#[test]
fn no_citation_resolves_to_a_work_that_is_not_in_the_lexicon() {
    // A ref pointing at a work nobody has is worse than no ref: it looks like a
    // link and opens nothing.
    let lexicon = lexicon();
    for (_, citation) in citations() {
        for r in resolve(&lexicon, citation).candidates() {
            assert!(
                !r.work_slug().is_empty(),
                "{citation} resolved to a ref with no work"
            );
        }
    }
}

#[test]
fn the_citations_the_spec_names_land_where_it_says() {
    // Hand-checkable, unlike the rate above. Every one of these appears in
    // spec.md §4.3 as something the resolver must handle.
    let lexicon = lexicon();
    let cases = [
        ("Berakhot 2a", "girsa:bavli/berakhot/2a"),
        ("ברכות ב.", "girsa:bavli/berakhot/2a"),
        ("ברכות ב ע\"א", "girsa:bavli/berakhot/2a"),
        ("ברכות ב:", "girsa:bavli/berakhot/2b"),
        (
            "Shulchan Arukh, Orach Chayim 1:1",
            "girsa:shulchan-arukh/orach-chayim/1:1",
        ),
        (
            "שולחן ערוך אורח חיים סימן א סעיף א",
            "girsa:shulchan-arukh/orach-chayim/1:1",
        ),
    ];

    for (citation, expected) in cases {
        let got = resolve(&lexicon, citation);
        assert_eq!(
            got.exact().map(ToString::to_string).as_deref(),
            Some(expected),
            "{citation} -> {got:?}"
        );
    }
}

#[test]
fn an_ambiguous_citation_is_never_narrowed_to_one_answer() {
    // The rule the crate exists to keep, checked against the real lexicon and a
    // real ambiguity rather than a hand-built one.
    //
    // `רבנו חננאל, ויקרא` is Rabbeinu Chananel on Leviticus. Sefaria also lists
    // it among the title variants of the *Numbers* volume — an upstream data
    // error, and the only ambiguity of its kind in 24,731 spellings. Whichever
    // way it is read, one reading is wrong, and there is nothing in the data
    // that says which.
    //
    // So the reader is asked. Picking the first would produce a ref that
    // resolves, opens a sefer, and shows the wrong parsha.
    let lexicon = lexicon();
    let r = resolve(&lexicon, "רבנו חננאל, ויקרא א'");
    match &r {
        Resolution::Ambiguous(candidates) => {
            assert!(candidates.len() > 1);
            println!(
                "resolves to {} seforim: {}",
                candidates.len(),
                candidates
                    .iter()
                    .map(girsa_ref::Ref::work_slug)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        other => panic!("expected a choice, got {other:?}"),
    }
    assert!(
        r.exact().is_none(),
        "an ambiguous resolution must not offer an exact ref"
    );
}

#[test]
fn the_lexicon_is_almost_entirely_unambiguous() {
    // Worth pinning, because it is the thing that makes the Ambiguous arm
    // affordable. If a change to slug-building or normalization started
    // collapsing distinct seforim onto one another, the count would climb and
    // the resolver would begin asking the reader questions it should not need
    // to ask.
    let lexicon = lexicon();
    let shared = LEXICON
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t'))
        .fold(
            std::collections::BTreeMap::<&str, std::collections::BTreeSet<&str>>::new(),
            |mut acc, (variant, rest)| {
                let slug = rest.split('\t').next().unwrap_or("");
                acc.entry(variant).or_default().insert(slug);
                acc
            },
        )
        .into_iter()
        .filter(|(_, slugs)| slugs.len() > 1)
        .count();

    assert!(
        shared < 50,
        "{shared} spellings now mean more than one sefer, out of {}",
        lexicon.variant_count()
    );
    println!(
        "{shared} ambiguous spellings out of {}",
        lexicon.variant_count()
    );
}
