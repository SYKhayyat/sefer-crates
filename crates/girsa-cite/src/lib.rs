//! Citation formatting — one implementation, compiled into both applications.
//!
//! The app that *produces* citations and the app that *prints* them cannot be
//! allowed to disagree; that is precisely the class of bug that would destroy
//! trust in the pairing (spec.md §12). So there is one formatter and both link
//! against it.
//!
//! ```
//! use girsa_cite::{cite, CiteStyle, Sefer};
//! use girsa_ref::Ref;
//!
//! let sefer = Sefer::new("שולחן ערוך, אורח חיים", "Shulchan Arukh, Orach Chayim")
//!     .with_sections(["סימן", "סעיף"]);
//! let r: Ref = "girsa:shulchan-arukh/orach-chayim/1:1".parse()?;
//!
//! assert_eq!(cite(&sefer, &r, CiteStyle::HebrewFull),
//!            "שולחן ערוך, אורח חיים סימן א' סעיף א'");
//! assert_eq!(cite(&sefer, &r, CiteStyle::HebrewShort),
//!            "שולחן ערוך, אורח חיים א', א'");
//! assert_eq!(cite(&sefer, &r, CiteStyle::English),
//!            "Shulchan Arukh, Orach Chayim 1:1");
//! # Ok::<(), girsa_ref::RefError>(())
//! ```
//!
//! # The printed string is not the citation
//!
//! What a document stores is the [`Ref`]; this is only how it is *shown*
//! (spec.md §10.2). That is what makes citations alive — a sefer can be
//! switched from the full form to the short one, or every quote regenerated
//! against a corrected edition, without touching the prose. It also means
//! nothing here is load-bearing for correctness in the way a ref is: get this
//! wrong and a reader sees an odd-looking mekor, rather than an anchor pointing
//! at the wrong words.
//!
//! # Two things it deliberately will not do
//!
//! **It does not invent an abbreviation.** `שו"ע או"ח` is how a sefer prints
//! Shulchan Arukh Orach Chayim, and this crate will print it only if the caller
//! supplies it as the title. Sefaria's schemas carry 44 title variants for that
//! work and nothing in the file says which of them a citation should use;
//! picking the shortest gives `OC` for one work and something unrecognisable
//! for the next. A citation that names a sefer nobody can identify is the same
//! failure as a ref that lands in the wrong place, one step further from being
//! noticed.
//!
//! **It does not invent the word for a level.** `סימן`, `סעיף`, `דף` come from
//! the schema's `heSectionNames`, through [`Sefer::with_sections`]. A work
//! whose schema never said — 1,101 of Sefaria's 6,595 are branch schemas that
//! do not carry them, and every Otzaria-only work has no schema at all — is
//! cited by number alone, which is an ordinary way to write a mekor and not a
//! degradation anyone has to be warned about.
//!
//! # What is asserted instead
//!
//! **Every citation printed here reads back, through
//! [`girsa_ref::resolve`], as the ref it was printed from.** The formatter and
//! the resolver are the two halves of one claim — that a mareh makom in a Ksav
//! document means a place in the library — and a printed form the resolver
//! cannot read is a citation this system cannot follow. See the test of that
//! name.

#![doc(html_root_url = "https://docs.rs/girsa-cite")]

use girsa_ref::address::Level;
use girsa_ref::{daf, numerals, Address, Ref};

/// How a citation is rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CiteStyle {
    /// `שולחן ערוך, אורח חיים סימן א' סעיף א'` — how it is written in a sefer,
    /// with the word for each level.
    HebrewFull,
    /// `שולחן ערוך, אורח חיים א', א'` — how it is written in a footnote, where
    /// the words are dropped and the numbers carry it.
    HebrewShort,
    /// `Shulchan Arukh, Orach Chayim 1:1` — Sefaria-compatible, so a citation
    /// can be pasted into their search box and land.
    English,
}

impl CiteStyle {
    /// The name this style goes by on the wire, for a chip or a setting.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::HebrewFull => "hebrew-full",
            Self::HebrewShort => "hebrew-short",
            Self::English => "english",
        }
    }

    /// Read a style back off the wire.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "hebrew-full" => Some(Self::HebrewFull),
            "hebrew-short" => Some(Self::HebrewShort),
            "english" => Some(Self::English),
            _ => None,
        }
    }
}

/// What a citation needs to know about the sefer it names.
///
/// Small on purpose: a formatter that took the whole catalogue record would
/// have to live in Girsa, and then Ksav would have a second implementation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sefer {
    /// How it is printed in Hebrew, exactly as it should appear.
    pub he_title: String,
    /// How it is printed in English.
    pub en_title: String,
    /// What the levels of an address are called, outermost first —
    /// `["סימן", "סעיף"]`. From the schema; empty where it does not say.
    pub he_sections: Vec<String>,
}

