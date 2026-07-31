//! Reading a Ksav document back — its structure, not only its words.
//!
//! [`to_text`](crate::to_text) took the commands off and kept what was between
//! the brackets, which was enough to put a paragraph on a shelf and no more.
//! Measured against a real document it loses three things, and two of them
//! silently:
//!
//! ```text
//! #כותרת1[מבוא]                        → "מבוא", as body text. Not a heading,
//!                                          and so not a level of the address.
//! #רשימה(פריט[א], פריט[ב])              → nothing at all. The items live in the
//! #טבלה(עמודות: 2, תא[א], תא[ב])         arguments, and the arguments were skipped.
//! סוף#הערה[הערת שוליים].                 → "סוף הערת שוליים ." — the note spliced
//!                                          into the middle of the sentence.
//! ```
//!
//! A sefer of yours whose lists and tables are **absent from the shelf and from
//! the search** is the silent-gap defect this project refuses everywhere else
//! (spec.md §9.7's rule, one file over): it does not look like a loss, it looks
//! like a document that did not have a table in it.
//!
//! So this reads the markup into [`Block`]s. It is still a **reading and not an
//! evaluation** — Typst is the only thing that can say what a document
//! *renders* as, and putting the compiler inside the library to shelve a
//! paragraph is not a trade this makes. What it does is know which commands
//! are structure.
//!
//! # Flattened in reading order, and nothing dropped
//!
//! Ksav nests without limit — the engine ships an example 25 lists deep, and a
//! table inside a footnote inside a table cell. A tree faithful to that would
//! be a tree nothing here can address, since a segment id is a path of levels
//! and a citation is a range over them.
//!
//! So the blocks come out **flat, in reading order**, with the nesting recorded
//! where it can be read: an [`Block::Item`] carries its depth, a note carries
//! the number that was left in the text. A table inside a footnote emits its
//! rows after that footnote. Everything that was written is there and is
//! addressable; what is lost is only the shape of the containment, and it is
//! lost loudly rather than by omission.
//!
//! # The command table, and where it really lives
//!
//! Ksav's `engine/src/commands.rs` is the registry — 104 commands, each with a
//! category. This file knows only the ones that are **structure**: headings,
//! lists, tables, footnotes and block quotes, about forty names. Everything
//! else is inline by default and its content is kept, so a new style command in
//! Ksav needs no change here and cannot lose a word by being unknown.

/// One thing in a document, in the order it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// `#כותרת1[…]`, `#כותרת(רמה: 4)[…]`, `#שער[…]`.
    Heading { level: u8, text: String },
    /// A run of body text. Notes that were in it have been taken out and left
    /// their marker behind.
    Paragraph(String),
    /// `#ציטוט[…]` — set apart from the text that quotes it.
    Quote(String),
    /// One `פריט[…]` of a list. `depth` is 0 for the outermost.
    Item {
        depth: u8,
        /// How the list numbers itself, where it does: `1`, `א`, or `None` for
        /// a bulleted list.
        ordinal: Option<String>,
        text: String,
    },
    /// One row of a `#טבלה`. A header row is the one built from `כותרת_תא`.
    Row { cells: Vec<String>, header: bool },
    /// A footnote of any of Ksav's kinds, lifted out of the sentence that
    /// carried it.
    Note {
        kind: NoteKind,
        /// The number left in the text, counting from 1 through the document.
        marker: usize,
        text: String,
    },
}

/// Which of Ksav's many footnotes this is.
///
/// They are different things on the page — a side note is beside the text and
/// an endnote is at the back — and they are different claims about the text: a
/// `#הערת_עורך` is a note **about** the words and never part of them, which is
/// exactly the distinction W20 draws between a correction and a girsa variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    /// `#הערה`, `#הערה_א/ב/ג`, `#הערה_בדרגה(n)`, `#הערה_זרם` — the ordinary
    /// footnote at the foot of the page, in whichever band.
    Foot,
    /// `#הערת_צד`, `#הערת_ימין`, `#הערת_שמאל`, `#הערת_גיליון`.
    Side,
    /// `#הערתסיום` — at the back.
    End,
    /// `#מראה_מקום`, `#הערת_מקור` — a citation, which is what a sefer does.
    Mekor,
    /// `#הערת_עורך` — about the text, and never part of it.
    Editor,
    /// `#הערה_על_הערה` — a note on a note.
    OnNote,
}

