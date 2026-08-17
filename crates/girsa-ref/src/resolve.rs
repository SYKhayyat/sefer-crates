//! Turning any way a human writes a citation into a canonical ref.
//!
//! Offline, local, and — the part that matters — **never a guess**.
//!
//! # The rule
//!
//! spec.md §4.3 and BUILDER.md rule 6: *ambiguity is surfaced as a choice,
//! never resolved silently*. A wrong ref is worse than no ref everywhere in
//! this system, because a wrong ref does not look wrong. It resolves, it opens
//! a page, and the page is the wrong one — and if it has been written into a
//! Ksav document, it is wrong in a printed sefer.
//!
//! So [`Resolution`] has three arms and the middle one is not a failure mode.
//! `או"ח` genuinely means the Orach Chayim of the Shulchan Arukh *and* of the
//! Tur *and* of a hundred sets of responsa. The honest answer is all of them.
//!
//! # What it reads
//!
//! ```text
//! שו"ע או"ח א' א'          gershayim, Hebrew numerals
//! שוע אוח סימן א סעיף א     no gershayim at all
//! רמב"ם הל' תפילה פ"ד ה"א   rabbinic shorthand, label-and-number in one token
//! ברכות ב.                  daf, printed notation
//! ברכות ב ע"א               daf, written out
//! Berakhot 2a               Sefaria's own
//! Shulchan Arukh, Orach Chayim 1:1
//! סעיף ה                    partial, against where the reader is standing
//! ```

use crate::address::{Address, Level};
use crate::lexicon::Lexicon;
use crate::numerals;
use crate::reference::Ref;
use girsa_hebrew::normalize;

/// What a citation turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// One work, one place. Safe to follow without asking.
    Exact(Ref),
    /// Several places it could equally be. **Always** shown to the reader as a
    /// choice; never narrowed by picking the first.
    Ambiguous(Vec<Ref>),
    /// Nothing in the lexicon matched. Better than a wrong answer.
    Unresolved,
}

impl Resolution {
    /// The single ref, if there is exactly one.
    #[must_use]
    pub fn exact(&self) -> Option<&Ref> {
        match self {
            Self::Exact(r) => Some(r),
            _ => None,
        }
    }

    /// Every candidate, however many there are.
    #[must_use]
    pub fn candidates(&self) -> &[Ref] {
        match self {
            Self::Exact(r) => std::slice::from_ref(r),
            Self::Ambiguous(rs) => rs,
            Self::Unresolved => &[],
        }
    }

    fn from_candidates(mut refs: Vec<Ref>) -> Self {
        refs.dedup_by(|a, b| a == b);
        match refs.len() {
            0 => Self::Unresolved,
            1 => Self::Exact(refs.remove(0)),
            _ => Self::Ambiguous(refs),
        }
    }
}

/// Where the reader currently is, for completing a partial citation.
///
/// This is the only context the resolver is allowed to use, and it is allowed
/// because the reader supplied it by being there: "see se'if 5" while standing
/// in Orach Chayim siman 1 means `1:5` and cannot mean anything else.
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub work: Option<Vec<String>>,
    pub address: Option<Address>,
}

/// Resolve a citation as written.
#[must_use]
pub fn resolve(lexicon: &Lexicon, citation: &str) -> Resolution {
    resolve_in_context(lexicon, citation, &Context::default())
}

