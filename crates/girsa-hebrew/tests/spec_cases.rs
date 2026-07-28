//! Every case spec.md states in prose, asserted.
//!
//! These are not invented examples. Each row of spec.md §9.2 and each line of
//! BUILDER.md W2's acceptance appears here, so the crate cannot drift away from
//! what was promised without a test going red.

use girsa_hebrew::{normalize, variants, VariantKind, VariantSet};

/// Whether a query could reach a piece of text, allowing the given rungs of the
/// relaxation ladder — the question every §9.2 row is really asking.
fn reachable(query: &str, text: &str, allowed: &[VariantKind]) -> bool {
    let text = normalize(text);
    let query = normalize(query);

    let mut candidates = vec![query.clone()];
    candidates.extend(
        variants(&query)
            .iter()
            .filter(|(kind, _)| allowed.contains(kind))
            .map(|(_, form)| form.to_string()),
    );
    // The reverse direction matters just as much: the reader types the short
    // form and the page carries the long one, or the other way round.
    let text_forms: Vec<String> = std::iter::once(text.clone())
        .chain(
            variants(&text)
                .iter()
                .filter(|(kind, _)| allowed.contains(kind))
                .map(|(_, form)| form.to_string()),
        )
        .collect();

    candidates.iter().any(|c| {
        text_forms
            .iter()
            .any(|t| t.contains(c.as_str()) || c.contains(t))
    })
}

const EVERY_RUNG: &[VariantKind] = &[
    VariantKind::PrefixPeeled,
    VariantKind::KtivSwapped,
    VariantKind::GershayimDropped,
    VariantKind::AbbreviationExpanded,
];

// ---------------------------------------------------------------------------
// BUILDER.md W2 — Acceptance
// ---------------------------------------------------------------------------

#[test]
fn acceptance_searching_shabbos_finds_it_menukad_and_prefixed() {
    assert!(reachable("שבת", "וּבַשַּׁבָּת", EVERY_RUNG));
}

#[test]
fn acceptance_searching_kohen_finds_the_male_spelling() {
    assert!(reachable("כהן", "כוהן", EVERY_RUNG));
}

#[test]
fn acceptance_searching_the_abbreviation_finds_the_sefer_written_out() {
    assert!(reachable("שו\"ע", "שולחן ערוך", EVERY_RUNG));
}

// ---------------------------------------------------------------------------
// spec.md §9.2 — the table, row by row
// ---------------------------------------------------------------------------

#[test]
fn row_1_nikud_stripped_and_stacked_prefixes_peeled() {
    assert_eq!(normalize("וּבַשַּׁבָּת"), "ובשבת");
    let v = variants("ובשבת");
    assert!(
        v.forms_of_kind(VariantKind::PrefixPeeled)
            .any(|f| f == "שבת"),
        "peeling ו then ב must reach שבת; got {:?}",
        v.forms_of_kind(VariantKind::PrefixPeeled)
            .collect::<Vec<_>>()
    );
}

#[test]
fn row_2_ktiv_male_and_chaser_are_the_same_word() {
    assert!(variants("כהן")
        .forms_of_kind(VariantKind::KtivSwapped)
        .any(|f| f == "כוהן"));
    assert!(variants("כוהן")
        .forms_of_kind(VariantKind::KtivSwapped)
        .any(|f| f == "כהן"));
}

#[test]
fn row_3_an_abbreviation_expands_to_the_sefer() {
    assert!(variants("שו\"ע")
        .forms_of_kind(VariantKind::AbbreviationExpanded)
        .any(|f| f == "שולחן ערוך"));
}

#[test]
fn row_3_expansion_runs_in_both_directions() {
    assert!(variants("שולחן ערוך")
        .forms_of_kind(VariantKind::AbbreviationExpanded)
        .any(|f| f == "שו\"ע"));
}

#[test]
fn row_4_a_rabbinic_acronym_expands_to_the_name() {
    assert!(variants("רמב\"ם")
        .forms_of_kind(VariantKind::AbbreviationExpanded)
        .any(|f| f == "רבינו משה בן מיימון"));
}

#[test]
fn row_5_a_trailing_geresh_does_not_hide_a_word() {
    assert!(reachable("ארץ", "אָרֶץ׳", EVERY_RUNG));
}

// ---------------------------------------------------------------------------
// The fold that looks done and isn't
// ---------------------------------------------------------------------------

