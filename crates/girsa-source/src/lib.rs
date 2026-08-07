//! The Source Packet — the wire contract between Girsa and Ksav.
//!
//! ```jsonc
//! { "schema":  1,
//!   "ref":     "girsa:shulchan-arukh/orach-chayim/1:1",
//!   "display": "שו\"ע או\"ח סימן א' סעיף א'",
//!   "text":    "יתגבר כארי לעמוד בבוקר לעבודת בוראו…",
//!   "nikud":   false,
//!   "lang":    "he",
//!   "version": { "edition": "…", "license": "CC-BY", "provenance": "…" },
//!   "note":    "my margin note, if attached" }
//! ```
//!
//! # Why this is a crate and not a documented JSON shape
//!
//! Both applications link against this one definition, so the app that
//! *produces* a citation and the app that *prints* it cannot drift into
//! disagreeing about what one is (spec.md §10.1). A documented shape drifts in
//! silence; a shared type drifts at compile time or not at all.
//!
//! How far that goes is worth being exact about, because this note used to
//! overstate it — it claimed *"adding a field is a compile error on the side
//! that ignores it"* thirty lines above [`PACKET_SCHEMA_VERSION`] saying an
//! optional field needs no bump, and both cannot be true.
//!
//! Adding a **required** field is a compile error, at every site that builds a
//! packet. Adding an **optional** one is not: serde fills it in, deliberately,
//! and that is exactly what lets a newer producer keep talking to an older
//! consumer instead of breaking it. The guard for the case the compiler cannot
//! see is the schema version, checked before any field is read — see below.
//! That is the stronger of the two mechanisms and the one to reason about.
//!
//! # Why the ref is stored and not just the printed string
//!
//! `display` is what the reader sees; [`SourcePacket::reference`] is what the
//! document keeps. Storing the ref is what makes a citation alive: a whole
//! sefer can be switched from abbreviated to full-form citations, or every
//! quote regenerated against a corrected edition (spec.md §7), without touching
//! the prose. No paste-based workflow can ever do that.
//!
//! # Why the schema version is checked before anything else
//!
//! Three repositories means a mismatched pair is possible. A packet from a
//! newer Girsa must fail *loudly* in an older Ksav, at the handshake — because
//! the alternative is that serde fills in defaults for the fields it does not
//! know, and the citation renders looking perfectly reasonable and slightly
//! wrong. In a printed sefer.

#![doc(html_root_url = "https://docs.rs/girsa-source/0.5.0")]

use serde::{Deserialize, Serialize};

pub use girsa_ref::Ref;

/// Wire-format version of the Source Packet.
///
/// Bumped when a change would make an older consumer misread a packet. Adding
/// an optional field does not need a bump; changing what an existing field
/// means does.
pub const PACKET_SCHEMA_VERSION: u32 = 1;

/// The MIME type the layered clipboard puts the packet down under (spec.md
/// §10.2), alongside `text/plain` and `text/html`.
pub const CLIPBOARD_MIME: &str = "application/x-girsa-source+json";

/// Which edition a quote came from, and under what terms.
///
/// Carried on every packet because it costs nothing now and is the only thing
/// preserving the option to distribute publicly later (spec.md §13). A sefer
/// typeset from quotes whose provenance was dropped cannot be un-dropped.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Version {
    /// The edition as the corpus names it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub edition: String,
    /// The licence the text is under — `CC-BY`, `Public Domain`, `Unknown`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub license: String,
    /// Where it came from — which project, which scan, which typist.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub provenance: String,
}

/// Which characters of the quoted place this quote actually is.
///
/// # Why the ref is not enough, and what it cost to find that out
///
/// A ref names **places** — `שו"ע או"ח א':א'`, or a span from one to another.
/// A reader highlighting half a se'if and pressing Ctrl+C gets exactly the
/// words they highlighted, and the packet said so in `text` and said nothing
/// about *which* words those were.
///
/// So the promise that makes the whole two-application system worth building —
/// regenerate every quote against a corrected edition without touching the
/// prose (spec.md §7, §10.2) — regenerated **the whole se'if** for any partial
/// selection. Two sentences in the README, both true, contradicting each other
/// at the regeneration step: *only the highlighted part goes*, and *citations
/// stay alive*.
///
/// # Characters, of what
///
/// Characters — not bytes — of the segment's text **as it was shown**: markup
/// off, and nikud on or off according to what the reader was reading. That is
/// the only offset the two ends can agree about, because it is the one the
/// reader was looking at when they dragged.
///
/// `from` counts into the **first** segment the ref names; `to` counts into the
/// **last**, exclusive, and is `None` for *to the end of it*. For a quote of
/// one segment they are the two ends of one line; for a span they are the two
/// ragged ends of it, and everything between is whole.
///
/// # Why optional and why now
///
/// Optional so that a packet written before this field existed still reads, and
/// so that an older Ksav ignores a newer Girsa's — which is what the module
/// note above says optional fields are for.
///
/// Now, because the cost was rising with every ref already written into every
/// document. Adding it later to the **ref syntax** would make old and new refs
/// the same string shape with two different meanings; adding it later as a
/// packet field, once documents are full of ranges that were never recorded,
/// means old quotes silently regenerate whole while new ones do not. Carrying
/// it is one optional field on a struct both applications already compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    /// Where the quote starts in the first place the ref names, in characters.
    #[serde(default)]
    pub from: usize,
    /// Where it ends in the last one, exclusive. `None` is *to the end*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<usize>,
}