impl NoteKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Foot => "foot",
            Self::Side => "side",
            Self::End => "end",
            Self::Mekor => "mekor",
            Self::Editor => "editor",
            Self::OnNote => "on-note",
        }
    }

    /// Whether this note is part of the text or a remark about it.
    ///
    /// An editor's note is the one that is not, and a sefer built from a
    /// document must not read one as though the author had written it into the
    /// sentence.
    #[must_use]
    pub const fn is_the_text(self) -> bool {
        !matches!(self, Self::Editor)
    }
}

/// What a command does to the shape of the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Heading(u8),
    /// A list. The `char` is how it numbers, where it does.
    List(Option<char>),
    Table,
    Note(NoteKind),
    Quote,
    /// A page or column break, a rule — a paragraph boundary with no words.
    Break,
    /// `#הגדרות_*` and friends: settings, no content, nothing to read.
    Setting,
    /// Anything else. Its content is words in the paragraph it sits in, which
    /// is what an unknown command gets too.
    Inline,
}

/// What a command name does.
///
/// Unknown is [`Role::Inline`] on purpose: Ksav gains style commands, and a
/// reader that dropped what it did not recognise would lose a sentence for a
/// font change.
fn role(name: &str) -> Role {
    match name {
        "שער" => Role::Heading(1),
        "תת_שער" => Role::Heading(2),
        "כותרת1" => Role::Heading(1),
        "כותרת2" => Role::Heading(2),
        "כותרת3" => Role::Heading(3),
        "כותרת4" => Role::Heading(4),
        "כותרת5" => Role::Heading(5),
        "כותרת6" => Role::Heading(6),
        // `#כותרת(רמה: n)[…]`. The level is in the arguments and is read there;
        // 4 is what Ksav's own snippet inserts, and is the floor if it is
        // missing rather than a guess at what was meant.
        "כותרת" => Role::Heading(4),

        "רשימה" => Role::List(None),
        "ממוספרת" => Role::List(Some('1')),
        "ממוספרת_עברית" => Role::List(Some('א')),
        "רשימת_הגדרות" => Role::List(None),

        "טבלה" => Role::Table,

        "הערה" | "הערה_א" | "הערה_ב" | "הערה_ג" | "הערה_בדרגה" | "הערה_זרם" => {
            Role::Note(NoteKind::Foot)
        }
        "הערת_צד" | "הערת_ימין" | "הערת_שמאל" | "הערת_גיליון" | "הערת_תוכן" => {
            Role::Note(NoteKind::Side)
        }
        "הערתסיום" => Role::Note(NoteKind::End),
        "מראה_מקום" | "הערת_מקור" => Role::Note(NoteKind::Mekor),
        "הערת_עורך" => Role::Note(NoteKind::Editor),
        "הערה_על_הערה" => Role::Note(NoteKind::OnNote),

        "ציטוט" => Role::Quote,

        "מעבר_עמוד" | "מעבר_טור" | "מעבר_שורה" | "קו_מפריד" | "מקטע_עמוד" | "חסר" => {
            Role::Break
        }

        _ if name.starts_with("הגדרות_") => Role::Setting,

        _ => Role::Inline,
    }
}

/// Read a Ksav document into its blocks, in reading order.
///
/// Never fails: a document that is half-written, or that ends inside a bracket,
/// yields the blocks it got to. This runs over files a person is in the middle
/// of typing.
#[must_use]
pub fn read(markup: &str) -> Vec<Block> {
    let mut reader = Reader {
        src: markup.as_bytes(),
        chars: markup.char_indices().collect(),
        at: 0,
        out: Vec::new(),
        pending: Vec::new(),
        notes: 0,
    };
    let text = reader.run(None, 0);
    reader.flush(text);
    reader.out
}

