//! Other surface forms the same word might wear — offered, never applied.
//!
//! This is the relaxation ladder of spec.md §9.6, as data. The literal mode
//! calls it to *count* what a widening would find and shows the number beside
//! the offer; Smart mode calls it to apply one. Neither is allowed to change a
//! query without the reader knowing, so the transformation and the decision to
//! use it are separated here rather than tangled together in a query builder.

use crate::{abbrev, ktiv, marks::CANONICAL_GERESH, marks::CANONICAL_GERSHAYIM, prefix};

/// Which transformation produced a variant.
///
/// The declaration order is the ladder order of spec.md §9.6 — other forms,
/// then root, then abbreviations — and [`VariantSet::counts`] sorts by it, so
/// the offers appear in the order the spec sets out rather than the order a
/// `HashMap` happened to iterate in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VariantKind {
    /// Prefixes ו ה ב כ ל מ ש ד peeled off the front: `ובשבת` → `שבת`.
    PrefixPeeled,
    /// Ktiv male ↔ chaser: a `ו` or `י` added or removed: `כהן` → `כוהן`.
    KtivSwapped,
    /// The geresh or gershayim removed: `שו"ע` → `שוע`.
    GershayimDropped,
    /// Looked up in the abbreviation table: `שו"ע` → `שולחן ערוך`.
    AbbreviationExpanded,
}

impl VariantKind {
    /// Every rung, in ladder order.
    pub const ALL: [Self; 4] = [
        Self::PrefixPeeled,
        Self::KtivSwapped,
        Self::GershayimDropped,
        Self::AbbreviationExpanded,
    ];

    /// How this rung is described to a reader, for the offer chip.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::PrefixPeeled => "other forms",
            Self::KtivSwapped => "other spellings",
            Self::GershayimDropped => "without gershayim",
            Self::AbbreviationExpanded => "expand abbreviations",
        }
    }
}

/// The variants of one word, each labelled with what produced it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariantSet {
    forms: Vec<(VariantKind, String)>,
}

impl VariantSet {
    /// Every variant, with its kind, in ladder order.
    pub fn iter(&self) -> impl Iterator<Item = (VariantKind, &str)> {
        self.forms.iter().map(|(k, s)| (*k, s.as_str()))
    }

    /// Just the forms produced by one rung of the ladder.
    pub fn forms_of_kind(&self, kind: VariantKind) -> impl Iterator<Item = &str> {
        self.forms
            .iter()
            .filter(move |(k, _)| *k == kind)
            .map(|(_, s)| s.as_str())
    }

    /// How many variants each rung would add.
    ///
    /// spec.md §9.6 requires these counts be computed **before** the reader
    /// clicks, so the offer is informative on its own: you learn there are seven
    /// other forms without leaving the literal mode. They come from the same
    /// list clicking would apply, so the promise and the result cannot disagree.
    #[must_use]
    pub fn counts(&self) -> Vec<(VariantKind, usize)> {
        VariantKind::ALL
            .into_iter()
            .map(|kind| (kind, self.forms_of_kind(kind).count()))
            .filter(|(_, n)| *n > 0)
            .collect()
    }

    /// Whether there is anything to offer at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.forms.is_empty()
    }

    /// How many variants there are in total.
    #[must_use]
    pub fn len(&self) -> usize {
        self.forms.len()
    }

    /// Every distinct form, whatever produced it — what a caller actually runs
    /// when it applies more than one rung at once.
    #[must_use]
    pub fn distinct_forms(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for (_, form) in &self.forms {
            if !out.contains(&form.as_str()) {
                out.push(form);
            }
        }
        out
    }

    fn push(&mut self, kind: VariantKind, form: String, original: &str) {
        if form.is_empty() || form == original {
            return;
        }
        // Deduplicated per rung, not globally. `חת"ם סופר` reaches `חתם סופר`
        // both by dropping the gershayim and by looking the sefer up in the
        // table, and it belongs to both: `[expand abbreviations — 2]` has to
        // count what that rung produces, or the offer under-reports itself and
        // a reader who clicks the rung they meant gets told it does nothing.
        //
        // Collapsing the union is the caller's job, at the point of applying —
        // see `distinct_forms`.
        if self
            .forms
            .iter()
            .any(|(k, existing)| *k == kind && *existing == form)
        {
            return;
        }
        self.forms.push((kind, form));
    }
}