impl Range {
    /// The whole of what the ref names.
    ///
    /// **Not the same as `None`.** `None` is a packet that never recorded a
    /// range — written by a Girsa older than this field — and the only honest
    /// thing a consumer can do with one is regenerate whole. `Some(Range::all())`
    /// is a reader who selected the whole place, and regenerating that whole is
    /// what they asked for.
    #[must_use]
    pub const fn all() -> Self {
        Self { from: 0, to: None }
    }

    /// Whether this names the whole of what the ref names.
    #[must_use]
    pub const fn is_all(&self) -> bool {
        self.from == 0 && self.to.is_none()
    }
}

/// A source, on its way from the library to the document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePacket {
    /// Wire-format version. First field, and checked before anything else.
    pub schema: u32,

    /// The canonical ref. **This is what the document stores.**
    ///
    /// Serialized as its text, so a packet stays readable and a version of
    /// Ksav that only wants to print something never has to understand refs.
    #[serde(rename = "ref")]
    pub reference: String,

    /// How the citation is printed — `שו"ע או"ח סימן א' סעיף א'`.
    pub display: String,

    /// The text itself.
    pub text: String,

    /// Whether `text` carries nikud, so the receiving document can decide
    /// whether to keep it rather than guessing from the characters.
    #[serde(default)]
    pub nikud: bool,

    /// `he`, `arc`, `en`.
    #[serde(default = "default_lang")]
    pub lang: String,

    /// Which edition, and under what terms.
    #[serde(default)]
    pub version: Version,

    /// A margin note travelling with the source, if one was attached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,

    /// Which characters of the quoted place this is — see [`Range`].
    ///
    /// `None` on a packet written before the field existed, which is the one
    /// case a consumer must regenerate whole for, because it is the one case
    /// where nobody knows what was highlighted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
}

fn default_lang() -> String {
    "he".to_string()
}

/// What can go wrong at the handshake.
#[derive(Debug, thiserror::Error)]
pub enum PacketError {
    /// The packet came from a newer Girsa than this build understands.
    ///
    /// Deliberately fatal. Reading it anyway means serde supplies defaults for
    /// the fields this build does not know about, and the citation renders
    /// looking reasonable and being wrong.
    #[error(
        "this packet is schema v{found}; this build understands up to v{supported}. \
         Update Ksav, or send from a matching Girsa."
    )]
    SchemaTooNew { found: u32, supported: u32 },

    /// The packet predates a change this build depends on.
    #[error("this packet is schema v{found}, which is no longer supported (minimum v{minimum})")]
    SchemaTooOld { found: u32, minimum: u32 },

    #[error("not a Source Packet: {0}")]
    Malformed(#[from] serde_json::Error),

    #[error("the ref `{0}` is not a girsa ref")]
    BadRef(String),
}

/// The oldest schema this build will read.
const MINIMUM_SCHEMA: u32 = 1;