struct Reader<'a> {
    src: &'a [u8],
    /// The document by character, so a Hebrew name is read a letter at a time
    /// and never split down the middle of one.
    chars: Vec<(usize, char)>,
    at: usize,
    out: Vec<Block>,
    /// Notes found inside the paragraph being read, emitted after it — a
    /// footnote belongs *after* the sentence that carried it and not inside it.
    pending: Vec<Block>,
    notes: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).map(|(_, c)| *c)
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.at += 1;
        }
        c
    }

    /// Finish a paragraph, and let its notes out behind it.
    fn flush(&mut self, text: String) {
        let text = tidy(&text);
        if !text.is_empty() {
            self.out.push(Block::Paragraph(text));
        }
        self.out.append(&mut self.pending);
    }

    /// Read markup until `close`, returning the words in it and emitting every
    /// block it meets.
    ///
    /// `depth` is how deep in lists we are, which is the one piece of nesting
    /// that survives into the blocks.
    fn run(&mut self, close: Option<char>, depth: u8) -> String {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if Some(c) == close {
                self.at += 1;
                return text;
            }
            match c {
                // An escape: the character after it is one somebody wrote.
                '\\' => {
                    self.at += 1;
                    if let Some(next) = self.next() {
                        text.push(next);
                    }
                }
                '#' => {
                    self.at += 1;
                    let taken = std::mem::take(&mut text);
                    text = self.command(taken, depth);
                }
                '\n' => {
                    self.at += 1;
                    // A blank line ends a paragraph. One newline is a line
                    // break inside one.
                    if self.peek() == Some('\n') && close.is_none() {
                        let taken = std::mem::take(&mut text);
                        self.flush(taken);
                    } else {
                        text.push('\n');
                    }
                }
                '[' | ']' => {
                    self.at += 1;
                    if !text.ends_with(char::is_whitespace) && !text.is_empty() {
                        text.push(' ');
                    }
                }
                _ => {
                    self.at += 1;
                    text.push(c);
                }
            }
        }
        text
    }

    /// A `#command`, with the paragraph so far handed in and handed back.
    fn command(&mut self, mut text: String, depth: u8) -> String {
        let name = self.name();
        let role = role(&name);

        // `(arguments)`, if there are any. Read for the ones that carry
        // content; skipped for everything else, because they are settings.
        let args = if self.peek() == Some('(') {
            self.at += 1;
            Some(self.args())
        } else {
            None
        };

        match role {
            Role::Setting | Role::Break => {
                if self.peek() == Some('[') {
                    self.at += 1;
                    let _ = self.run(Some(']'), depth);
                }
                self.flush(text);
                String::new()
            }
            Role::Heading(default_level) => {
                let level = args
                    .as_ref()
                    .and_then(|a| a.level)
                    .unwrap_or(default_level)
                    .clamp(1, 6);
                let (heading, mut inside) = self.content(depth);
                self.flush(text);
                if !heading.is_empty() {
                    self.out.push(Block::Heading {
                        level,
                        text: heading,
                    });
                }
                self.out.append(&mut inside);
                String::new()
            }
            Role::Quote => {
                let (quoted, mut inside) = self.content(depth);
                self.flush(text);
                if !quoted.is_empty() {
                    self.out.push(Block::Quote(quoted));
                }
                self.out.append(&mut inside);
                String::new()
            }
            Role::List(numbering) => {
                self.flush(text);
                if let Some(args) = args {
                    for (n, item) in args.parts.iter().enumerate() {
                        let ordinal = numbering.map(|kind| ordinal_at(kind, n));
                        let (text, mut inside) = self.sub(&item.body, depth.saturating_add(1));
                        if !text.is_empty() {
                            self.out.push(Block::Item {
                                depth,
                                ordinal,
                                text,
                            });
                        }
                        // Whatever the item held — a nested list, a heading, a
                        // table — comes out after it, where it was written,
                        // rather than being thrown away for being in an
                        // awkward place.
                        self.out.append(&mut inside);
                    }
                }
                String::new()
            }
            Role::Table => {
                self.flush(text);
                if let Some(args) = args {
                    self.table(&args, depth);
                }
                String::new()
            }
            Role::Note(kind) => {
                let (body, mut inside) = self.content(depth);
                if !body.is_empty() {
                    self.notes += 1;
                    // The marker goes where the note was, so the sentence still
                    // says that something hangs here — which is the whole
                    // difference between a footnote and an interruption.
                    text.push_str(&self.notes.to_string());
                    self.pending.push(Block::Note {
                        kind,
                        marker: self.notes,
                        text: body,
                    });
                }
                // Whatever the note held follows it, not the sentence above it.
                self.pending.append(&mut inside);
                text
            }
            Role::Inline => {
                let (inner, mut inside) = self.content(depth);
                if !inner.is_empty() {
                    if !text.is_empty() && !text.ends_with(char::is_whitespace) {
                        text.push(' ');
                    }
                    text.push_str(&inner);
                }
                self.pending.append(&mut inside);
                text
            }
        }
    }

    /// `[…]` if it is there, read as its own piece of markup.
    ///
    /// Read apart rather than in place, and this is not a detail: a block
    /// inside a content block — a table inside a footnote, a list inside a
    /// heading — ends the paragraph it is in, and in place that paragraph is
    /// *this* command's own words. They would leave as a stray paragraph and
    /// the command would come out empty, which is how a footnote with a table
    /// in it stopped being a footnote at all.
    fn content(&mut self, depth: u8) -> (String, Vec<Block>) {
        if self.peek() != Some('[') {
            return (String::new(), Vec::new());
        }
        self.at += 1;
        let markup = self.raw(']');
        self.sub(&markup, depth)
    }

    /// A command name — the letters after `#`.
    fn name(&mut self) -> String {
        let mut out = String::new();
        while let Some(c) = self.peek() {
            if c == '_' || c.is_alphanumeric() {
                out.push(c);
                self.at += 1;
            } else {
                break;
            }
        }
        out
    }

    /// Run a slice of markup as its own document, keeping this reader's notes
    /// and output.
    ///
    /// Used for the bodies inside `(arguments)`, which were sliced out before
    /// they could be walked — a list's items and a table's cells live there.
    fn sub(&mut self, markup: &str, depth: u8) -> (String, Vec<Block>) {
        let mut inner = Reader {
            src: markup.as_bytes(),
            chars: markup.char_indices().collect(),
            at: 0,
            out: Vec::new(),
            pending: Vec::new(),
            notes: self.notes,
        };
        let trailing = inner.run(None, depth);
        inner.notes_out();
        self.notes = inner.notes;
        let mut blocks = inner.out;

        // The item's own words. A list inside a list item ends the item's
        // paragraph before the item is finished, so those words are already a
        // block by the time the body is read out — and taking them back is the
        // difference between an item that says `חיצוני` and one that says
        // nothing while a stray paragraph floats beside it.
        let mut text = tidy(&trailing);
        if text.is_empty() {
            if let Some(Block::Paragraph(first)) = blocks.first() {
                text = first.clone();
                blocks.remove(0);
            }
        }
        (text, blocks)
    }

    /// Let any notes still held out, at the end of a sub-reading.
    fn notes_out(&mut self) {
        let mut pending = std::mem::take(&mut self.pending);
        self.out.append(&mut pending);
    }

    /// The rows of a `#טבלה`.
    fn table(&mut self, args: &Args, depth: u8) {
        let columns = args.columns.unwrap_or(0);
        let mut row: Vec<String> = Vec::new();
        let mut header = false;
        // Blocks found inside cells — a table inside a footnote inside a cell
        // is in Ksav's own sample document. They follow the whole table rather
        // than interrupting its rows.
        let mut held: Vec<Block> = Vec::new();
        for cell in &args.parts {
            if row.is_empty() {
                header = cell.header;
            }
            let (text, inside) = self.sub(&cell.body, depth);
            row.push(text);
            held.extend(inside);
            // A row ends when the column count says so, or when the kind of
            // cell changes — which is what separates a header row from the
            // first row of the body in a table that never said how wide it is.
            let full = columns > 0 && row.len() >= columns;
            if full {
                self.out.push(Block::Row {
                    cells: std::mem::take(&mut row),
                    header,
                });
            }
        }
        if !row.is_empty() {
            self.out.push(Block::Row { cells: row, header });
        }
        self.out.append(&mut held);
    }

    /// Everything between `(` and its `)`.
    ///
    /// Two kinds of thing live in there and they have to be told apart: named
    /// settings — `עמודות: 2`, `רמה: 4` — and the bodies of a container, which
    /// are written `פריט[…]`, `תא[…]`, `כותרת_תא[…]` with no `#` in front of
    /// them, or as `#מיזוג(2)[…]` with one.
    fn args(&mut self) -> Args {
        let mut out = Args::default();
        let mut word = String::new();
        while let Some(c) = self.peek() {
            match c {
                ')' => {
                    self.at += 1;
                    break;
                }
                '"' => {
                    self.at += 1;
                    while let Some(c) = self.next() {
                        if c == '\\' {
                            self.at += 1;
                        } else if c == '"' {
                            break;
                        }
                    }
                    word.clear();
                }
                ':' => {
                    self.at += 1;
                    let value = self.setting();
                    match word.trim() {
                        "עמודות" => out.columns = value.trim().parse().ok(),
                        "רמה" => out.level = value.trim().parse().ok(),
                        _ => {}
                    }
                    word.clear();
                }
                '[' => {
                    self.at += 1;
                    let body = self.raw(']');
                    let name = word.trim().trim_start_matches('#').to_string();
                    out.parts.push(Part {
                        header: name == "כותרת_תא",
                        body,
                    });
                    word.clear();
                }
                '(' => {
                    // A nested call inside the arguments — `#מיזוג(2)[…]`, or a
                    // colour. Its own arguments are not this level's business.
                    self.at += 1;
                    let _ = self.raw(')');
                }
                ',' => {
                    self.at += 1;
                    word.clear();
                }
                _ => {
                    self.at += 1;
                    word.push(c);
                }
            }
        }
        out
    }

    /// The value of a `key: value` setting — up to the comma that ends it.
    fn setting(&mut self) -> String {
        let mut out = String::new();
        let mut depth = 0usize;
        while let Some(c) = self.peek() {
            match c {
                ',' | ')' if depth == 0 => break,
                '(' | '[' => {
                    depth += 1;
                    out.push(c);
                    self.at += 1;
                }
                ')' | ']' => {
                    depth -= 1;
                    out.push(c);
                    self.at += 1;
                }
                _ => {
                    out.push(c);
                    self.at += 1;
                }
            }
        }
        out
    }

    /// Everything up to the matching `close`, as it stands — brackets and all,
    /// so it can be read as markup afterwards.
    fn raw(&mut self, close: char) -> String {
        let open = match close {
            ']' => '[',
            ')' => '(',
            _ => close,
        };
        let from = self.chars.get(self.at).map_or(self.src.len(), |(i, _)| *i);
        let mut depth = 0usize;
        let mut to = from;
        while let Some(c) = self.peek() {
            if c == '\\' {
                self.at += 2;
                to = self.chars.get(self.at).map_or(self.src.len(), |(i, _)| *i);
                continue;
            }
            if c == close {
                if depth == 0 {
                    self.at += 1;
                    break;
                }
                depth -= 1;
            } else if c == open {
                depth += 1;
            }
            self.at += 1;
            to = self.chars.get(self.at).map_or(self.src.len(), |(i, _)| *i);
        }
        // Lossy in type only, and checked when the lossy-decode family was swept in
        // B11: `from` and `to` both come from `char_indices` over this same
        // `&str`'s bytes, so they are character boundaries by construction and there
        // is nothing for a replacement character to stand in for. Left as it is
        // rather than turned into a fallible read that cannot fail.
        String::from_utf8_lossy(self.src.get(from..to).unwrap_or_default()).into_owned()
    }
}

