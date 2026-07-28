//! Daf and amud, in every notation the corpus uses.
//!
//! A page of Gemara has two sides, and there are at least six ways in
//! circulation to say which one:
//!
//! ```text
//! ב.        ב:          a full stop and a colon, the commonest in print
//! ב ע"א     ב ע"ב       amud alef, amud beis, written out
//! ב, א      ב, ב        the way Otzaria's heRefs come out
//! 2a        2b          Sefaria's own
//! ```
//!
//! All six mean the same two pages. A resolver that reads one of them reads
//! one sefer's citations and misses the next sefer's entirely.
//!
//! The canonical form is Sefaria's — `2a` — because that is what the link CSVs
//! are addressed in (spec.md §2.2), and the whole of W8 is resolving those.

use girsa_hebrew::normalize;

use crate::numerals;

/// Read a daf however it was written and return the canonical `2a` / `2b`.
///
/// Returns `None` when the text is not a daf, including for a bare number: `2`
/// is a siman or a perek far more often than it is a daf, and deciding which
/// belongs to the schema rather than to this function.
#[must_use]
pub fn parse(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    // Sefaria's own, and anything already canonical.
    if let Some(canonical) = parse_english(raw) {
        return Some(canonical);
    }

    let normalized = normalize(raw);
    let (number_part, amud) = split_amud(&normalized, raw)?;
    let daf = numerals::parse(number_part.trim())?;
    // Daf numbering starts at 2 — there is no daf 1 in any masechta, because
    // the first leaf is the title page. A "daf 1" is a misread siman.
    if daf < 2 {
        return None;
    }
    Some(format!("{daf}{amud}"))
}

/// `2a`, `2B`, `14b`.
fn parse_english(raw: &str) -> Option<String> {
    let mut chars = raw.chars();
    let last = chars.next_back()?;
    let amud = match last.to_ascii_lowercase() {
        'a' => 'a',
        'b' => 'b',
        _ => return None,
    };
    let digits: String = chars.collect();
    let n: u32 = digits.trim().parse().ok()?;
    (n >= 2).then(|| format!("{n}{amud}"))
}

/// Split the daf from whatever marks the amud, returning the number text and
/// the canonical amud letter.
fn split_amud(normalized: &str, raw: &str) -> Option<(String, char)> {
    // ב ע"א  /  ב ע"ב  — amud written out.
    for (marker, amud) in [("ע\"א", 'a'), ("ע\"ב", 'b')] {
        if let Some(head) = normalized.strip_suffix(marker) {
            return Some((head.trim().to_string(), amud));
        }
    }

    // ב, א  /  ב, ב  — the shape Otzaria's heRefs arrive in.
    if let Some((head, tail)) = raw.rsplit_once(',') {
        let tail = normalize(tail);
        let tail = tail.trim();
        if tail == "א" {
            return Some((head.to_string(), 'a'));
        }
        if tail == "ב" {
            return Some((head.to_string(), 'b'));
        }
    }

    // ב.  /  ב:  — the commonest in print, and the reason this is not a simple
    // suffix match: the full stop and colon are also ordinary punctuation.
    if let Some(head) = raw.strip_suffix('.') {
        return Some((head.to_string(), 'a'));
    }
    if let Some(head) = raw.strip_suffix(':') {
        return Some((head.to_string(), 'b'));
    }

    None
}

/// Write a daf the way a sefer prints it: `2a` → `ב.`, `2b` → `ב:`.
#[must_use]
pub fn to_hebrew(canonical: &str) -> Option<String> {
    let mut chars = canonical.chars();
    let amud = chars.next_back()?;
    let n: u32 = chars.as_str().parse().ok()?;
    let letters = numerals::to_hebrew(n).replace(['"', '\''], "");
    match amud {
        'a' => Some(format!("{letters}.")),
        'b' => Some(format!("{letters}:")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace bans these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn every_notation_for_the_first_daf_of_berakhot_agrees() {
        for written in ["ב.", "ב ע\"א", "ב ע״א", "ב, א", "2a", "2A"] {
            assert_eq!(parse(written).as_deref(), Some("2a"), "{written}");
        }
        for written in ["ב:", "ב ע\"ב", "ב ע״ב", "ב, ב", "2b"] {
            assert_eq!(parse(written).as_deref(), Some("2b"), "{written}");
        }
    }

    #[test]
    fn a_two_letter_daf_reads_correctly() {
        assert_eq!(parse("ט\"ו.").as_deref(), Some("15a"));
        assert_eq!(parse("קכ\"א:").as_deref(), Some("121b"));
    }

    #[test]
    fn there_is_no_daf_one_so_a_bare_first_page_is_refused() {
        // Every masechta starts at ב. A citation reading "daf 1" is a siman
        // that was mistaken for a daf, and guessing would put the reader on
        // another page entirely.
        assert_eq!(parse("א."), None);
        assert_eq!(parse("1a"), None);
    }

    #[test]
    fn a_bare_number_is_not_assumed_to_be_a_daf() {
        // `2` is a siman or a perek far more often than a daf. Which one it is
        // comes from the schema, not from here.
        assert_eq!(parse("2"), None);
        assert_eq!(parse("ב"), None);
    }

    #[test]
    fn a_daf_survives_a_round_trip_through_the_way_it_is_printed() {
        for canonical in ["2a", "2b", "15a", "121b"] {
            let printed = to_hebrew(canonical).expect("prints");
            assert_eq!(parse(&printed).as_deref(), Some(canonical), "{printed}");
        }
    }
}