/// Resolve a citation, completing a partial one against where the reader is.
#[must_use]
pub fn resolve_in_context(lexicon: &Lexicon, citation: &str, context: &Context) -> Resolution {
    let citation = citation.trim();
    if citation.is_empty() {
        return Resolution::Unresolved;
    }

    // Something already canonical resolves to itself. This is the paste path:
    // a ref copied out of a Ksav document has to come back unchanged.
    if let Ok(parsed) = citation.parse::<Ref>() {
        return Resolution::Exact(parsed);
    }

    let words: Vec<&str> = citation.split_whitespace().collect();
    if words.is_empty() {
        return Resolution::Unresolved;
    }

    // Longest title first. `שו"ע או"ח` has to beat `שו"ע`, or every citation
    // into Orach Chayim resolves to the whole Shulchan Arukh.
    let max_title_words = words.len().min(lexicon.longest_variant_words());
    for take in (1..=max_title_words).rev() {
        let title = words[..take].join(" ");
        let works = lexicon.lookup(&title);
        if works.is_empty() {
            continue;
        }

        let rest = words[take..].join(" ");
        let span = if rest.trim().is_empty() {
            None
        } else {
            match parse_span(&rest) {
                Some(s) => Some(s),
                // Text after the title that is not an address. Falling back to
                // a ref for the whole sefer would silently drop it and open the
                // work at its first page — `ברכות שבת` would resolve, look
                // fine, and be somewhere the reader never asked for. A shorter
                // title might still explain the whole string, so keep looking.
                None => continue,
            }
        };
        let span = match (span, &context.address) {
            (Some((from, to)), Some(here)) => Some((from.completed_against(here), to)),
            (s, _) => s,
        };

        let refs = works
            .into_iter()
            .map(|work| {
                let path: Vec<String> = work.slug.split('/').map(str::to_string).collect();
                match &span {
                    Some((from, Some(to))) => Ref::span(path, from.clone(), to.clone()),
                    Some((from, None)) => Ref::point(path, from.clone()),
                    None => Ref::whole_work(path),
                }
            })
            .collect();
        return Resolution::from_candidates(refs);
    }

    // No title at all. If the reader is standing in a sefer, a bare address is
    // a citation into it — "see se'if 5".
    if let (Some(work), Some(here)) = (&context.work, &context.address) {
        if let Some((partial, _)) = parse_span(citation) {
            return Resolution::Exact(Ref::point(work.clone(), partial.completed_against(here)));
        }
    }

    Resolution::Unresolved
}

/// Read an address, or a range of two.
///
/// `Exodus 1:1-6:1` is one citation covering a parsha, and the corpus is full
/// of them — Sefaria's link CSVs address whole sedras this way. A quote is a
/// range (spec.md §4.2), so a resolver that only reads points cannot express
/// half of what it is handed.
fn parse_span(rest: &str) -> Option<(Address, Option<Address>)> {
    // A hyphen is a range separator *only if the side after it is addressed
    // entirely by number*. It is also an ordinary character in a section name —
    // `שער חמישי - שער ייחוד המעשה`, `כסלו-טבת` — and splitting those tears a
    // name in half and loses the citation. Try the range reading, and fall back
    // rather than fail.
    //
    // The rule is [`crate::reference`]'s rule, so what this hands back is what
    // the parser gives back when it is read out of a document again. The
    // asymmetry is the corpus's: `Abarbanel on Torah, Exodus 27:20:1-14` opens
    // on a *named* level, because a commentary on Chumash is divided by book
    // before anything numbered, and 11,806 distinct citations here are shaped
    // that way. What is never named is the closing end.
    for (at, _) in rest.match_indices('-') {
        let (from, to) = rest.split_at(at);
        let (Some(from), Some(to)) = (parse_address(from), numbered_address(&to[1..])) else {
            continue;
        };
        return Some(if from == to {
            (from, None)
        } else {
            (from, Some(to))
        });
    }
    Some((parse_address(rest)?, None))
}

/// An address read from a citation, every level of which is a number or a daf.
fn numbered_address(rest: &str) -> Option<Address> {
    let address = parse_address(rest)?;
    address
        .levels()
        .iter()
        .all(Level::is_numbered)
        .then_some(address)
}