/// The settings and bodies of one `(…)`.
#[derive(Debug, Default)]
struct Args {
    /// `עמודות: n` on a table.
    columns: Option<usize>,
    /// `רמה: n` on a heading.
    level: Option<u8>,
    parts: Vec<Part>,
}

/// One `פריט[…]` or `תא[…]`, still as markup.
#[derive(Debug)]
struct Part {
    header: bool,
    body: String,
}

/// `1`, `2`, `3` … or `א`, `ב`, `ג` …
fn ordinal_at(kind: char, n: usize) -> String {
    if kind != 'א' {
        return (n + 1).to_string();
    }
    // Hebrew letters, and past כ"ב it falls back to a number rather than
    // inventing a spelling nobody writes.
    const LETTERS: [char; 22] = [
        'א', 'ב', 'ג', 'ד', 'ה', 'ו', 'ז', 'ח', 'ט', 'י', 'כ', 'ל', 'מ', 'נ', 'ס', 'ע', 'פ', 'צ',
        'ק', 'ר', 'ש', 'ת',
    ];
    LETTERS
        .get(n)
        .map_or_else(|| (n + 1).to_string(), ToString::to_string)
}

/// Collapse the whitespace a reading leaves behind, keeping line breaks.
fn tidy(text: &str) -> String {
    text.lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_heading_is_a_heading_and_not_a_paragraph() {
        assert_eq!(
            read("#כותרת1[מבוא]"),
            vec![Block::Heading {
                level: 1,
                text: "מבוא".into()
            }]
        );
        assert_eq!(
            read("#כותרת(רמה: 4)[פרט]"),
            vec![Block::Heading {
                level: 4,
                text: "פרט".into()
            }]
        );
        // `#שער` is the document's own title and is level 1; the subtitle
        // under it is level 2, which is what it is on the page.
        assert_eq!(
            read("#שער[ספר]\n#תת_שער[חלק א]"),
            vec![
                Block::Heading {
                    level: 1,
                    text: "ספר".into()
                },
                Block::Heading {
                    level: 2,
                    text: "חלק א".into()
                }
            ]
        );
    }

    #[test]
    fn a_list_is_items_and_not_nothing() {
        // The defect this file exists for: the items are in the *arguments*,
        // and a reader that skipped the arguments dropped the whole list
        // without a word.
        assert_eq!(
            read("#רשימה(\n  פריט[ראשון],\n  פריט[שני],\n)"),
            vec![
                Block::Item {
                    depth: 0,
                    ordinal: None,
                    text: "ראשון".into()
                },
                Block::Item {
                    depth: 0,
                    ordinal: None,
                    text: "שני".into()
                },
            ]
        );
    }

    #[test]
    fn a_numbered_list_says_how_it_numbers() {
        let read = read("#ממוספרת_עברית(פריט[א], פריט[ב])");
        assert_eq!(
            read,
            vec![
                Block::Item {
                    depth: 0,
                    ordinal: Some("א".into()),
                    text: "א".into()
                },
                Block::Item {
                    depth: 0,
                    ordinal: Some("ב".into()),
                    text: "ב".into()
                },
            ]
        );
        let numbered = super::read("#ממוספרת(פריט[x])");
        assert_eq!(
            numbered[0],
            Block::Item {
                depth: 0,
                ordinal: Some("1".into()),
                text: "x".into()
            }
        );
    }

    #[test]
    fn a_table_is_rows_and_not_nothing() {
        assert_eq!(
            read("#טבלה(עמודות: 2, כותרת_תא[א], כותרת_תא[ב], תא[1], תא[2],)"),
            vec![
                Block::Row {
                    cells: vec!["א".into(), "ב".into()],
                    header: true
                },
                Block::Row {
                    cells: vec!["1".into(), "2".into()],
                    header: false
                },
            ]
        );
    }

    #[test]
    fn a_footnote_leaves_the_sentence_alone() {
        // Before this, `סוף#הערה[הערה].` read as `סוף הערה .` — the note
        // spliced into the middle of the sentence and the full stop orphaned.
        assert_eq!(
            read("סוף#הערה[הערת שוליים]."),
            vec![
                Block::Paragraph("סוף1.".into()),
                Block::Note {
                    kind: NoteKind::Foot,
                    marker: 1,
                    text: "הערת שוליים".into()
                },
            ]
        );
    }

    #[test]
    fn the_kinds_of_note_are_kept_apart() {
        // An editor's note is *about* the text and is never part of it — the
        // same distinction W20 draws between a correction and a girsa variant.
        let blocks = read("א#הערת_עורך[צריך עיון] ב#מראה_מקום[ברכות ב.] ג#הערתסיום[בסוף]");
        let kinds: Vec<NoteKind> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Note { kind, .. } => Some(*kind),
                _ => None,
            })
            .collect();
        assert_eq!(
            kinds,
            vec![NoteKind::Editor, NoteKind::Mekor, NoteKind::End]
        );
        assert!(!NoteKind::Editor.is_the_text());
        assert!(NoteKind::Mekor.is_the_text());
    }

    #[test]
    fn a_list_inside_a_list_is_deeper_and_is_still_there() {
        let blocks = read("#רשימה(פריט[חיצוני #ממוספרת(פריט[פנימי])])");
        let depths: Vec<u8> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Item { depth, .. } => Some(*depth),
                _ => None,
            })
            .collect();
        assert_eq!(depths, vec![0, 1], "the inner item is one level down");
        assert!(blocks.iter().any(|b| matches!(
            b,
            Block::Item { text, .. } if text.contains("פנימי")
        )));
    }

    #[test]
    fn a_footnote_with_a_table_in_it_is_still_a_footnote() {
        // Found in Ksav's own sample document, and it is the sharpest form of
        // the bug: the table ended the note's paragraph while the note was
        // still being read, so the note's words left as a stray paragraph and
        // the note came out empty — which meant no marker in the cell above
        // and no note anywhere. A footnote that vanishes is worse than one
        // drawn plainly.
        let blocks = read(
            "תא#הערה[מלים
#טבלה(עמודות: 2, תא[א], תא[ב])]",
        );
        assert_eq!(
            blocks[0],
            Block::Paragraph("תא1".into()),
            "the marker is still in the sentence: {blocks:#?}"
        );
        assert_eq!(
            blocks[1],
            Block::Note {
                kind: NoteKind::Foot,
                marker: 1,
                text: "מלים".into()
            },
            "{blocks:#?}"
        );
        assert_eq!(
            blocks[2],
            Block::Row {
                cells: vec!["א".into(), "ב".into()],
                header: false
            },
            "and the table it held follows it: {blocks:#?}"
        );
    }

    #[test]
    fn a_table_inside_a_footnote_inside_a_cell_still_comes_out() {
        // Ksav's own sample document does exactly this. Nothing here pretends
        // to keep the containment — the blocks are flat — but every word is
        // present and addressable, which is the property that matters.
        let blocks =
            read("#טבלה(עמודות: 1, תא[ראה הערה#הערה[#טבלה(עמודות: 2, תא[פנימי א], תא[פנימי ב])]])");
        let words: String = format!("{blocks:?}");
        assert!(words.contains("פנימי א"), "{blocks:#?}");
        assert!(words.contains("פנימי ב"), "{blocks:#?}");
    }

    #[test]
    fn an_unknown_command_keeps_its_words() {
        // Ksav gains style commands. A reader that dropped what it did not
        // know would lose a sentence for a font change.
        assert_eq!(
            read("#פקודה_שאין[מילים]"),
            vec![Block::Paragraph("מילים".into())]
        );
        assert_eq!(
            read("א #הדגשה[ב] ג"),
            vec![Block::Paragraph("א ב ג".into())]
        );
    }

    #[test]
    fn a_setting_has_no_words_in_it() {
        assert_eq!(read("#הגדרות_רשימות(סמן: ([◆], [–]), הזחה: 1.5em)"), vec![]);
    }

    #[test]
    fn an_escaped_bracket_is_a_bracket() {
        assert_eq!(
            read(r"\#לא פקודה \[ולא סוגר\]"),
            vec![Block::Paragraph("#לא פקודה [ולא סוגר]".into())]
        );
    }

    #[test]
    fn a_document_that_stops_mid_bracket_yields_what_it_had() {
        // This runs over files somebody is in the middle of typing.
        let blocks = read("#כותרת1[מבוא]\n\nטקסט#הערה[לא נסגר");
        assert!(blocks.contains(&Block::Heading {
            level: 1,
            text: "מבוא".into()
        }));
        assert!(!blocks.is_empty());
    }

    #[test]
    fn a_blank_line_ends_a_paragraph_and_one_newline_does_not() {
        assert_eq!(
            read("שורה א\nשורה ב\n\nפסקה שנייה"),
            vec![
                Block::Paragraph("שורה א\nשורה ב".into()),
                Block::Paragraph("פסקה שנייה".into()),
            ]
        );
    }

    #[test]
    fn a_break_ends_a_paragraph_and_adds_no_words() {
        assert_eq!(
            read("א\n#מעבר_עמוד\nב"),
            vec![Block::Paragraph("א".into()), Block::Paragraph("ב".into())]
        );
    }
}
