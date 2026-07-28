//! Citation formatting — one implementation, compiled into both applications.
//!
//! The app that *produces* citations and the app that *prints* them cannot be
//! allowed to disagree; that is precisely the class of bug that would destroy
//! trust in the pairing. So there is one formatter and both link against it.
//!
//! Filled in alongside W4/W15. This is the W1 scaffold.

/// How a citation is rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CiteStyle {
    /// `שו"ע או"ח סימן א' סעיף א'` — how it is written in a sefer.
    HebrewFull,
    /// `שו"ע או"ח א', א'` — how it is written in a footnote.
    HebrewShort,
    /// `Shulchan Arukh, Orach Chayim 1:1` — Sefaria-compatible.
    English,
}
