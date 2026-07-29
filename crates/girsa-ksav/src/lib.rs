//! Writing real Ksav markup — the document language, in the shared repository.
//!
//! # Why this is not inside Ksav
//!
//! spec.md §10.3 is blunt about it: *lightweight means the UI, not the format.
//! The buffer writes real Ksav/Typst markup from the first keystroke. If it
//! invents its own note format, we get exactly the drift that embedding was
//! meant to prevent and the handoff becomes lossy.*
//!
//! Girsa has a Ksav buffer in it (W17). For that buffer to write real Ksav, the
//! two applications have to agree on what a quote block *is* — and an agreement
//! in prose between two repositories is the thing this whole shared repository
//! exists to replace. So the markup writer lives here, both applications
//! compile it, and Ksav's own `source` module is a wrapper around it.
//!
//! Every construct below is an existing command in Ksav's `typst/ksav.typ`:
//!
//! | | |
//! |---|---|
//! | [`quote_block`] | `#ציטוט[…]` — a block quote |
//! | [`mekor`] | `#מראה_מקום[…]` — the citation as a footnote, which is what a sefer does |
//! | [`inline_citation`] | `#מקור[…]` — the citation inline, in small grey type |
//! | [`editor_note`] | `#הערת_עורך[…]` — a note *about* the text, never part of it |
//! | [`heading`] | `#כותרת1[…]` … `#כותרת6[…]` |
//!
//! # Escaping is the whole of the danger
//!
//! A quote from a sefer is arbitrary text and routinely contains `#` (as a
//! numeral sign), `[`, `]` and `\`. Unescaped, a `[` opens a content block that
//! never closes, and Typst cannot report it until it reaches end of file —
//! thousands of characters from the quote, with the preview blank and nothing
//! pointing at what caused it.

#![doc(html_root_url = "https://docs.rs/girsa-ksav/0.4.0")]

pub mod read;

pub use read::{read, Block, NoteKind};

use girsa_source::SourcePacket;

/// How the citation is placed relative to the quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CitationPlacement {
    /// A mekor footnote — `#מראה_מקום[…]`. What a sefer does.
    #[default]
    Mekor,
    /// Inline after the quote, in small type — `#מקור[…]`.
    Inline,
}

/// Turn a packet into real Ksav markup.
///
/// What lands in the buffer is a document, not an import format that has to be
/// converted later.
///
/// # A source with no words is a mareh makom
///
/// A packet whose `text` is empty is not a failed quote — it is a **citation on
/// its own**, which is what a page of a scan is until it has been OCR'd
/// (spec.md §6.3, W25): the daf is citable long before anything can read the
/// words off it. Written as a quote it would arrive in the document as
/// `#ציטוט[]`, an empty block that looks exactly like a paste that went wrong,
/// and the writer has no way to tell which end it happened at. So it arrives as
/// the mekor alone, which is the thing that was actually sent.
#[must_use]
pub fn to_ksav(packet: &SourcePacket, placement: CitationPlacement) -> String {
    let mut out = String::with_capacity(packet.text.len() + packet.display.len() + 64);
    let quoting = !packet.text.trim().is_empty();
    if quoting {
        out.push_str(&quote_block(&packet.text));
    }
    match placement {
        CitationPlacement::Mekor => out.push_str(&mekor(&packet.display, Some(&packet.reference))),
        CitationPlacement::Inline if quoting => {
            out.push(' ');
            out.push_str(&inline_citation(&packet.display));
        }
        // With nothing in front of it there is nothing for a citation to be
        // *inline* with, and `#מקור[…]` alone is a source note floating in the
        // prose. A mareh makom is what a sefer writes here.
        CitationPlacement::Inline => out.push_str(&mekor(&packet.display, Some(&packet.reference))),
    }
    if let Some(note) = &packet.note {
        // A margin note that travelled with the source is the writer's own
        // words, so it arrives as an editor's comment rather than as part of
        // the quote.
        out.push_str(&editor_note(note));
    }
    out.push('\n');
    out
}

/// `#ציטוט[…]` — a block quote.
#[must_use]
pub fn quote_block(text: &str) -> String {
    format!("#ציטוט[{}]", escape(text))
}