impl Sefer {
    #[must_use]
    pub fn new(he_title: impl Into<String>, en_title: impl Into<String>) -> Self {
        Self {
            he_title: he_title.into(),
            en_title: en_title.into(),
            he_sections: Vec::new(),
        }
    }

    /// The words for the levels, as the schema states them.
    #[must_use]
    pub fn with_sections<S: Into<String>>(mut self, names: impl IntoIterator<Item = S>) -> Self {
        self.he_sections = names.into_iter().map(Into::into).collect();
        self
    }
}

/// Print a ref the way it is cited.
///
/// A ref with no address is the sefer itself, and comes back as its name.
#[must_use]
pub fn cite(sefer: &Sefer, reference: &Ref, style: CiteStyle) -> String {
    let title = match style {
        CiteStyle::English => sefer.en_title.trim(),
        CiteStyle::HebrewFull | CiteStyle::HebrewShort => sefer.he_title.trim(),
    };

    let mut out = title.to_string();
    if reference.from().is_empty() {
        return out;
    }

    out.push(' ');
    out.push_str(&address(sefer, reference.from(), style));
    if let Some(to) = reference.to() {
        // The two ends of a span, joined the way Sefaria joins them — and the
        // way `girsa-ref` reads one back, which is the property the round-trip
        // test turns on.
        out.push('-');
        out.push_str(&address(sefer, to, style));
    }
    out
}

/// One address — `סימן א' סעיף א'`, `א', א'`, `1:1`.
fn address(sefer: &Sefer, address: &Address, style: CiteStyle) -> String {
    let levels = address.levels();
    // The words are printed only where the address is the shape the schema
    // described: as many levels as it named, every one of them a number or a
    // daf. Mishnah Berurah's schema says `["סימן", "סעיף קטן"]` for its body
    // and it also has an introduction, addressed `הקדמה:3` — one named level
    // and one number. Applying the words by position there prints
    // *`סימן הקדמה סעיף קטן ג'`*, which names two divisions the sefer does not
    // have. Where the shape does not match, the numbers carry it alone.
    let words = style == CiteStyle::HebrewFull
        && !sefer.he_sections.is_empty()
        && levels.len() <= sefer.he_sections.len()
        && levels.iter().all(Level::is_numbered);

    let mut out = String::new();
    for (depth, level) in levels.iter().enumerate() {
        let text = written(level, style);
        if out.is_empty() {
            out.push_str(&section_word(sefer, depth, words));
            out.push_str(&text);
            continue;
        }
        match style {
            // Sefaria's own separator, and what a pasted citation is read with.
            CiteStyle::English => out.push(':'),
            CiteStyle::HebrewFull => out.push(' '),
            // A daf prints its own mark — `ב.` is daf 2 side one — so nothing
            // follows it but a space. Elsewhere the comma is what keeps
            // `קכ"א ג'` from reading as one number.
            CiteStyle::HebrewShort => {
                out.push_str(if ends_a_daf(&out) { " " } else { ", " });
            }
        }
        out.push_str(&section_word(sefer, depth, words));
        out.push_str(&text);
    }
    out
}

/// The schema's word for this level, with its trailing space — or nothing.
fn section_word(sefer: &Sefer, depth: usize, words: bool) -> String {
    if !words {
        return String::new();
    }
    sefer
        .he_sections
        .get(depth)
        .map(|word| format!("{} ", word.trim()))
        .unwrap_or_default()
}

/// One level, as it is printed.
fn written(level: &Level, style: CiteStyle) -> String {
    match style {
        CiteStyle::English => level.as_str().to_string(),
        CiteStyle::HebrewFull | CiteStyle::HebrewShort => {
            if let Some(hebrew) = daf::to_hebrew(level.as_str()) {
                return hebrew;
            }
            match level.as_number() {
                Some(n) => numerals::to_hebrew(n),
                // A named level — `Introduction`, `הקדמה`. Printed as the
                // corpus names it: a section name this crate translated would
                // be a name the reader cannot find in the sefer.
                None => level.as_str().to_string(),
            }
        }
    }
}