impl SourcePacket {
    /// Build a packet for the current schema.
    #[must_use]
    pub fn new(reference: &Ref, display: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            schema: PACKET_SCHEMA_VERSION,
            reference: reference.to_string(),
            display: display.into(),
            text: text.into(),
            nikud: false,
            lang: default_lang(),
            version: Version::default(),
            note: None,
            // `Range::all()` and not `None`. A packet this build writes always
            // knows what was selected, and *the whole place* is a thing the
            // reader chose. `None` is reserved for a packet written before the
            // field existed, which is the one case a consumer cannot know
            // about and must regenerate whole for.
            range: Some(Range::all()),
        }
    }

    /// The same, for a quote of part of what the ref names.
    #[must_use]
    pub fn part(
        reference: &Ref,
        display: impl Into<String>,
        text: impl Into<String>,
        range: Range,
    ) -> Self {
        Self {
            range: Some(range),
            ..Self::new(reference, display, text)
        }
    }

    /// The ref, parsed.
    ///
    /// Fallible on purpose: the string arrived over a wire, and a ref that does
    /// not parse must be reported rather than silently treated as a work with a
    /// strange name.
    pub fn reference(&self) -> Result<Ref, PacketError> {
        self.reference
            .parse()
            .map_err(|_| PacketError::BadRef(self.reference.clone()))
    }

    /// Serialize for the clipboard or the loopback transport.
    pub fn to_json(&self) -> Result<String, PacketError> {
        Ok(serde_json::to_string(self)?)
    }

    /// Read a packet, **checking the schema version first**.
    ///
    /// This is the handshake. A packet from a newer Girsa fails here, with a
    /// message naming both versions, instead of deserializing into something
    /// that looks fine.
    pub fn from_json(json: &str) -> Result<Self, PacketError> {
        // Two stages, deliberately. Deserializing straight into `SourcePacket`
        // would apply every `#[serde(default)]` before anyone looked at the
        // version, so a v2 packet whose `text` moved somewhere else would
        // arrive with an empty quote and no complaint.
        #[derive(Deserialize)]
        struct Envelope {
            schema: u32,
        }

        let Envelope { schema } = serde_json::from_str(json)?;
        if schema > PACKET_SCHEMA_VERSION {
            return Err(PacketError::SchemaTooNew {
                found: schema,
                supported: PACKET_SCHEMA_VERSION,
            });
        }
        if schema < MINIMUM_SCHEMA {
            return Err(PacketError::SchemaTooOld {
                found: schema,
                minimum: MINIMUM_SCHEMA,
            });
        }

        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace bans these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn packet() -> SourcePacket {
        let r: Ref = "girsa:shulchan-arukh/orach-chayim/1:1"
            .parse()
            .expect("the ref parses");
        let mut p = SourcePacket::new(
            &r,
            "שו\"ע או\"ח סימן א' סעיף א'",
            "יתגבר כארי לעמוד בבוקר לעבודת בוראו",
        );
        p.version = Version {
            edition: "Torat Emet 357".into(),
            license: "Public Domain".into(),
            provenance: "Sefaria".into(),
        };
        p
    }

    #[test]
    fn a_packet_survives_a_round_trip() {
        let original = packet();
        let json = original.to_json().expect("serializes");
        let back = SourcePacket::from_json(&json).expect("deserializes");
        assert_eq!(original, back);
    }

    #[test]
    fn the_ref_survives_as_a_ref_and_not_only_as_text() {
        // The whole point of storing the ref: it has to still be a ref on the
        // far side, or a document cannot regenerate its quotes.
        let back = SourcePacket::from_json(&packet().to_json().expect("serializes"))
            .expect("deserializes");
        let r = back.reference().expect("the ref parses on the far side");
        assert_eq!(r.work_slug(), "shulchan-arukh/orach-chayim");
        assert_eq!(r.from().to_string(), "1:1");
    }

    #[test]
    fn a_packet_from_a_newer_girsa_fails_loudly_rather_than_half_arriving() {
        // The test that justifies the schema field. Without the check, serde
        // fills in defaults for what it does not recognise and the citation
        // renders looking reasonable and being wrong — in a printed sefer.
        let json = r#"{"schema":99,"ref":"girsa:x/1:1","display":"d","text":"t"}"#;
        match SourcePacket::from_json(json) {
            Err(PacketError::SchemaTooNew { found, supported }) => {
                assert_eq!(found, 99);
                assert_eq!(supported, PACKET_SCHEMA_VERSION);
            }
            other => panic!("expected a loud refusal, got {other:?}"),
        }
    }

    #[test]
    fn the_refusal_names_both_versions_so_it_can_be_acted_on() {
        let json = r#"{"schema":99,"ref":"girsa:x/1:1","display":"d","text":"t"}"#;
        let message = SourcePacket::from_json(json)
            .expect_err("must fail")
            .to_string();
        assert!(message.contains("99"), "{message}");
        assert!(
            message.contains(&PACKET_SCHEMA_VERSION.to_string()),
            "{message}"
        );
    }

    #[test]
    fn a_newer_packet_is_refused_even_when_every_field_it_has_is_readable() {
        // The dangerous case: a v2 packet that happens to be shaped like a v1
        // one. Every field deserializes; the meaning of one of them changed.
        let mut json: serde_json::Value =
            serde_json::from_str(&packet().to_json().expect("serializes")).expect("valid json");
        json["schema"] = serde_json::json!(PACKET_SCHEMA_VERSION + 1);
        assert!(SourcePacket::from_json(&json.to_string()).is_err());
    }

    #[test]
    fn a_packet_with_no_schema_field_at_all_is_not_a_packet() {
        let json = r#"{"ref":"girsa:x/1:1","display":"d","text":"t"}"#;
        assert!(matches!(
            SourcePacket::from_json(json),
            Err(PacketError::Malformed(_))
        ));
    }

    #[test]
    fn provenance_travels_with_the_quote() {
        // spec.md §13: carrying it costs nothing now and is the only thing
        // preserving the option to distribute publicly later.
        let back = SourcePacket::from_json(&packet().to_json().expect("serializes"))
            .expect("deserializes");
        assert_eq!(back.version.license, "Public Domain");
        assert_eq!(back.version.provenance, "Sefaria");
    }

    #[test]
    fn an_optional_field_that_was_not_set_does_not_appear_on_the_wire() {
        // Keeps the clipboard flavour small, and keeps a diff of two packets
        // readable.
        let json = packet().to_json().expect("serializes");
        assert!(!json.contains("note"), "{json}");
    }

    #[test]
    fn a_note_travels_when_there_is_one() {
        let mut p = packet();
        p.note = Some("צריך עיון".into());
        let back =
            SourcePacket::from_json(&p.to_json().expect("serializes")).expect("deserializes");
        assert_eq!(back.note.as_deref(), Some("צריך עיון"));
    }

    #[test]
    fn a_ref_that_does_not_parse_is_reported_rather_than_treated_as_a_work() {
        let json = r#"{"schema":1,"ref":"Berakhot 2a","display":"d","text":"t"}"#;
        let p = SourcePacket::from_json(json).expect("the packet itself is well formed");
        assert!(matches!(p.reference(), Err(PacketError::BadRef(_))));
    }

    #[test]
    fn a_packet_written_before_the_range_existed_still_reads() {
        // The point of an optional field, and the module note's own argument
        // for why adding one is not a break. A packet from a Girsa that had
        // never heard of `range` reads, and says `None` — which is *nobody
        // knows what was highlighted*, not *the whole thing was*.
        let old = r#"{"schema":1,"ref":"girsa:tur/1","display":"טור א'","text":"…"}"#;
        let packet = SourcePacket::from_json(old).expect("it reads");
        assert_eq!(packet.range, None);
    }

    #[test]
    fn the_whole_place_and_nobody_knows_are_two_different_answers() {
        // The distinction the whole field exists for. Regenerating a quote
        // whose range is `None` has to take the whole place, because that is
        // all anybody can know; regenerating one whose range is `all()` takes
        // the whole place because that is what the reader selected. Same
        // outcome, different reason, and only one of them is a guess.
        let reference: Ref = "girsa:tur/1".parse().expect("a ref");
        let whole = SourcePacket::new(&reference, "טור א'", "…");
        assert_eq!(whole.range, Some(Range::all()));
        assert!(whole.range.is_some_and(|r| r.is_all()));

        let part = SourcePacket::part(
            &reference,
            "טור א'",
            "…",
            Range {
                from: 4,
                to: Some(19),
            },
        );
        assert!(part.range.is_some_and(|r| !r.is_all()));
    }

    #[test]
    fn a_range_survives_the_wire() {
        let reference: Ref = "girsa:tur/1".parse().expect("a ref");
        let sent = SourcePacket::part(
            &reference,
            "טור א'",
            "…",
            Range {
                from: 4,
                to: Some(19),
            },
        );
        let json = serde_json::to_string(&sent).expect("it writes");
        assert!(json.contains("\"range\""), "{json}");
        let back = SourcePacket::from_json(&json).expect("it reads back");
        assert_eq!(back.range, sent.range);
    }

    #[test]
    fn a_whole_selection_costs_one_key_on_the_wire() {
        // `to` is skipped when it is `None`, so the common case — a whole
        // se'if — is `"range":{"from":0}` and not a second way of spelling
        // nothing.
        let reference: Ref = "girsa:tur/1".parse().expect("a ref");
        let json = serde_json::to_string(&SourcePacket::new(&reference, "טור א'", "…"))
            .expect("it writes");
        assert!(json.contains(r#""range":{"from":0}"#), "{json}");
    }
}