/// Words that name a division rather than being part of the address.
///
/// `סימן א' סעיף ב'` carries the same information as `א':ב'`; the labels tell a
/// person which level is which and tell this function nothing, because the
/// order already does.
///
/// # Where the list comes from
///
/// **Measured, not thought of.** Sefaria's 6,595 schemas name their levels in
/// `heSectionNames`, and across all of them there are exactly 42 distinct
/// words. Those are the words `girsa-cite` prints a citation with, so a word
/// the formatter writes and this function cannot read is a citation the system
/// prints and cannot follow. `שורה` — Sefaria's name for the segment inside a
/// daf, 242 nodes — was missing, and `ברכות דף ב. שורה א'` resolved to
/// `2a:שורה:1`: three levels, one of them a word, and it resolved.
///
/// Multi-word names (`סעיף קטן`, `מצות עשה`) are listed by their parts, because
/// the matching is per token.
///
/// # The five that are deliberately absent
///
/// `תורה`, `תלמוד`, `ספר`, `תפילה` and `מדרש` are all level names in some
/// schema and all of them are how seforim are *called*. A title is matched
/// before the address, so they are usually safe — but where the lexicon knows
/// only a prefix of the title, skipping them turns the rest of the name into
/// nothing and the citation lands on the whole sefer instead of saying it did
/// not understand. Kept as named levels, which is the more honest failure.
///
/// `שער הגמול - רמב"ן` is a node's *title* rather than a level word, and is
/// left alone for the same reason.
const SECTION_WORDS: [&str; 45] = [
    // Hebrew, as the schemas print them.
    "סימן",
    "סעיף",
    "קטן",
    "פרק",
    "פסקה",
    "פירוש",
    "פרשנות",
    "הלכה",
    "הלכות",
    "משנה",
    "תוספתא",
    "דף",
    "עמוד",
    "שורה",
    "פסוק",
    "פרשה",
    "חלק",
    "כרך",
    "מצוה",
    "דרוש",
    "מאמר",
    "כלל",
    "תשובה",
    "שורש",
    "שער",
    "אות",
    "מזמור",
    "רמז",
    "נתיב",
    "חדר",
    "קובץ",
    "פיוט",
    "הדרן",
    "סיפור",
    // English, from the same schemas' `sectionNames`.
    "siman",
    "seif",
    "chapter",
    "halakhah",
    "halacha",
    "daf",
    "mishnah",
    "verse",
    "paragraph",
    "comment",
    "line",
];

/// The same words in the form a token is compared in.
///
/// Normalized once rather than written normalized: `סעיף` normalizes to
/// `סעיפ`, and a hand-normalized list is a list that drifts the first time
/// somebody adds a word with a final letter in it. When it drifted, the word
/// fell through to the numeral reader and `סעיף` became 220, so
/// `שוע אוח סימן א סעיף א` resolved to `160:1:220:1` — four levels, all wrong,
/// and it resolved.
static SECTION_WORD_SET: std::sync::LazyLock<std::collections::BTreeSet<String>> =
    std::sync::LazyLock::new(|| SECTION_WORDS.iter().map(|w| normalize(w)).collect());

/// Abbreviated labels, recognised **only when the token carries a geresh**.
///
/// `סי'` is siman and `פ'` is perek. Bare, they are the numbers 60 and 80, and
/// a se'if 60 is an ordinary thing to cite — so the geresh is what makes the
/// difference and it is required.
///
/// `ה` is deliberately absent: `ה'` is halacha *and* the number 5, and se'if 5
/// is far commoner than a bare halacha marker.
const SECTION_ABBREVIATIONS: [&str; 6] = ["סי", "סע", "הל", "מס", "עמ", "פ"];

/// How a Gemara marks which side of the daf, as its own word.
///
/// `ברכות ב ע"א` is three tokens, and the third belongs to the second. Read
/// separately, `ע"א` sums to 71 and the citation resolves to Berakhot 2:71.
const AMUD_MARKERS: [(&str, char); 2] = [("ע\"א", 'a'), ("ע\"ב", 'b')];

/// Single letters that label the level they are attached to.
///
/// `פ"ד` is perek 4 and `ה"א` is halacha 1 — the label and the number share one
/// token. Read as a plain numeral, `פ"ד` is 84, and the citation lands on a
/// perek that does not exist.
///
/// Only single letters, and only these: `קכ"א` is 121 and must stay 121, so a
/// two-letter head is never a label.
const LABEL_LETTERS: [char; 5] = ['פ', 'ה', 'ס', 'ד', 'מ'];