/// `#מראה_מקום(מקור: "girsa:…")[…]` — the citation as a footnote, **carrying
/// the ref**.
///
/// The ref is stored and not printed, and it is the whole of the pairing's
/// promise (spec.md §10.2): a document that keeps the *place* can be re-printed
/// in another style, or have its quotes regenerated against a corrected
/// edition, without touching a word of the prose. A document that keeps only
/// the printed string can do neither — and until this argument existed, that is
/// exactly what Girsa was writing.
///
/// It is also what `#מראה_מקומות()` collects into a source list at the back:
/// the refs are already in the document, so a mareh mekomos is a sort and a
/// print.
#[must_use]
pub fn mekor(citation: &str, reference: Option<&str>) -> String {
    match reference {
        Some(reference) => format!(
            "#מראה_מקום(מקור: \"{}\")[{}]",
            in_a_string(reference),
            escape(citation)
        ),
        None => format!("#מראה_מקום[{}]", escape(citation)),
    }
}

/// Escaping for a Typst **string**, which is not the same as for markup.
///
/// A ref carries `:` and `/` and neither matters here; what would end the
/// string early is a quote mark, and what would eat the next character is a
/// backslash. Refs contain neither today — and "today" is not a thing to rely
/// on when the alternative is two lines.
fn in_a_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// `#מקור_חי(מקור: "girsa:…")[…]` — a citation in the flow of the prose that
/// keeps its ref.
///
/// What linkify writes (spec.md §10.5): the words are printed exactly as they
/// were written and the ref rides underneath, so the citation counts in the
/// mareh mekomos and — because Ksav renders it as a `link` — **opens the page
/// it names when it is clicked in a compiled PDF** (§10.6).
#[must_use]
pub fn live_citation(citation: &str, reference: &str) -> String {
    format!(
        "#מקור_חי(מקור: \"{}\")[{}]",
        in_a_string(reference),
        escape(citation)
    )
}

/// `#מקור[…]` — the citation inline, in small grey type.
#[must_use]
pub fn inline_citation(citation: &str) -> String {
    format!("#מקור[{}]", escape(citation))
}

/// `#הערת_עורך[…]` — a note *about* the text.
#[must_use]
pub fn editor_note(note: &str) -> String {
    format!("#הערת_עורך[{}]", escape(note))
}

/// `#כותרת1[…]` … `#כותרת6[…]`.
#[must_use]
pub fn heading(level: u8, text: &str) -> String {
    format!("#כותרת{}[{}]\n", level.clamp(1, 6), escape(text))
}

/// The words of a Ksav document, with the commands taken off.
///
/// # What this is and is not
///
/// It is a **reading**, not an evaluation: Typst is the only thing that can
/// say what a document *renders* as, and running the compiler to put a
/// paragraph on a shelf would put the whole engine inside the library.
///
/// This is [`read`] rendered flat, and is deliberately not a second parser.
/// The one it replaced took the command names and their arguments off and kept
/// what was between the brackets, which lost a document's lists and tables
/// **entirely** — they live in the arguments — and spliced its footnotes into
/// the middle of the sentences that carried them. See [`read`] for the
/// measurements.
///
/// The file stays the truth, as everywhere else here. This is what a reader
/// sees; the `.ksav` beside it is the document.
#[must_use]
pub fn to_text(markup: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for block in read(markup) {
        match block {
            Block::Heading { text, .. } | Block::Paragraph(text) | Block::Quote(text) => {
                out.push(text);
            }
            Block::Item { ordinal, text, .. } => out.push(match ordinal {
                Some(n) => format!("{n}. {text}"),
                None => text,
            }),
            // Tab-separated, which is what a cell boundary is in every plain
            // rendering of a table and what a reader can still see columns in.
            Block::Row { cells, .. } => out.push(cells.join("	")),
            Block::Note { marker, text, .. } => out.push(format!("{marker}. {text}")),
        }
    }
    out.join(
        "
",
    )
}