/// Whether what has been printed so far ends in a daf's own mark.
fn ends_a_daf(printed: &str) -> bool {
    printed.ends_with('.') || printed.ends_with(':')
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_ref::{resolve, Lexicon, Resolution};

    fn r(s: &str) -> Ref {
        s.parse().expect("a ref")
    }

    fn shulchan_arukh() -> Sefer {
        Sefer::new("שולחן ערוך, אורח חיים", "Shulchan Arukh, Orach Chayim")
            .with_sections(["סימן", "סעיף"])
    }

    fn berakhot() -> Sefer {
        // Verbatim from `corpus/sefaria/schemas/Berakhot.json`.
        Sefer::new("ברכות", "Berakhot").with_sections(["דף", "שורה"])
    }

    #[test]
    fn a_citation_names_the_sefer_and_the_place() {
        assert_eq!(
            cite(
                &shulchan_arukh(),
                &r("girsa:shulchan-arukh/orach-chayim/1:1"),
                CiteStyle::HebrewFull
            ),
            "שולחן ערוך, אורח חיים סימן א' סעיף א'"
        );
    }

    #[test]
    fn the_short_form_drops_the_words_and_keeps_the_numbers() {
        assert_eq!(
            cite(
                &shulchan_arukh(),
                &r("girsa:shulchan-arukh/orach-chayim/121:3"),
                CiteStyle::HebrewShort
            ),
            "שולחן ערוך, אורח חיים קכ\"א, ג'"
        );
    }

    #[test]
    fn english_is_written_the_way_sefaria_writes_it() {
        // So that a citation Girsa printed can be pasted into their search box
        // and land — the one interoperability this crate is asked for.
        assert_eq!(
            cite(
                &shulchan_arukh(),
                &r("girsa:shulchan-arukh/orach-chayim/121:3"),
                CiteStyle::English
            ),
            "Shulchan Arukh, Orach Chayim 121:3"
        );
    }

    #[test]
    fn a_daf_is_printed_as_a_daf_and_carries_its_own_mark() {
        let gemara = r("girsa:bavli/berakhot/2a:1");
        assert_eq!(
            cite(&berakhot(), &gemara, CiteStyle::HebrewShort),
            "ברכות ב. א'"
        );
        assert_eq!(
            cite(&berakhot(), &gemara, CiteStyle::HebrewFull),
            "ברכות דף ב. שורה א'"
        );
        assert_eq!(
            cite(&berakhot(), &gemara, CiteStyle::English),
            "Berakhot 2a:1"
        );
        // Amud beis, so the mark is a colon rather than a full stop — and the
        // numeral is written bare. The mark is what says these letters are a
        // daf, so a gershayim on top of it would be saying it twice; the daf
        // reader takes both spellings back.
        assert_eq!(
            cite(
                &berakhot(),
                &r("girsa:bavli/berakhot/33b:9"),
                CiteStyle::HebrewShort
            ),
            "ברכות לג: ט'"
        );
    }

    #[test]
    fn a_sefer_whose_schema_never_said_what_a_level_is_gets_no_invented_word() {
        // 1,101 branch schemas carry no `heSectionNames`, and no Otzaria-only
        // work has a schema at all. Cited by number, which is an ordinary way
        // to write a mekor — not a degraded one.
        let mishnah_berurah = Sefer::new("משנה ברורה", "Mishnah Berurah");
        let place = r("girsa:mishnah-berurah/121:3");
        assert_eq!(
            cite(&mishnah_berurah, &place, CiteStyle::HebrewFull),
            "משנה ברורה קכ\"א ג'"
        );
        assert_eq!(
            cite(&mishnah_berurah, &place, CiteStyle::HebrewShort),
            "משנה ברורה קכ\"א, ג'"
        );
    }

    #[test]
    fn a_span_is_printed_as_a_span_because_a_quote_is_a_range() {
        assert_eq!(
            cite(
                &berakhot(),
                &r("girsa:bavli/berakhot/2a:1-2b:4"),
                CiteStyle::English
            ),
            "Berakhot 2a:1-2b:4"
        );
        assert_eq!(
            cite(
                &berakhot(),
                &r("girsa:bavli/berakhot/2a:1-2b:4"),
                CiteStyle::HebrewShort
            ),
            "ברכות ב. א'-ב: ד'"
        );
    }

    #[test]
    fn a_ref_to_a_whole_sefer_is_its_name() {
        // The trailing slash is the ref saying it has no address —
        // `girsa:bavli/berakhot` without it means the work `bavli` at a
        // section called `berakhot`, which is a different place. Writing this
        // test is what found that a whole sefer could not be written down at
        // all; `girsa-ref` grew the slash for it.
        for text in ["girsa:bavli/berakhot/", "girsa:berakhot"] {
            assert_eq!(cite(&berakhot(), &r(text), CiteStyle::HebrewShort), "ברכות");
            assert_eq!(cite(&berakhot(), &r(text), CiteStyle::English), "Berakhot");
        }
    }

    #[test]
    fn the_words_are_printed_only_where_the_address_is_the_shape_the_schema_described() {
        // Mishnah Berurah's schema names its body `["סימן", "סעיף קטן"]` and
        // the sefer also has an introduction, addressed `הקדמה:3`. By position
        // that would print `סימן הקדמה סעיף קטן ג'` — two divisions the sefer
        // does not have, in a mekor somebody would go looking for.
        let mb = Sefer::new("משנה ברורה", "Mishnah Berurah").with_sections(["סימן", "סעיף קטן"]);
        assert_eq!(
            cite(
                &mb,
                &r("girsa:mishnah-berurah/121:3"),
                CiteStyle::HebrewFull
            ),
            "משנה ברורה סימן קכ\"א סעיף קטן ג'"
        );
        assert_eq!(
            cite(
                &mb,
                &r("girsa:mishnah-berurah/הקדמה:3"),
                CiteStyle::HebrewFull
            ),
            "משנה ברורה הקדמה ג'"
        );
        // Deeper than the schema named, too: the words would run out and the
        // last level would be the only one without one.
        assert_eq!(
            cite(
                &mb,
                &r("girsa:mishnah-berurah/121:3:2"),
                CiteStyle::HebrewFull
            ),
            "משנה ברורה קכ\"א ג' ב'"
        );
    }

    #[test]
    fn a_named_level_is_printed_as_the_corpus_names_it() {
        // `Abarbanel on Ezekiel, Introduction 3` — the level is a name, not a
        // number, and translating it would name a section the reader cannot
        // find in the sefer.
        let sefer = Sefer::new("אברבנאל על יחזקאל", "Abarbanel on Ezekiel");
        assert_eq!(
            cite(
                &sefer,
                &r("girsa:abarbanel-on-ezekiel/Introduction:3"),
                CiteStyle::HebrewShort
            ),
            "אברבנאל על יחזקאל Introduction, ג'"
        );
    }

    #[test]
    fn the_title_is_printed_as_given_and_never_shortened_by_this_crate() {
        // The short style shortens the *address*. Which abbreviation a sefer
        // goes by is not in the data, and a citation naming a sefer nobody can
        // identify fails in a way nobody notices.
        let long = Sefer::new("שולחן ערוך, אורח חיים", "Shulchan Arukh, Orach Chayim");
        for style in [CiteStyle::HebrewShort, CiteStyle::HebrewFull] {
            assert!(cite(&long, &r("girsa:x/1:1"), style).starts_with("שולחן ערוך, אורח חיים"));
        }
        // And a caller who *has* the abbreviation gets it printed, unchanged.
        let short = Sefer::new("שו\"ע או\"ח", "SA OC");
        assert_eq!(
            cite(&short, &r("girsa:x/1:1"), CiteStyle::HebrewShort),
            "שו\"ע או\"ח א', א'"
        );
    }

    #[test]
    fn every_citation_printed_here_reads_back_as_the_ref_it_came_from() {
        // The property the whole crate is for. A mareh makom in a Ksav document
        // has to mean a place in the library, and a printed form the resolver
        // cannot read is a citation this system cannot follow.
        let mut lexicon = Lexicon::default();
        lexicon.add(
            girsa_ref::Work {
                slug: "shulchan-arukh/orach-chayim".into(),
                he_title: "שולחן ערוך, אורח חיים".into(),
                en_title: "Shulchan Arukh, Orach Chayim".into(),
            },
            &["שולחן ערוך, אורח חיים", "Shulchan Arukh, Orach Chayim"],
        );
        lexicon.add(
            girsa_ref::Work {
                slug: "bavli/berakhot".into(),
                he_title: "ברכות".into(),
                en_title: "Berakhot".into(),
            },
            &["ברכות", "Berakhot"],
        );

        let cases = [
            (shulchan_arukh(), "girsa:shulchan-arukh/orach-chayim/1:1"),
            (shulchan_arukh(), "girsa:shulchan-arukh/orach-chayim/121:3"),
            (berakhot(), "girsa:bavli/berakhot/2a:1"),
            (berakhot(), "girsa:bavli/berakhot/33b:9"),
            (berakhot(), "girsa:bavli/berakhot/2a:1-2b:4"),
        ];
        for (sefer, text) in cases {
            let reference = r(text);
            for style in [
                CiteStyle::HebrewShort,
                CiteStyle::HebrewFull,
                CiteStyle::English,
            ] {
                let printed = cite(&sefer, &reference, style);
                // A canonical ref resolves to itself — that is the paste path —
                // so a formatter that printed the ref would pass this test
                // without citing anything. It was written first against exactly
                // that stub, and this line is what caught it.
                assert_ne!(
                    printed,
                    reference.to_string(),
                    "that is the ref, not a citation"
                );
                match resolve(&lexicon, &printed) {
                    Resolution::Exact(back) => assert_eq!(
                        back, reference,
                        "{printed:?} ({style:?}) read back as a different place"
                    ),
                    other => panic!("{printed:?} ({style:?}) did not read back: {other:?}"),
                }
            }
        }
    }
}
