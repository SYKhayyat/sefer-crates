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

#![doc(html_root_url = "https://docs.rs/girsa-ksav/0.3.0")]

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
#[must_use]
pub fn to_ksav(packet: &SourcePacket, placement: CitationPlacement) -> String {
    let mut out = String::with_capacity(packet.text.len() + packet.display.len() + 64);
    out.push_str(&quote_block(&packet.text));
    match placement {
        CitationPlacement::Mekor => out.push_str(&mekor(&packet.display)),
        CitationPlacement::Inline => {
            out.push(' ');
            out.push_str(&inline_citation(&packet.display));
        }
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

/// `#מראה_מקום[…]` — the citation as a footnote.
#[must_use]
pub fn mekor(citation: &str) -> String {
    format!("#מראה_מקום[{}]", escape(citation))
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
        assert!(markup.contains("#מראה_מקום["));
        assert!(markup.contains("יתגבר כארי"));
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
    fn a_heading_is_a_ksav_heading_and_the_level_is_one_that_exists() {
        assert_eq!(heading(2, "סוגיא"), "#כותרת2[סוגיא]\n");
        assert_eq!(heading(9, "סוגיא"), "#כותרת6[סוגיא]\n");
        assert_eq!(heading(0, "סוגיא"), "#כותרת1[סוגיא]\n");
    }
}
