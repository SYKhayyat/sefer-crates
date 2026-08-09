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

#![doc(html_root_url = "https://docs.rs/girsa-ksav")]

pub mod read;

pub use read::{read, Block, NoteKind, ALIASES, PARAM_ALIASES};

use girsa_source::{Range, SourcePacket};

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
        CitationPlacement::Mekor => out.push_str(&mekor(
            &packet.display,
            Some(&packet.reference),
            packet.range,
        )),
        CitationPlacement::Inline if quoting => {
            out.push(' ');
            out.push_str(&inline_citation(&packet.display));
        }
        // With nothing in front of it there is nothing for a citation to be
        // *inline* with, and `#מקור[…]` alone is a source note floating in the
        // prose. A mareh makom is what a sefer writes here.
        CitationPlacement::Inline => out.push_str(&mekor(
            &packet.display,
            Some(&packet.reference),
            packet.range,
        )),
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
/// `range` is which characters of the quoted place the quote actually was.
/// [`Range::is_all`] and `None` are both written as **nothing at all** — a
/// document that quotes a whole se'if says so by saying nothing, which is what
/// every document written before this argument existed already says, and what
/// makes them all still correct.
#[must_use]
pub fn mekor(citation: &str, reference: Option<&str>, range: Option<Range>) -> String {
    match reference {
        Some(reference) => format!(
            "#מראה_מקום(מקור: \"{}\"{})[{}]",
            in_a_string(reference),
            characters(range),
            escape(citation)
        ),
        None => format!("#מראה_מקום[{}]", escape(citation)),
    }
}

/// The named argument that carries the range, or nothing.
///
/// `תווים: "4-19"` — characters, half-open, counted in the text **as
/// the reader was shown it**. `"4-"` is *from there to the end*, which is what
/// a highlight that runs off the last word means and what an editor who adds a
/// word to the se'if should still get.
fn characters(range: Option<Range>) -> String {
    match range {
        Some(range) if !range.is_all() => match range.to {
            Some(to) => format!(", תווים: \"{}-{to}\"", range.from),
            None => format!(", תווים: \"{}-\"", range.from),
        },
        _ => String::new(),
    }
}

/// One `מקור:` a document stores: the place, and which characters of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cited {
    /// The `girsa:` ref.
    pub reference: String,
    /// The characters of it this citation quoted, if the document said.
    ///
    /// `None` is *the whole of what the ref names* — see [`mekor`]. It is not
    /// the packet's `None`, which means nobody recorded one; by the time a
    /// citation is in a document the two have the same answer, and only the
    /// document's spelling of it survives.
    pub range: Option<Range>,
}

/// Every citation the document stores, in the order they appear.
///
/// [`refs_in`] is this with the ranges dropped and the repeats removed. It is
/// written that way round on purpose: a second scanner over the same markup
/// would be a second answer to *what does this document cite*, and the two
/// would disagree the first time either grew an argument.
#[must_use]
pub fn cited_in(markup: &str) -> Vec<Cited> {
    let mut out = Vec::new();
    let mut rest = markup;
    while let Some(at) = rest.find("מקור:") {
        rest = &rest[at + "מקור:".len()..];
        let Some(open) = rest.find('"') else { break };
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else { break };
        let found = &after[..close];
        rest = &after[close + 1..];
        if !found.starts_with("girsa:") {
            continue;
        }
        out.push(Cited {
            reference: found.to_string(),
            // Only up to the `]` that ends this citation: the next one's
            // argument is not this one's, and a document is a list of these.
            range: range_in(&rest[..rest.find(']').unwrap_or(rest.len())]),
        });
    }
    out
}

/// `תווים: "4-19"` read back off the markup.
fn range_in(args: &str) -> Option<Range> {
    let at = args.find("תווים:")? + "תווים:".len();
    let rest = &args[at..];
    let open = rest.find('"')?;
    let after = &rest[open + 1..];
    let close = after.find('"')?;
    let (from, to) = after[..close].split_once('-')?;
    Some(Range {
        from: from.trim().parse().ok()?,
        // Empty is *to the end*, which is a range and not a failure to write
        // one. `"4-"` and `"4-0"` are different documents.
        to: match to.trim() {
            "" => None,
            to => Some(to.parse().ok()?),
        },
    })
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
    let mut out: Vec<String> = Vec::new();
    for cited in cited_in(markup) {
        if !out.contains(&cited.reference) {
            out.push(cited.reference);
        }
    }
    out
}

