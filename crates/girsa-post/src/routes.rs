//! Every path the loopback carries, named once, in the crate both applications
//! compile.
//!
//! # Why a module and not a string in each application
//!
//! The seam is two servers, each serving paths the other calls, and until this
//! module existed the only place the two halves of a path met was a quoted
//! string in each repository. Nothing checked that they were the same string,
//! and nothing recorded which direction a path ran in — which is how
//! `/document` came to mean two unrelated errands:
//!
//! * Ksav → Girsa, it carries `{path, name, forget}` and means **a document is
//!   saved here, put it in the registry** (or take it out).
//! * Girsa → Ksav, it carries `{name, text}` and means **take this document**.
//!
//! One name, two errands, one shared crate, and nothing on either side that
//! said so. Both are below, under names that cannot be confused, with the
//! direction in the name.
//!
//! # The wire strings have not changed
//!
//! [`girsa::DOCUMENT`] and [`ksav::DOCUMENT`] are both still `"/document"`,
//! because a rename is only safe when both servers rename at once and this
//! repository is not both servers. What this module buys today is that the
//! collision is *stated* rather than latent, and that the rename, when the two
//! applications take it together, is one edit here rather than a search across
//! two trees. See [`COLLISION`] for the pair that has to move.

/// Answered by both applications, in both directions: *are you there, and which
/// version*. The one path that is not an errand.
///
/// [`presence`](crate::presence) is the caller.
pub const HEALTH: &str = "/health";

/// The paths **Girsa serves**, which is to say the ones Ksav calls.
pub mod girsa {
    /// `{path, name, forget}` — a document is saved at this path, put it in the
    /// registry, or with `forget` take it out.
    ///
    /// Not [`super::ksav::DOCUMENT`]. See the module note.
    pub const DOCUMENT: &str = "/document";

    /// Re-quote every citation in a document against the corpus as it stands
    /// now.
    ///
    /// One row per citation, **in the order they appear in the document**, and
    /// the caller zips by position — so the row list is total: a citation whose
    /// ref this build cannot read is a row that says so, never a row that is
    /// missing. A missing row re-quotes everything after it from the wrong
    /// place.
    pub const REFRESH: &str = "/refresh";

    /// Which place a `girsa:` ref names, in words.
    pub const WHERE_FROM: &str = "/where-from";

    /// Turn the mareh mekomos in a stretch of text into real citations.
    pub const LINKIFY: &str = "/linkify";
}

/// The paths **Ksav serves**, which is to say the ones Girsa calls.
pub mod ksav {
    /// `{name, text}` — take this document.
    ///
    /// Not [`super::girsa::DOCUMENT`]. See the module note.
    pub const DOCUMENT: &str = "/document";

    /// `{…SourcePacket}` — put this source into the document being written.
    pub const INSERT: &str = "/insert";
}

/// The one pair of paths that means two things, kept here so a sweep can find
/// it and so the rename has somewhere to start.
///
/// When the two applications are ready to rename together, this constant is the
/// list of what moves. A test in either repository can assert against it that
/// the collision is still the only one.
pub const COLLISION: (&str, &str) = (girsa::DOCUMENT, ksav::DOCUMENT);

#[cfg(test)]
mod tests {
    use super::*;

    /// The collision is real, and it is the only one.
    ///
    /// If this ever goes green by the two paths differing, the rename has
    /// happened and this test becomes the record that it did.
    #[test]
    fn document_still_means_two_things_and_nothing_else_does() {
        assert_eq!(COLLISION.0, COLLISION.1, "the rename has not happened yet");

        let girsa = [
            girsa::DOCUMENT,
            girsa::REFRESH,
            girsa::WHERE_FROM,
            girsa::LINKIFY,
        ];
        let ksav = [ksav::DOCUMENT, ksav::INSERT];
        let shared: Vec<&str> = girsa.iter().filter(|p| ksav.contains(p)).copied().collect();
        assert_eq!(
            shared,
            vec![girsa::DOCUMENT],
            "a second path serving two errands under one name, unnamed"
        );

        // `/health` is answered by both and is not a collision: it is the same
        // question with the same answer in both directions, which is the whole
        // difference.
        assert!(!girsa.contains(&HEALTH) && !ksav.contains(&HEALTH));
    }
}