/// Every ref the document stores.
///
/// The other half of `מקור:` (see [`mekor`]): because the refs are *in* the
/// document, "where did I use this?" is a scan rather than a guess, and a
/// mareh mekomos is a sort and a print (spec.md §10.4).
#[must_use]
pub fn refs_in(markup: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = markup;
    while let Some(at) = rest.find("מקור:") {
        rest = &rest[at + "מקור:".len()..];
        let Some(open) = rest.find('"') else { break };
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else { break };
        let found = &after[..close];
        if found.starts_with("girsa:") && !out.iter().any(|r| r == found) {
            out.push(found.to_string());
        }
        rest = &after[close + 1..];
    }
    out
}

/// Escape the characters Typst reads as markup.
///
/// See the module note: an unclosed `[` from an unescaped quote is reported at
/// end of file, nowhere near the quote that caused it.
#[must_use]
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(
            c,
            '#' | '[' | ']' | '\\' | '$' | '*' | '_' | '<' | '>' | '@'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use girsa_source::Ref;

    fn packet() -> SourcePacket {
        let r: Ref = "girsa:shulchan-arukh/orach-chayim/1:1"
            .parse()
            .expect("the ref parses");
        SourcePacket::new(
            &r,
            "שו\"ע או\"ח סימן א' סעיף א'",
            "יתגבר כארי לעמוד בבוקר לעבודת בוראו",
        )
    }

    #[test]
    fn a_source_becomes_a_quote_with_a_mekor() {
        let markup = to_ksav(&packet(), CitationPlacement::Mekor);
        assert!(markup.contains("#ציטוט["));
        assert!(markup.contains("#מראה_מקום("));
        assert!(markup.contains("יתגבר כארי"));
    }

    #[test]
    fn the_document_keeps_the_place_and_not_only_the_printed_string() {
        // spec.md §10.2, and for three work orders this was quietly false:
        // the markup carried `#מראה_מקום[שו"ע או"ח סימן א' סעיף א']` and the
        // ref went nowhere. A document like that cannot be re-styled, cannot
        // have its quotes regenerated, and cannot answer *where did I use
        // this* — every one of which the spec promises.
        let markup = to_ksav(&packet(), CitationPlacement::Mekor);
        assert!(
            markup.contains("מקור: \"girsa:shulchan-arukh/orach-chayim/1:1\""),
            "{markup}"
        );
    }

    #[test]
    fn a_source_with_no_words_arrives_as_a_mareh_makom_and_not_as_an_empty_quote() {
        // A page of a scan (W25). The daf is citable the moment somebody says
        // which page it is on, and there is nothing to quote until the scan has
        // been OCR'd — so what is sent is a mekor, and `#ציטוט[]` in the middle
        // of somebody's chaburah reads as a paste that failed.
        let r: Ref = "girsa:bavli/berakhot/2a".parse().expect("the ref parses");
        let page = SourcePacket::new(&r, "ברכות ב.", "");
        for placement in [CitationPlacement::Mekor, CitationPlacement::Inline] {
            let markup = to_ksav(&page, placement);
            assert!(!markup.contains("#ציטוט"), "{markup}");
            assert!(markup.contains("#מראה_מקום("), "{markup}");
            assert!(markup.contains("girsa:bavli/berakhot/2a"), "{markup}");
        }
        // And a quote that has words still has both halves.
        assert!(to_ksav(&packet(), CitationPlacement::Mekor).contains("#ציטוט["));
    }

    #[test]
    fn a_citation_with_no_ref_behind_it_still_prints() {
        // Linkified prose and a citation typed by hand have no ref until
        // somebody resolves one. The command takes both shapes rather than
        // making the caller invent a ref to satisfy it.
        assert_eq!(
            mekor("שו\"ע או\"ח א' א'", None),
            "#מראה_מקום[שו\"ע או\"ח א' א']"
        );
    }

    #[test]
    fn a_quote_carrying_typst_markup_does_not_open_a_block_that_never_closes() {
        // Real seforim contain `[`, `#` and `*`. Unescaped, the error lands at
        // end of file with the preview blank.
        let mut p = packet();
        p.text = "וכתב [הרמב\"ם] #ד' *כאן* עיין _שם_".into();
        let markup = to_ksav(&p, CitationPlacement::Mekor);
        assert!(markup.contains("\\[הרמב"));
        assert!(markup.contains("\\#ד'"));
        // The property, rather than a count of characters: every bracket Typst
        // *reads* is matched. A count would pass while the escaped ones were
        // doing the balancing, which is the bug this guards against.
        assert_eq!(open_blocks(&markup), 0, "{markup}");
    }

    /// How many content blocks are still open — counting only the brackets
    /// Typst reads, which is to say the unescaped ones.
    fn open_blocks(markup: &str) -> i32 {
        let mut depth = 0;
        let mut escaped = false;
        for c in markup.chars() {
            if escaped {
                escaped = false;
                continue;
            }
            match c {
                '\\' => escaped = true,
                '[' => depth += 1,
                ']' => depth -= 1,
                _ => {}
            }
        }
        depth
    }

    #[test]
    fn a_note_is_a_comment_about_the_text_and_not_part_of_the_quote() {
        let mut p = packet();
        p.note = Some("צריך עיון".into());
        let markup = to_ksav(&p, CitationPlacement::Mekor);
        assert!(markup.contains("#הערת_עורך[צריך עיון]"));
        // Inside the quote block it is not.
        let quote = markup.split("#מראה_מקום").next().unwrap_or_default();
        assert!(!quote.contains("צריך עיון"));
    }

    #[test]
    fn a_linkified_citation_prints_the_same_words_and_keeps_the_place() {
        let markup = live_citation(
            "שו\"ע או\"ח סימן א' סעיף ג'",
            "girsa:shulchan-arukh/orach-chayim/1:3",
        );
        assert!(markup.contains("שו\"ע או\"ח סימן א' סעיף ג'"), "{markup}");
        assert_eq!(refs_in(&markup), ["girsa:shulchan-arukh/orach-chayim/1:3"]);
        // And it reads back as the words, with nothing of the markup in them.
        assert_eq!(to_text(&markup).trim(), "שו\"ע או\"ח סימן א' סעיף ג'");
    }

    #[test]
    fn a_document_reads_back_as_the_words_somebody_wrote() {
        // A real document: a heading, a quote with its mekor, and a line of
        // the writer's own with an escaped `#` in it — which is a numeral sign
        // in Hebrew and turns up constantly.
        let markup = r#"#כותרת1[השכמת הבוקר]
#ציטוט[ראוי לכל ירא שמים]#מראה_מקום(מקור: "girsa:x/1:3")[שו"ע א' ג']

וצריך עיון על מה שכתב \#ד'.
"#;
        let text = to_text(markup);
        assert!(text.contains("השכמת הבוקר"), "{text}");
        assert!(text.contains("ראוי לכל ירא שמים"), "{text}");
        assert!(text.contains("וצריך עיון על מה שכתב #ד'."), "{text}");
        // The command names and their arguments are not words anybody wrote.
        assert!(!text.contains("כותרת1"), "{text}");
        assert!(!text.contains("girsa:x/1:3"), "{text}");
        assert!(!text.contains('#') || text.contains("#ד'"), "{text}");
    }

    #[test]
    fn the_refs_a_document_stores_can_be_read_back_out_of_it() {
        // *Where did I use this?* is a scan, because the refs are in the
        // document rather than only in the printed string.
        let markup = to_ksav(&packet(), CitationPlacement::Mekor).repeat(2)
            + "#מראה_מקום(מקור: \"girsa:bavli/berakhot/2a:1\")[ברכות ב.]";
        assert_eq!(
            refs_in(&markup),
            [
                "girsa:shulchan-arukh/orach-chayim/1:1",
                "girsa:bavli/berakhot/2a:1"
            ]
        );
        assert!(refs_in("prose with no citations at all").is_empty());
    }

    #[test]
    fn a_heading_is_a_ksav_heading_and_the_level_is_one_that_exists() {
        assert_eq!(heading(2, "סוגיא"), "#כותרת2[סוגיא]\n");
        assert_eq!(heading(9, "סוגיא"), "#כותרת6[סוגיא]\n");
        assert_eq!(heading(0, "סוגיא"), "#כותרת1[סוגיא]\n");
    }
}
