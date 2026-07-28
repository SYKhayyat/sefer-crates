//! Every way a sefer is written, and which sefer it is.
//!
//! This is the resolver's whole vocabulary, and it already existed: Sefaria
//! ships a schema per work carrying `titleVariants` and `heTitleVariants`.
//! `Shulchan_Arukh,_Orach_Chayim.json` alone has 44 of them —
//!
//! ```text
//! שולחן ערוך אורח חיים · שו"ע או"ח · שו״ע או״ח · שו”ע או”ח · שלחן ערוך או"ח
//! או"ח · או״ח · או”ח · אורח חיים · S.A. O.C. · SA OC · OC · O.C. · O.Ch.
//! Shulchan Aruch, Orach Chaim · Shulhan Arukh OH · Orakh Hayim · …
//! ```
//!
//! — across 6,595 works, in both languages, machine-readable. spec.md §4.3 is
//! largely downstream of that file.
//!
//! Variants are keyed by their normal form (`girsa_hebrew::normalize`), so
//! which gershayim character the reader typed never reaches this map.
//!
//! # One variant, several seforim
//!
//! `או"ח` is Orach Chayim, and it is also the Orach Chayim volume of the Tur,
//! of the Arukh HaShulchan, and of a hundred sets of responsa. The map holds
//! **all** of them, and the resolver hands the reader a choice. Storing one and
//! calling it the answer is the guess BUILDER rule 6 forbids, and it is exactly
//! how a citation quietly lands in the wrong sefer.

use std::collections::BTreeMap;

use girsa_hebrew::normalize;

/// A work the lexicon knows about.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Work {
    /// The ref slug — `shulchan-arukh/orach-chayim`.
    pub slug: String,
    /// How to print it in Hebrew — `שולחן ערוך, אורח חיים`.
    pub he_title: String,
    /// How to print it in English — `Shulchan Arukh, Orach Chayim`.
    pub en_title: String,
}

/// Title variant → the works that could be meant.
#[derive(Debug, Clone, Default)]
pub struct Lexicon {
    by_variant: BTreeMap<String, Vec<usize>>,
    works: Vec<Work>,
    /// How many words the longest variant is, so the resolver knows how far to
    /// look before giving up rather than trying every split of the input.
    longest_variant_words: usize,
}

impl Lexicon {
    /// Load the generated lexicon.
    ///
    /// One row per variant: `variant \t slug \t he-title \t en-title`. Lines
    /// starting with `#` are comments. A malformed row is skipped rather than
    /// failing the load — a lexicon that refuses to load leaves the reader with
    /// no resolver at all, which is worse than one missing a title.
    #[must_use]
    pub fn from_tsv(tsv: &str) -> Self {
        let mut lexicon = Self::default();
        let mut index_of: BTreeMap<String, usize> = BTreeMap::new();

        for line in tsv.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(variant), Some(slug)) = (fields.next(), fields.next()) else {
                continue;
            };
            let he_title = fields.next().unwrap_or(variant);
            let en_title = fields.next().unwrap_or(slug);
            if variant.trim().is_empty() || slug.trim().is_empty() {
                continue;
            }

            let index = *index_of.entry(slug.to_string()).or_insert_with(|| {
                lexicon.works.push(Work {
                    slug: slug.to_string(),
                    he_title: he_title.to_string(),
                    en_title: en_title.to_string(),
                });
                lexicon.works.len() - 1
            });

            lexicon.insert(variant, index);
        }
        lexicon
    }

    fn insert(&mut self, variant: &str, work: usize) {
        let key = normalize(variant);
        if key.is_empty() {
            return;
        }
        let words = key.split_whitespace().count();
        self.longest_variant_words = self.longest_variant_words.max(words);

        let entry = self.by_variant.entry(key).or_default();
        if !entry.contains(&work) {
            entry.push(work);
        }
    }

    /// Add a work and its variants. Used by the personal layer for a reader's
    /// own seforim (spec.md §5) and by tests.
    pub fn add(&mut self, work: Work, variants: &[&str]) {
        let index = match self.works.iter().position(|w| w.slug == work.slug) {
            Some(i) => i,
            None => {
                self.works.push(work);
                self.works.len() - 1
            }
        };
        for v in variants {
            self.insert(v, index);
        }
    }

    /// Every work a title could mean. Empty if the title is unknown.
    ///
    /// More than one is the normal case, not an error — see the module note.
    #[must_use]
    pub fn lookup(&self, title: &str) -> Vec<&Work> {
        let key = normalize(title);
        self.by_variant
            .get(&key)
            .map(|indices| indices.iter().filter_map(|i| self.works.get(*i)).collect())
            .unwrap_or_default()
    }

    /// How many works are known.
    #[must_use]
    pub fn len(&self) -> usize {
        self.works.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.works.is_empty()
    }

    /// How many distinct spellings are known.
    #[must_use]
    pub fn variant_count(&self) -> usize {
        self.by_variant.len()
    }

    /// The word count of the longest title, which bounds the resolver's search.
    #[must_use]
    pub fn longest_variant_words(&self) -> usize {
        self.longest_variant_words.max(1)
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace bans these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn sample() -> Lexicon {
        Lexicon::from_tsv(
            "שולחן ערוך אורח חיים\tshulchan-arukh/orach-chayim\tשולחן ערוך, אורח חיים\tShulchan Arukh, Orach Chayim\n\
             שו\"ע או\"ח\tshulchan-arukh/orach-chayim\tשולחן ערוך, אורח חיים\tShulchan Arukh, Orach Chayim\n\
             או\"ח\tshulchan-arukh/orach-chayim\tשולחן ערוך, אורח חיים\tShulchan Arukh, Orach Chayim\n\
             או\"ח\ttur/orach-chayim\tטור, אורח חיים\tTur, Orach Chayim\n\
             ברכות\tbavli/berakhot\tברכות\tBerakhot\n",
        )
    }

    #[test]
    fn a_title_finds_its_work() {
        let lex = sample();
        let found = lex.lookup("שולחן ערוך אורח חיים");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].slug, "shulchan-arukh/orach-chayim");
    }

    #[test]
    fn which_gershayim_was_typed_never_reaches_the_map() {
        let lex = sample();
        for spelling in ["שו\"ע או\"ח", "שו״ע או״ח", "שו”ע או”ח"] {
            assert_eq!(lex.lookup(spelling).len(), 1, "{spelling}");
        }
    }

    #[test]
    fn a_variant_two_seforim_share_returns_both() {
        // או"ח is Orach Chayim in the Shulchan Arukh and in the Tur. Returning
        // one of them would be a guess.
        let lex = sample();
        let found = lex.lookup("או\"ח");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn an_unknown_title_returns_nothing_rather_than_the_nearest_thing() {
        assert!(sample().lookup("קרן אורה").is_empty());
    }

    #[test]
    fn a_malformed_row_is_skipped_and_the_rest_still_loads() {
        let lex = Lexicon::from_tsv(
            "ברכות\tbavli/berakhot\nrubbish-with-no-tab\n\t\nשבת\tbavli/shabbat\n",
        );
        assert_eq!(lex.len(), 2);
    }
}