#[test]
fn every_spelling_of_shulchan_aruch_normalizes_the_same() {
    // Berakhot writes gershayim as U+05F4; Mishnah Berurah writes it as an
    // ASCII quote; anything pasted from Word arrives curly. Same sefer.
    let spellings = ["שו\"ע", "שו״ע", "שו”ע", "שו“ע"];
    let first = normalize(spellings[0]);
    for s in spellings {
        assert_eq!(
            normalize(s),
            first,
            "{s} must normalize like {}",
            spellings[0]
        );
    }
}

#[test]
fn every_spelling_of_a_geresh_normalizes_the_same() {
    let spellings = ["גמ׳", "גמ'", "גמ’", "גמ‘"];
    let first = normalize(spellings[0]);
    for s in spellings {
        assert_eq!(
            normalize(s),
            first,
            "{s} must normalize like {}",
            spellings[0]
        );
    }
}

// ---------------------------------------------------------------------------
// Properties the whole system leans on
// ---------------------------------------------------------------------------

#[test]
fn normalizing_is_idempotent() {
    // Nikud coverage is inconsistent across the corpus — Berakhot is fully
    // menukad and Mishnah Berurah has none — so the normalizer meets already
    // bare text constantly and must be safe on it.
    for s in [
        "וּבַשַּׁבָּת",
        "ובשבת",
        "שו\"ע",
        "בְּרֵאשִׁית בָּרָא אֱלֹהִים אֵת הַשָּׁמַיִם וְאֵת הָאָרֶץ׃",
        "",
        "   ",
        "hello world",
    ] {
        let once = normalize(s);
        assert_eq!(normalize(&once), once, "not idempotent on {s:?}");
    }
}

#[test]
fn normalizing_never_empties_a_real_word() {
    for s in ["משה", "אל", "ה'", "כהן", "שבת"] {
        assert!(!normalize(s).is_empty(), "{s} normalized to nothing");
    }
}

#[test]
fn a_maqaf_separates_two_words_rather_than_gluing_them() {
    // Deleting U+05BE would produce אתהשמים, and the second verse of the Torah
    // would stop being findable by searching for either word in it.
    let n = normalize("אֶת־הַשָּׁמַיִם");
    assert!(
        n.split_whitespace().count() == 2,
        "maqaf must break the word; got {n:?}"
    );
    assert!(n.contains("את"));
    assert!(n.contains("השמימ") || n.contains("השמים"));
}

#[test]
fn a_sof_pasuq_does_not_glue_a_verse_to_the_next() {
    let n = normalize("הָאָרֶץ׃ וְהָאָרֶץ");
    assert_eq!(n.split_whitespace().count(), 2, "got {n:?}");
}

// ---------------------------------------------------------------------------
// Rule 6, in the one place it can be tested here
// ---------------------------------------------------------------------------

#[test]
fn peeling_never_eats_a_word_down_to_nothing() {
    // משה is four prefix letters in a row if you are not careful: מ, ש, ה.
    // Peeling it to a single letter would make it match half the corpus.
    for word in ["משה", "שם", "מה", "לב", "בו", "הוא", "דוד"] {
        for peeled in variants(word).forms_of_kind(VariantKind::PrefixPeeled) {
            assert!(
                peeled.chars().count() >= 2,
                "peeling {word} produced {peeled:?}, which is not a word"
            );
        }
    }
}

#[test]
fn the_counts_the_interface_shows_match_what_it_would_apply() {
    // §9.6 requires the count be computed before the reader clicks, and clicking
    // must then apply exactly that many. One source, so they cannot disagree.
    let v: VariantSet = variants("ובשבת");
    for (kind, count) in v.counts() {
        assert_eq!(
            v.forms_of_kind(kind).count(),
            count,
            "offer for {kind:?} promises {count} and would apply a different number"
        );
    }
}

#[test]
fn a_variant_is_never_the_word_it_came_from() {
    // An offer of "try other forms — 7" where one of the seven is what you
    // already typed is a lie about how much is on the other side of the click.
    for word in ["ובשבת", "כהן", "שו\"ע", "רמב\"ם"] {
        let normalized = normalize(word);
        for (kind, form) in variants(&normalized).iter() {
            assert_ne!(
                form, normalized,
                "{kind:?} offered the input back as a variant"
            );
        }
    }
}

#[test]
fn variants_are_offered_in_ladder_order() {
    // spec.md §9.6: drop nikud → other forms → root → expand abbreviations.
    // Nikud is not a rung here because it is never optional (§9.1).
    assert!(VariantKind::PrefixPeeled < VariantKind::AbbreviationExpanded);
    assert!(VariantKind::KtivSwapped < VariantKind::AbbreviationExpanded);
}