/// Every variant of a word, across every rung of the ladder.
///
/// Operates on the string as given, character for character — it does not
/// normalize first. A caller comparing against the index normalizes, then asks
/// for variants; a caller showing a reader `שו"ע` → `שולחן ערוך` wants the
/// spelling a person would read, final letters and all.
#[must_use]
pub fn variants(word: &str) -> VariantSet {
    variants_with(word, &VariantKind::ALL)
}

/// [`variants`], restricted to the given rungs.
///
/// The relaxation ladder is climbed a rung at a time (§9.6), so the caller
/// needs to ask for one rung without paying for the rest.
#[must_use]
pub fn variants_with(word: &str, kinds: &[VariantKind]) -> VariantSet {
    let mut set = VariantSet::default();

    // The ladder is climbed in declaration order, not in the order `kinds`
    // happens to be written, so the offers a reader sees are always in the same
    // sequence.
    for kind in VariantKind::ALL {
        if !kinds.contains(&kind) {
            continue;
        }
        match kind {
            VariantKind::PrefixPeeled => {
                for form in prefix::peelings(word) {
                    set.push(kind, form, word);
                }
            }
            VariantKind::KtivSwapped => {
                for form in ktiv::spellings(word) {
                    set.push(kind, form, word);
                }
            }
            VariantKind::GershayimDropped => {
                let bare: String = word
                    .chars()
                    .filter(|c| *c != CANONICAL_GERESH && *c != CANONICAL_GERSHAYIM)
                    .collect();
                set.push(kind, bare, word);
            }
            VariantKind::AbbreviationExpanded => {
                for form in abbrev::expansions_of(word) {
                    set.push(kind, form, word);
                }
            }
        }
    }

    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asking_for_one_rung_does_not_pay_for_the_others() {
        let only_prefixes = variants_with("ובשבת", &[VariantKind::PrefixPeeled]);
        assert!(only_prefixes
            .counts()
            .iter()
            .all(|(k, _)| *k == VariantKind::PrefixPeeled));
    }

    #[test]
    fn a_word_with_nothing_to_offer_offers_nothing() {
        // An honest empty is the point: the interface must not show
        // "[try other forms — 0]".
        //
        // אב is two letters, so there is no internal position to put a mater
        // lectionis in and no prefix that could come off without dissolving it;
        // "hello" is not Hebrew at all. Most Hebrew words of three letters or
        // more *do* have something to offer, because a ו or י could always have
        // been written in — which is the point of the rung, not a defect in it.
        assert!(variants("אב").is_empty());
        assert!(variants("hello").is_empty());
    }

    #[test]
    fn one_rung_never_offers_the_same_form_twice() {
        // Within a rung, a repeat would make "[try other forms — 7]" count one
        // result as two and the number beside the offer would be a lie.
        for kind in VariantKind::ALL {
            let v = variants_with("שו\"ע", &[kind]);
            let mut forms: Vec<&str> = v.iter().map(|(_, f)| f).collect();
            let before = forms.len();
            forms.sort_unstable();
            forms.dedup();
            assert_eq!(forms.len(), before, "{kind:?} repeated a form");
        }
    }

    #[test]
    fn a_form_two_rungs_both_reach_belongs_to_both() {
        // חת"ם סופר becomes חתם סופר by dropping the gershayim *and* by looking
        // the sefer up in the table. Filing it under only whichever rung ran
        // first would make the other one claim it produces nothing.
        let v = variants("חת\"ם סופר");
        assert!(v
            .forms_of_kind(VariantKind::GershayimDropped)
            .any(|f| f == "חתם סופר"));
        assert!(v
            .forms_of_kind(VariantKind::AbbreviationExpanded)
            .any(|f| f == "חתם סופר"));
    }

    #[test]
    fn applying_two_rungs_at_once_runs_each_query_once() {
        // The counts are per rung; the work is not. A caller that applies the
        // whole ladder must not search for the same string twice.
        let v = variants("חת\"ם סופר");
        let distinct = v.distinct_forms();
        assert!(
            distinct.len() < v.len(),
            "nothing was shared, so this proves nothing"
        );
        let mut sorted = distinct.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), distinct.len());
    }

    #[test]
    fn dropping_gershayim_from_a_word_without_any_offers_nothing() {
        let v = variants_with("שבת", &[VariantKind::GershayimDropped]);
        assert!(v.is_empty());
    }
}
