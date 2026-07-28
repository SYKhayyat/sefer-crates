//! A hyphen inside an address level is a hyphen, not a span separator.
//!
//! # The hazard
//!
//! `Ref` writes a span as `from-to` and has no escape, so a `-` that is part of
//! a *level* reads back as the separator between two addresses:
//!
//! ```text
//! girsa:tur/orach-chayim:240:1
//!           └── one named level ──┘   what it means
//!           └─┘ └──────────────┘      what it parsed as: a range,
//!           from        to            from `orach` to `chayim:240:1`
//! ```
//!
//! That is a place-shaped thing which is not a place, and nothing errors. W8
//! stopped Girsa's importer from ever writing such an id — section labels join
//! with `_` — but the misreading lived in this crate, so it was still waiting
//! for the next caller. Sefaria really does name sections `כסלו-טבת` and
//! `שער חמישי - שער ייחוד המעשה`, and this crate's own resolver builds levels
//! out of them.
//!
//! # The rule these tests fix in place
//!
//! **A hyphen separates two addresses only when both sides are addressed by
//! number** — a daf, a siman, a se'if, a perek. A named level on either side of
//! it means the hyphen belongs to the name.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use girsa_ref::{Address, Level, Ref};

/// Every one of these is a level that really occurs, or one letter away from
/// one.
const HYPHENATED_LEVELS: [&str; 4] = [
    "orach-chayim",
    "כסלו-טבת",
    "שער חמישי - שער ייחוד המעשה",
    "Ki-Tisa",
];

#[test]
fn a_hyphenated_section_is_one_level_and_not_a_range() {
    let r: Ref = "girsa:tur/orach-chayim:240:1"
        .parse()
        .expect("a hyphenated section name is still a ref");

    assert_eq!(r.work_slug(), "tur");
    assert!(
        !r.is_span(),
        "read as a range from `orach` to `chayim:240:1` — \
         a place-shaped thing that is not a place, and no error"
    );
    assert_eq!(
        r.from()
            .levels()
            .iter()
            .map(Level::as_str)
            .collect::<Vec<_>>(),
        ["orach-chayim", "240", "1"],
    );
}

#[test]
fn a_hyphenated_level_survives_being_written_down_and_read_back() {
    // Refs are stored as text inside Ksav documents. A ref that does not
    // round-trip is a ref that changes meaning the next time it is opened.
    for name in HYPHENATED_LEVELS {
        let built = Ref::point(
            vec!["work".into()],
            Address::new(vec![Level::canonical(name), Level::number(5)]),
        );
        let printed = built.to_string();
        let read: Ref = printed.parse().unwrap_or_else(|e| panic!("{printed}: {e}"));
        assert_eq!(read, built, "{printed} came back as a different ref");
        assert!(
            built.is_well_formed(),
            "{printed} should report itself writable"
        );
    }
}

#[test]
fn a_range_between_two_numbered_addresses_is_still_a_range() {
    // The reason the separator exists. Nothing here may regress: spec.md §4.2
    // writes a span exactly this way, and the link importer resolves millions
    // of them.
    for (written, from, to) in [
        ("girsa:bavli/berakhot/2a:1-2b:4", "2a:1", "2b:4"),
        ("girsa:shulchan-arukh/orach-chayim/1:1-3:1", "1:1", "3:1"),
        ("girsa:torah/exodus/1:1-6:1", "1:1", "6:1"),
    ] {
        let r: Ref = written.parse().unwrap_or_else(|e| panic!("{written}: {e}"));
        assert!(r.is_span(), "{written} stopped being a span");
        assert_eq!(r.from().to_string(), from, "{written}");
        assert_eq!(r.to().map(ToString::to_string).as_deref(), Some(to));
        assert_eq!(r.to_string(), written);
    }
}

#[test]
fn a_span_whose_end_is_written_short_still_reads_as_a_span() {
    // `Arakhin 33b:21-22` — two lines of one daf. The end is completed against
    // the start later, in the index; here it only has to survive parsing.
    let r: Ref = "girsa:bavli/arakhin/33b:21-22".parse().expect("parses");
    assert!(r.is_span());
    assert_eq!(r.to().map(ToString::to_string).as_deref(), Some("22"));
}

#[test]
fn a_level_that_cannot_be_written_down_says_so_rather_than_lying() {
    // `1-2` as a single level is unrepresentable: written down it is the range
    // from 1 to 2, and there is no escape in the grammar. A caller holding one
    // has to be able to find that out — silently printing it is how a wrong ref
    // gets into a printed sefer.
    let unwritable = Ref::point(
        vec!["work".into()],
        Address::new(vec![Level::canonical("1-2")]),
    );
    assert!(!unwritable.is_well_formed());

    for bad in ["a/b", "a:b", "a#b", ""] {
        let r = Ref::point(
            vec!["work".into()],
            Address::new(vec![Level::canonical(bad)]),
        );
        assert!(!r.is_well_formed(), "a level of {bad:?} is not writable");
    }
}

#[test]
fn the_ordinary_refs_are_all_writable() {
    for written in [
        "girsa:shulchan-arukh/orach-chayim/1:1",
        "girsa:bavli/berakhot/2a:1-2b:4",
        "girsa:mishnah-berurah/121:3",
        "girsa:tur/orach-chayim:240:1",
    ] {
        let r: Ref = written.parse().unwrap_or_else(|e| panic!("{written}: {e}"));
        assert!(r.is_well_formed(), "{written}");
        assert_eq!(r.to_string(), written);
    }
}
