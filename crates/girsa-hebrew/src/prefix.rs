//! Stacked prefix peeling — offered, never baked in.
//!
//! `וּבַשַּׁבָּת` is `ו` + `ב` + `שבת`, and a reader who types `שבת` means to find it.
//! But `משה` is `מ` + `ש` + `ה` to a machine, and peeling it leaves a single
//! letter that matches a tenth of the corpus. There is no lexicon here to tell
//! the two apart (spec.md §9.4 — morphology is deferred, and deliberately), so
//! the rule that keeps this safe is structural rather than linguistic:
//!
//! **A peeling that leaves fewer than two letters is not offered.**
//!
//! That is not a guess about Hebrew; it is a floor under how wrong this can go.
//! A junk stem that no sefer contains costs nothing — it contributes zero to the
//! count beside `[try other forms — 7]`, so the reader never sees it. A stem of
//! one letter matches everything, which the reader very much does see.

/// The letters that attach to the front of a Hebrew word (spec.md §9.2).
///
/// Conjunctive ו, definite ה, and the prepositions ב כ ל מ ש ד. Read
/// `Sefaria-ElasticSearch` for which set it uses; this one is written from the
/// grammar, because that repository is GPL-3.0 and these crates are compiled
/// into Ksav (BUILDER.md T7).
pub const PREFIX_LETTERS: [char; 8] = ['ו', 'ה', 'ב', 'כ', 'ל', 'מ', 'ש', 'ד'];

/// Below this many letters, a stem is not a word — it is a wildcard.
const MIN_STEM_LETTERS: usize = 2;

/// Every stem that could be left after peeling prefixes off a normalized word.
///
/// Returned shallowest first, so `ובשבת` gives `["בשבת", "שבת", "בת"]` and a
/// caller that wants only the most conservative reading can take the first.
#[must_use]
pub fn peelings(word: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut frontier = vec![word.to_string()];

    while let Some(current) = frontier.pop() {
        let mut chars = current.chars();
        let Some(first) = chars.next() else { continue };
        if !PREFIX_LETTERS.contains(&first) {
            continue;
        }

        let stem: String = chars.collect();
        if stem.chars().count() < MIN_STEM_LETTERS {
            continue;
        }
        if out.contains(&stem) || stem == word {
            continue;
        }

        out.push(stem.clone());
        frontier.push(stem);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stacked_prefixes_peel_one_at_a_time() {
        assert_eq!(peelings("ובשבת"), vec!["בשבת", "שבת", "בת"]);
    }

    #[test]
    fn four_stacked_prefixes_still_reach_the_stem() {
        // וכשהמלך — the case that looks done and isn't.
        assert!(peelings("וכשהמלכ").contains(&"מלכ".to_string()));
    }

    #[test]
    fn a_word_that_is_all_prefix_letters_is_not_dissolved() {
        for stem in peelings("משה") {
            assert!(stem.chars().count() >= MIN_STEM_LETTERS, "{stem:?}");
        }
    }

    #[test]
    fn a_word_starting_with_no_prefix_letter_has_no_peelings() {
        assert!(peelings("אמת").is_empty());
    }
}
