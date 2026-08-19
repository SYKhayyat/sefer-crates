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
//! * Ksav → Girsa, it carried `{path, name, forget}` and meant **a document is
//!   saved here, put it in the registry** (or take it out).
//! * Girsa → Ksav, it carried `{name, text}` and meant **take this document**.
//!
//! One name, two errands, one shared crate, and nothing on either side that
//! said so.
//!
//! # The rename, and why it did not need a flag day
//!
//! They are [`girsa::DOCUMENT_SAVED`] and [`ksav::TAKE_DOCUMENT`] now. The
//! direction is in the name, so neither can be read as the other.
//!
//! The reason this was held open as *blocked* for a release is worth writing
//! down, because the reasoning was wrong: a rename looked like it required both
//! servers to move in the same commit, which two repositories cannot do. They
//! do not have to. A path collides only across the seam, never within one
//! server — each of them serves exactly one errand under `/document` and always
//! did. So each side can accept the old name as well as the new one, and each
//! sender can fall back to the old one when the other side answers 404. Any
//! pairing of an old and a new build works, in either combination, and the
//! repositories land whenever they land.
//!
//! [`girsa::LEGACY_DOCUMENT`] and [`ksav::LEGACY_DOCUMENT`] are that
//! compatibility, kept here so the eventual deletion is also one edit. They can
//! go once no build old enough to speak them is installed anywhere — the
//! senders' fallback first, then the servers' acceptance, in that order and not
//! the other, since a sender that has stopped falling back is harmless and a
//! server that has stopped accepting is not.

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
    /// Not [`super::ksav::TAKE_DOCUMENT`], which is the other direction and a
    /// different errand. See the module note.
    pub const DOCUMENT_SAVED: &str = "/document-saved";

    /// What [`DOCUMENT_SAVED`] was called before the direction was in the name.
    ///
    /// **Accepted, never sent.** A Ksav built before the rename still calls it,
    /// and a save must not fail because the two applications updated on
    /// different days.
    pub const LEGACY_DOCUMENT: &str = "/document";

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
    /// Not [`super::girsa::DOCUMENT_SAVED`], which is the other direction and a
    /// different errand. See the module note.
    pub const TAKE_DOCUMENT: &str = "/take-document";

    /// What [`TAKE_DOCUMENT`] was called before the direction was in the name.
    ///
    /// **Accepted, never sent.** See [`super::girsa::LEGACY_DOCUMENT`].
    pub const LEGACY_DOCUMENT: &str = "/document";

    /// `{…SourcePacket}` — put this source into the document being written.
    pub const INSERT: &str = "/insert";
}

/// The pair that used to mean two things under one name, kept as the record
/// that it did and as the list of what a sweep has to check.
///
/// `.0` is what Girsa serves, `.1` is what Ksav serves. They differ now; the
/// test below is what holds them apart.
pub const RENAMED: (&str, &str) = (girsa::DOCUMENT_SAVED, ksav::TAKE_DOCUMENT);

#[cfg(test)]
mod tests {
    use super::*;

    /// The collision is gone, and no other path has taken its place.
    ///
    /// This test was written the other way round — asserting the collision was
    /// real and was the only one — while the rename was thought to need both
    /// servers in one commit. It is the record that the rename happened.
    #[test]
    fn no_path_serves_two_errands_under_one_name() {
        assert_ne!(RENAMED.0, RENAMED.1, "the rename is what this file is for");

        let girsa = [
            girsa::DOCUMENT_SAVED,
            girsa::REFRESH,
            girsa::WHERE_FROM,
            girsa::LINKIFY,
        ];
        let ksav = [ksav::TAKE_DOCUMENT, ksav::INSERT];
        let shared: Vec<&str> = girsa.iter().filter(|p| ksav.contains(p)).copied().collect();
        assert!(
            shared.is_empty(),
            "a path serving two errands under one name: {shared:?}"
        );

        // `/health` is answered by both and is not a collision: it is the same
        // question with the same answer in both directions, which is the whole
        // difference.
        assert!(!girsa.contains(&HEALTH) && !ksav.contains(&HEALTH));
    }

    /// The compatibility name is the *old* name on both sides, and it is not
    /// any live path.
    ///
    /// Both sides accepting one string is safe precisely because a server sees
    /// only its own direction. What would not be safe is the old name
    /// surviving as a live errand somewhere, which is what this asserts against.
    #[test]
    fn the_legacy_name_is_only_a_legacy_name() {
        assert_eq!(girsa::LEGACY_DOCUMENT, ksav::LEGACY_DOCUMENT);
        for live in [
            girsa::DOCUMENT_SAVED,
            girsa::REFRESH,
            girsa::WHERE_FROM,
            girsa::LINKIFY,
            ksav::TAKE_DOCUMENT,
            ksav::INSERT,
            HEALTH,
        ] {
            assert_ne!(live, girsa::LEGACY_DOCUMENT);
        }
    }
}