/// Whether a token reads as a **numbered** level on its own.
///
/// The three branches of the loop below that produce a number, asked as a
/// question instead of taken as an answer — so a label word can look at what
/// follows it before deciding it is labelling anything. See [`parse_address`],
/// where that decision is made and why it has to be.
fn is_numbered(token: &str) -> bool {
    crate::daf::parse(token).is_some()
        || split_label_and_number(token).is_some()
        || numerals::parse(token).is_some()
}

/// Read whatever address is left after the title has been taken off.
fn parse_address(rest: &str) -> Option<Address> {
    let mut levels: Vec<Level> = Vec::new();
    // A section name is usually several words — `שער ייחוד המעשה` is one
    // level, not three. Words accumulate here and become a single level when a
    // numbered one arrives after them.
    let mut pending_name: Vec<String> = Vec::new();

    macro_rules! flush_name {
        () => {
            if !pending_name.is_empty() {
                levels.push(Level::canonical(pending_name.join(" ")));
                pending_name.clear();
            }
        };
    }

    let tokens = split_tokens(rest);
    for (nth, token) in tokens.iter().enumerate() {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        let normalized = normalize(token);
        let had_geresh = normalized.ends_with('\'') || normalized.ends_with('"');
        let bare = normalized.trim_end_matches(['\'', '"']);

        // **A label word labels the number after it, and where there is no
        // number after it, it is part of a name.**
        //
        // This used to skip the word unconditionally, and the cost was 2,125
        // chalakim of Girsa's shelf. Half the level words on the list are also
        // the first word of the *name* of a level: `הלכות ברכות` is what the
        // Avudraham's schema calls that section, and `שער ייחוד המעשה` is what
        // Chovos HaLevavos calls one of its. Stripping the head handed back
        // `ברכות` and `ייחוד המעשה`, which are names no schema has — and the
        // citation then failed to land, or landed somewhere else, with nothing
        // saying which had happened.
        //
        // Looking ahead one token settles it without knowing anything about the
        // schema, because the two readings differ in exactly that place.
        // `סימן א` is a label and a number; `הלכות ברכות` is two words of one
        // name. Every citation in `the_words_the_corpus_uses_for_a_level_are_read_as_labels`
        // is the first shape, which is what makes this safe: a label with a
        // number after it is still a label.
        let is_label = SECTION_WORD_SET.contains(bare)
            || (had_geresh && SECTION_ABBREVIATIONS.contains(&bare));
        if is_label {
            match tokens.get(nth + 1).map(|next| is_numbered(next.trim())) {
                // A label with a number after it. What it always was.
                Some(true) | None => continue,
                // A label with a *word* after it, which is the case this is
                // about: it is the head of a name and not a label at all.
                //
                // It must go on `pending_name` and not fall through, because
                // what is below it is the numeral reader — and every one of
                // these words is also a number. `סעיף` reads as 220, which is
                // how `שוע אוח סימן א סעיף א` once resolved to `160:1:220:1`:
                // four levels, all wrong, and it resolved.
                Some(false) => {
                    pending_name.push(token.to_string());
                    continue;
                }
            }
        }

        // `ע"א` is not a level of its own — it says which side of the daf the
        // *previous* level named.
        if let Some(amud) = AMUD_MARKERS
            .iter()
            .find(|(marker, _)| *marker == normalized)
            .map(|(_, amud)| *amud)
        {
            let last = levels.pop()?;
            let daf = last.as_number()?;
            if daf < 2 {
                return None;
            }
            levels.push(Level::canonical(format!("{daf}{amud}")));
            continue;
        }

        // A daf keeps its amud, and `ב.` must be seen before the full stop is
        // trimmed off as punctuation.
        if let Some(daf) = crate::daf::parse(token) {
            flush_name!();
            levels.push(Level::canonical(daf));
            continue;
        }

        if let Some(n) = split_label_and_number(token) {
            flush_name!();
            levels.push(Level::number(n));
            continue;
        }

        if let Some(n) = numerals::parse(token) {
            flush_name!();
            levels.push(Level::number(n));
            continue;
        }

        // A named section. Not every level is a number: a commentary on Chumash
        // is divided by book, so Sefaria writes `Avi Ezer, Numbers 10:2:1` and
        // Otzaria writes `אבי עזר, במדבר, י, ב, א` — the `במדבר` is a section of
        // the work, not part of its title. A dictionary is divided by headword,
        // so `A Dictionary of the Talmud, אֱגוֹד 1` has one for a level too.
        //
        // Accepted **only when a numbered level follows it**, which is what
        // tells a section apart from a loose word: `ברכות שבת` has nothing
        // after the `שבת`, and reading it as a section would invent a place the
        // reader never asked for. Whether the section exists is settled at
        // lookup, where a missing one fails visibly rather than resolving to
        // somewhere else.
        pending_name.push(token.to_string());
    }

    // A name with nothing numbered after it is not a section — it is a word the
    // title match did not eat. `ברכות שבת` ends here, and reading the `שבת` as
    // a section would invent a place the reader never asked for.
    if !pending_name.is_empty() {
        return None;
    }

    (!levels.is_empty()).then(|| Address::new(levels))
}