/// Every character Typst reads as markup inside a `[…]` body.
///
/// Public so it can be *checked*. The same list exists in Ksav — in
/// `engine/src/escape.rs`, which owns it, because the escaper is needed in
/// Ksav's browser build and this crate is a native-only dependency there — and
/// in the editor's TypeScript, which had **five** of the ten. Both write
/// `#מראה_מקום(מקור: …)[…]` out of the same Girsa `display` string, so the five
/// missing ones (`*` strong, `_` emph, `<`/`>` a label, `@` a ref, all of which
/// occur in Sefaria titles) were the difference between one door and the other.
///
/// Ksav's `engine/tests/from_girsa.rs` holds the two lists together. That is the
/// direction that can be run: Ksav compiles this crate, and this crate cannot
/// compile Ksav.
pub const MARKUP: &[char] = &['#', '[', ']', '\\', '$', '*', '_', '<', '>', '@'];

/// Escape the characters Typst reads as markup.
///
/// See the module note: an unclosed `[` from an unescaped quote is reported at
/// end of file, nowhere near the quote that caused it.
#[must_use]
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if MARKUP.contains(&c) {
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
    fn a_whole_seif_says_nothing_about_characters() {
        // Every document written before this argument existed says exactly
        // this, which is why they are all still right.
        let markup = mekor("שו\"ע", Some("girsa:shulchan-arukh/orach-chayim/1:1"), None);
        assert!(!markup.contains("תווים"));
        assert_eq!(
            markup,
            mekor(
                "שו\"ע",
                Some("girsa:shulchan-arukh/orach-chayim/1:1"),
                Some(Range::all())
            )
        );
    }

    #[test]
    fn half_a_seif_says_which_half_and_can_be_read_back() {
        let markup = mekor(
            "שו\"ע",
            Some("girsa:shulchan-arukh/orach-chayim/1:1"),
            Some(Range {
                from: 4,
                to: Some(19),
            }),
        );
        assert!(markup.contains("תווים: \"4-19\""));
        let cited = cited_in(&markup);
        assert_eq!(cited.len(), 1);
        assert_eq!(
            cited[0].range,
            Some(Range {
                from: 4,
                to: Some(19)
            })
        );
    }

    #[test]
    fn to_the_end_is_a_range_and_not_a_missing_one() {
        // `"4-"` and `"4-0"` are different documents, and reading the first as
        // the second would quote nothing at all.
        let markup = mekor(
            "שו\"ע",
            Some("girsa:shulchan-arukh/orach-chayim/1:1"),
            Some(Range { from: 4, to: None }),
        );
        assert!(markup.contains("תווים: \"4-\""));
        assert_eq!(
            cited_in(&markup)[0].range,
            Some(Range { from: 4, to: None })
        );
    }

    #[test]
    fn one_citation_s_characters_are_not_the_next_one_s() {
        let document = format!(
            "{}\nוכן\n{}\n",
            mekor(
                "א'",
                Some("girsa:shulchan-arukh/orach-chayim/1:1"),
                Some(Range {
                    from: 0,
                    to: Some(10)
                })
            ),
            mekor("ב'", Some("girsa:shulchan-arukh/orach-chayim/1:2"), None),
        );
        let cited = cited_in(&document);
        assert_eq!(cited.len(), 2);
        assert_eq!(
            cited[0].range,
            Some(Range {
                from: 0,
                to: Some(10)
            })
        );
        assert_eq!(
            cited[1].range, None,
            "the second one quotes the whole se'if"
        );
    }

    #[test]
    fn the_range_travels_from_the_packet_into_the_document() {
        let mut packet = packet();
        packet.range = Some(Range {
            from: 0,
            to: Some(10),
        });
        let markup = to_ksav(&packet, CitationPlacement::Mekor);
        assert_eq!(
            cited_in(&markup)[0].range,
            Some(Range {
                from: 0,
                to: Some(10)
            })
        );
    }

    #[test]
    fn the_ranges_do_not_change_which_refs_a_document_cites() {
        // `refs_in` is `cited_in` with the ranges dropped. The same sefer
        // quoted twice at two different ranges is one entry in a mareh
        // mekomos, because it is one place.
        let document = format!(
            "{}\n{}\n",
            mekor(
                "א'",
                Some("girsa:shulchan-arukh/orach-chayim/1:1"),
                Some(Range {
                    from: 0,
                    to: Some(4)
                })
            ),
            mekor("א'", Some("girsa:shulchan-arukh/orach-chayim/1:1"), None),
        );
        assert_eq!(
            refs_in(&document),
            ["girsa:shulchan-arukh/orach-chayim/1:1"]
        );
        assert_eq!(cited_in(&document).len(), 2);
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
            mekor("שו\"ע או\"ח א' א'", None, None),
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
