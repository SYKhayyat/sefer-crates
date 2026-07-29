//! Canonical refs, citation parsing, offline resolution, and the redirect table.
//!
//! A ref points at a *span*, because a quote is a range. Refs travel between
//! Girsa and Ksav and get stored inside Ksav documents, so they have to survive
//! corpus updates — that is the promise the two-app system rests on.
//!
//! # The one rule
//!
//! **Ambiguity is surfaced as a choice, never guessed.** A wrong ref is worse
//! than no ref, everywhere in this system, because a wrong ref does not look
//! wrong: it resolves, it opens a page, and the page is the wrong one. If it
//! has been written into a Ksav document, it is wrong in a printed sefer.
//!
//! ```
//! use girsa_ref::{resolve, Lexicon, Resolution, Work};
//!
//! let mut lexicon = Lexicon::default();
//! lexicon.add(
//!     Work { slug: "shulchan-arukh/orach-chayim".into(),
//!            he_title: "שולחן ערוך, אורח חיים".into(),
//!            en_title: "Shulchan Arukh, Orach Chayim".into() },
//!     &["שו\"ע או\"ח", "או\"ח"],
//! );
//! lexicon.add(
//!     Work { slug: "tur/orach-chayim".into(),
//!            he_title: "טור, אורח חיים".into(),
//!            en_title: "Tur, Orach Chayim".into() },
//!     &["או\"ח"],
//! );
//!
//! // Unambiguous: one sefer, one place.
//! let r = resolve(&lexicon, "שו\"ע או\"ח א' א'");
//! assert_eq!(r.exact().map(ToString::to_string).as_deref(),
//!            Some("girsa:shulchan-arukh/orach-chayim/1:1"));
//!
//! // Ambiguous: two seforim genuinely go by או"ח, so the reader chooses.
//! assert!(matches!(resolve(&lexicon, "או\"ח א'"), Resolution::Ambiguous(_)));
//! ```

#![doc(html_root_url = "https://docs.rs/girsa-ref/0.4.0")]

pub mod address;
pub mod daf;
pub mod lexicon;
pub mod numerals;
pub mod redirect;
pub mod reference;
pub mod resolve;

pub use address::{Address, Level};
pub use lexicon::{Lexicon, Work};
pub use redirect::RedirectTable;
pub use reference::{Ref, RefError};
pub use resolve::{resolve, resolve_in_context, Context, Resolution};

/// The scheme every canonical ref is written under: `girsa:...`.
pub const SCHEME: &str = "girsa";