/// Split an address into tokens, keeping a word-final colon attached.
///
/// A colon does two jobs and they have to be told apart:
///
/// ```text
/// 2a:1     Sefaria's separator, between two levels of one address
/// ב:       the amud, the second side of daf ב
/// ```
///
/// Splitting on every colon turns `ברכות ב:` into `ב` and nothing, and the
/// citation resolves to daf 2 with no side — off by one page, silently. So a
/// colon at the end of a whitespace-delimited word stays with the word, and
/// only an interior one separates.
fn split_tokens(rest: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in rest.split_whitespace() {
        if word.ends_with(':') || word.ends_with('.') {
            out.push(word.to_string());
            continue;
        }
        out.extend(
            word.split([':', ','])
                .filter(|p| !p.is_empty())
                .map(str::to_string),
        );
    }
    out
}

/// `פ"ד` → 4. `קכ"א` → None, because its head is two letters and it is 121.
fn split_label_and_number(token: &str) -> Option<u32> {
    let normalized = normalize(token);
    let (head, tail) = normalized.split_once('"')?;
    let mut head_chars = head.chars();
    let label = head_chars.next()?;
    if head_chars.next().is_some() || !LABEL_LETTERS.contains(&label) {
        return None;
    }
    numerals::parse_hebrew(tail)
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace bans these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::lexicon::Work;

    fn lexicon() -> Lexicon {
        let mut lex = Lexicon::default();
        lex.add(
            Work {
                slug: "shulchan-arukh/orach-chayim".into(),
                he_title: "שולחן ערוך, אורח חיים".into(),
                en_title: "Shulchan Arukh, Orach Chayim".into(),
            },
            &[
                "שולחן ערוך אורח חיים",
                "שו\"ע או\"ח",
                "שוע אוח",
                "או\"ח",
                "Shulchan Arukh, Orach Chayim",
                "S.A. O.C.",
            ],
        );
        lex.add(
            Work {
                slug: "tur/orach-chayim".into(),
                he_title: "טור, אורח חיים".into(),
                en_title: "Tur, Orach Chayim".into(),
            },
            &["טור או\"ח", "או\"ח"],
        );
        lex.add(
            Work {
                slug: "shulchan-arukh".into(),
                he_title: "שולחן ערוך".into(),
                en_title: "Shulchan Arukh".into(),
            },
            &["שולחן ערוך", "שו\"ע"],
        );
        lex.add(
            Work {
                slug: "bavli/berakhot".into(),
                he_title: "ברכות".into(),
                en_title: "Berakhot".into(),
            },
            &["ברכות", "Berakhot"],
        );
        lex.add(
            Work {
                slug: "mishneh-torah/tefilah".into(),
                he_title: "משנה תורה, הלכות תפילה".into(),
                en_title: "Mishneh Torah, Tefilah".into(),
            },
            &["רמב\"ם הל' תפילה", "משנה תורה הלכות תפילה"],
        );
        lex
    }

    fn resolved(citation: &str) -> String {
        match resolve(&lexicon(), citation) {
            Resolution::Exact(r) => r.to_string(),
            Resolution::Ambiguous(rs) => {
                format!("AMBIGUOUS({})", rs.len())
            }
            Resolution::Unresolved => "UNRESOLVED".into(),
        }
    }

    #[test]
    fn gershayim_or_not_reaches_the_same_place() {
        assert_eq!(
            resolved("שו\"ע או\"ח א' א'"),
            "girsa:shulchan-arukh/orach-chayim/1:1"
        );
        assert_eq!(
            resolved("שוע אוח סימן א סעיף א"),
            "girsa:shulchan-arukh/orach-chayim/1:1"
        );
    }

    #[test]
    fn the_longest_title_wins() {
        // `שו"ע או"ח` must beat `שו"ע`, or every citation into Orach Chayim
        // resolves to the whole Shulchan Arukh and lands on siman 1.
        assert_eq!(
            resolved("שו\"ע או\"ח קכ\"א ג'"),
            "girsa:shulchan-arukh/orach-chayim/121:3"
        );
        assert_eq!(resolved("שו\"ע קכ\"א"), "girsa:shulchan-arukh/121");
    }

    #[test]
    fn rabbinic_shorthand_reads_the_label_and_the_number_out_of_one_token() {
        assert_eq!(
            resolved("רמב\"ם הל' תפילה פ\"ד ה\"א"),
            "girsa:mishneh-torah/tefilah/4:1"
        );
    }

    #[test]
    fn the_words_the_corpus_uses_for_a_level_are_read_as_labels() {
        // Not a list somebody thought of: these are the distinct
        // `heSectionNames` across Sefaria's 6,595 schemas, which is what
        // `girsa-cite` prints a citation with. A word the formatter writes and
        // the resolver cannot read is a citation this system prints and cannot
        // follow — see `girsa-cite`'s round-trip test, which is what found
        // `שורה` missing here.
        assert_eq!(resolved("ברכות דף ב. שורה א'"), "girsa:bavli/berakhot/2a:1");
        assert_eq!(
            resolved("שו\"ע או\"ח סימן קכ\"א סעיף ג'"),
            "girsa:shulchan-arukh/orach-chayim/121:3"
        );
        // `פסקה` is the commonest of the 42 by a distance — 18,793 nodes.
        assert_eq!(resolved("ברכות פסקה ג'"), "girsa:bavli/berakhot/3");
        // `אות` is how a nosei keilim is cited, and it was missing too.
        assert_eq!(resolved("ברכות אות ה'"), "girsa:bavli/berakhot/5");
        assert_eq!(
            resolved("ברכות חלק ב' פירוש ד'"),
            "girsa:bavli/berakhot/2:4"
        );
    }

    #[test]
    fn a_level_word_at_the_head_of_a_name_is_part_of_the_name() {
        // **The 2,125.** Girsa measured how many of its shelf's chalakim a
        // typed mareh makom lands on, and this was the largest single reason
        // the rest did not: half the words on `SECTION_WORDS` are also the
        // first word of the *name* of a section, and the head was being taken
        // off before the name was looked up.
        //
        // `הלכות ברכות` is what the Avudraham's schema calls that section, and
        // `ברכות` is a name it does not have. Nothing said so — the citation
        // failed to land, or landed elsewhere, and both looked the same from
        // outside.
        //
        // The rule is one token of lookahead: a label labels the number after
        // it, and a word after it means it was never a label.
        assert_eq!(
            resolved("ברכות הלכות ברכות ג'"),
            "girsa:bavli/berakhot/הלכות ברכות:3"
        );
        // The same shape with the level word Chovos HaLevavos uses.
        assert_eq!(
            resolved("ברכות שער הבחינה ב'"),
            "girsa:bavli/berakhot/שער הבחינה:2"
        );

        // **And the one it cannot reach, written down rather than hidden.**
        //
        // `שער ייחוד המעשה` is another of that sefer's sections, and this
        // resolves it to `38:המעשה:2` — before this change and after it, the
        // same way. The lookahead asks *is the next token a number*, and
        // `ייחוד` is י-י-ח-ו-ד: 10, 10, 8, 6, 4, which never goes back up. It
        // is a legal numeral by the only rule `parse_hebrew` has, and that rule
        // is the one keeping `ברכות שבת` from being siman 702.
        //
        // So a name whose second word reads as a numeral is out of reach here,
        // and it is out of reach for the reason the crate is built on rather
        // than by an oversight. Telling those two apart needs the schema, which
        // this layer does not have. It is asserted rather than left unspoken:
        // a limit nobody has written down is a limit somebody rediscovers.
        assert_eq!(
            resolved("ברכות שער ייחוד המעשה ב'"),
            "girsa:bavli/berakhot/38:המעשה:2"
        );

        // And the half of the rule that keeps every citation above working: a
        // label with a number after it is still a label, in all three of the
        // notations a number arrives in.
        assert_eq!(resolved("ברכות סימן ג'"), "girsa:bavli/berakhot/3");
        assert_eq!(resolved("ברכות דף ב."), "girsa:bavli/berakhot/2a");
        assert_eq!(
            resolved("רמב\"ם הל' תפילה פ\"ד ה\"א"),
            "girsa:mishneh-torah/tefilah/4:1"
        );

        // A label word must never reach the numeral reader, whichever branch
        // it leaves by. Every one of these words is also a number — `סעיף` is
        // 220 — and a citation that resolved through one of them would be four
        // levels of nonsense that looked like an address.
        // Refused, both ways round, which is the right answer: a citation
        // naming a level and no number has not named a place.
        assert_eq!(resolved("ברכות סעיף"), "UNRESOLVED");
        assert_eq!(resolved("ברכות הלכות סעיף"), "UNRESOLVED");
    }

    #[test]
    fn a_word_that_names_a_sefer_is_not_thrown_away_as_a_label() {
        // The other half of the same decision. `תורה`, `תלמוד`, `ספר`,
        // `מדרש` and `תפילה` all appear in the schemas as level names, and all
        // of them are how seforim are called. Skipping them would turn
        // `משנה תורה הלכות תפילה` into a citation of nothing in particular —
        // so they are deliberately not on the list, and a citation that uses
        // one as a section keeps it as a named level instead.
        assert_eq!(
            resolved("משנה תורה הלכות תפילה פ\"ד ה\"א"),
            "girsa:mishneh-torah/tefilah/4:1"
        );
        assert_eq!(resolved("ברכות תורה ג'"), "girsa:bavli/berakhot/תורה:3");
    }

    #[test]
    fn a_daf_reads_in_every_notation() {
        for written in ["ברכות ב.", "ברכות ב ע\"א", "Berakhot 2a"] {
            assert_eq!(resolved(written), "girsa:bavli/berakhot/2a", "{written}");
        }
        assert_eq!(resolved("ברכות ב:"), "girsa:bavli/berakhot/2b");
    }

    #[test]
    fn a_sefaria_style_ref_pasted_from_anywhere_resolves() {
        assert_eq!(
            resolved("Shulchan Arukh, Orach Chayim 1:1"),
            "girsa:shulchan-arukh/orach-chayim/1:1"
        );
        assert_eq!(resolved("Berakhot 2a"), "girsa:bavli/berakhot/2a");
    }

    #[test]
    fn a_canonical_ref_resolves_to_itself() {
        // The paste path: a ref copied out of a Ksav document comes back
        // unchanged rather than being re-derived and possibly re-decided.
        assert_eq!(
            resolved("girsa:bavli/berakhot/2a:1-2b:4"),
            "girsa:bavli/berakhot/2a:1-2b:4"
        );
    }

    #[test]
    fn a_citation_with_two_plausible_targets_is_never_silently_picked() {
        // The rule the whole crate exists to keep. או"ח is Orach Chayim in the
        // Shulchan Arukh and in the Tur, and no amount of cleverness makes one
        // of them the answer.
        let r = resolve(&lexicon(), "או\"ח סימן א");
        match &r {
            Resolution::Ambiguous(candidates) => {
                assert_eq!(candidates.len(), 2);
                let slugs: Vec<String> = candidates.iter().map(Ref::work_slug).collect();
                assert!(slugs.contains(&"shulchan-arukh/orach-chayim".to_string()));
                assert!(slugs.contains(&"tur/orach-chayim".to_string()));
            }
            other => panic!("expected a choice, got {other:?}"),
        }
        assert!(
            r.exact().is_none(),
            "an ambiguous resolution has no exact ref"
        );
    }

    #[test]
    fn a_partial_citation_completes_against_where_the_reader_is_standing() {
        let context = Context {
            work: Some(vec!["shulchan-arukh".into(), "orach-chayim".into()]),
            address: Address::parse("121:3"),
        };
        let r = resolve_in_context(&lexicon(), "סעיף ה", &context);
        assert_eq!(
            r.exact().map(ToString::to_string),
            Some("girsa:shulchan-arukh/orach-chayim/121:5".into())
        );
    }

    #[test]
    fn an_unknown_sefer_is_unresolved_rather_than_the_nearest_match() {
        assert_eq!(resolved("קרן אורה על נדרים ב."), "UNRESOLVED");
        assert_eq!(resolved(""), "UNRESOLVED");
    }

    #[test]
    fn a_word_in_the_address_position_is_refused_rather_than_read_as_a_number() {
        // Every Hebrew word is a number if you insist. `ברכות שבת` would be
        // Berakhot siman 702 to a resolver that summed whatever it was given.
        assert_eq!(resolved("ברכות שבת"), "UNRESOLVED");
    }

    #[test]
    fn the_resolver_never_hands_back_a_ref_it_cannot_write_down() {
        // A resolved ref is stored as text — in a Ksav document, in a link row.
        // If the text reads back as a *different* ref, the citation changes
        // meaning the next time it is opened, and nothing errors.
        //
        // `Part 2` is a real Sefaria node name, so a citation can genuinely put
        // a named level on both sides of a hyphen. Read as a range, the ref
        // printed `girsa:bavli/berakhot/Part:2-Part:3`, and *that* string is one
        // address with a level called `2-Part`. Two readings of one hyphen, in
        // two functions of one crate.
        let r = resolve(&lexicon(), "ברכות Part 2-Part 3");
        for candidate in r.candidates() {
            assert!(
                candidate.is_well_formed(),
                "{candidate} reads back as {:?}",
                candidate.to_string().parse::<Ref>()
            );
        }
        assert!(!r.candidates().is_empty(), "the citation still resolves");
    }

    #[test]
    fn a_hyphenated_section_name_is_not_torn_in_half() {
        // `כסלו-טבת` is one section of one sefer. Split on the hyphen it is two
        // words, neither of which is an address, and the citation is lost.
        assert_eq!(
            resolved("ברכות כסלו-טבת 5"),
            "girsa:bavli/berakhot/כסלו-טבת:5"
        );
    }

    #[test]
    fn a_two_letter_numeral_is_not_mistaken_for_a_label() {
        assert_eq!(split_label_and_number("קכ\"א"), None);
        assert_eq!(split_label_and_number("י\"א"), None);
        assert_eq!(split_label_and_number("פ\"ד"), Some(4));
        assert_eq!(split_label_and_number("ה\"א"), Some(1));
    }
}
