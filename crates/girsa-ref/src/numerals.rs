//! Hebrew numerals, both directions.
//!
//! Seforim number their own divisions in letters: `סימן קכ"א` is siman 121,
//! `פ"ד ה"א` is perek 4 halacha 1. Every citation in the corpus is written this
//! way, so a resolver that only reads `121` cannot read a sefer.
//!
//! The rule is addition — each letter is worth something and the word is the
//! sum — which is why `טו` is 15 and not `יה`: the obvious spelling of fifteen
//! would be two letters of the Name. That convention costs nothing to support
//! here, because 9 + 6 already comes to 15.

use girsa_hebrew::normalize;

/// Letter values. Finals are worth what their medial form is worth — the shape
/// is positional, not numeric.
const VALUES: [(char, u32); 27] = [
    ('א', 1),
    ('ב', 2),
    ('ג', 3),
    ('ד', 4),
    ('ה', 5),
    ('ו', 6),
    ('ז', 7),
    ('ח', 8),
    ('ט', 9),
    ('י', 10),
    ('כ', 20),
    ('ך', 20),
    ('ל', 30),
    ('מ', 40),
    ('ם', 40),
    ('נ', 50),
    ('ן', 50),
    ('ס', 60),
    ('ע', 70),
    ('פ', 80),
    ('ף', 80),
    ('צ', 90),
    ('ץ', 90),
    ('ק', 100),
    ('ר', 200),
    ('ש', 300),
    ('ת', 400),
];

/// Descending, for writing a number out.
const WRITING: [(u32, char); 22] = [
    (400, 'ת'),
    (300, 'ש'),
    (200, 'ר'),
    (100, 'ק'),
    (90, 'צ'),
    (80, 'פ'),
    (70, 'ע'),
    (60, 'ס'),
    (50, 'נ'),
    (40, 'מ'),
    (30, 'ל'),
    (20, 'כ'),
    (10, 'י'),
    (9, 'ט'),
    (8, 'ח'),
    (7, 'ז'),
    (6, 'ו'),
    (5, 'ה'),
    (4, 'ד'),
    (3, 'ג'),
    (2, 'ב'),
    (1, 'א'),
];

fn value_of(c: char) -> Option<u32> {
    VALUES.iter().find(|(l, _)| *l == c).map(|(_, v)| *v)
}

/// Read a Hebrew numeral. `קכ"א` → 121, `ב` → 2, `טו` → 15.
///
/// # Every Hebrew word is a number if you let it
///
/// `שבת` sums to 702 and `ברכות` to 628. A resolver that summed whatever it was
/// handed would read `ברכות שבת` as Berakhot siman 702 — a citation that
/// resolves, opens a page, and is wrong. That is the failure mode this whole
/// crate is built to avoid, so summing is not enough.
///
/// The rule that separates them is how numerals are *written*: **high to low**.
/// `קכ"א` is 100, 20, 1. `תרצ"ז` is 400, 200, 90, 7. A numeral never goes back
/// up, and a word does almost immediately — `שבת` is 300, 2, 400, and the 2
/// gives it away.
///
/// Equal is allowed, because 800 is `תת`.
#[must_use]
pub fn parse_hebrew(s: &str) -> Option<u32> {
    let normalized = normalize(s);

    // Thousands are written as the count of thousands, a geresh, then the rest:
    // `א'תתקצ"ט` is 1,999. The geresh resets the descent, so the two halves are
    // read separately.
    if let Some((thousands, rest)) = normalized.split_once('\'') {
        if !rest.trim().is_empty() {
            let thousands = read_descending(thousands)?;
            let rest = read_descending(rest)?;
            return thousands.checked_mul(1000)?.checked_add(rest);
        }
    }

    read_descending(&normalized)
}

/// Sum the letters, refusing anything that is not written the way a numeral is.
fn read_descending(s: &str) -> Option<u32> {
    let letters: Vec<char> = s
        .chars()
        .filter(|c| *c != '\'' && *c != '"' && !c.is_whitespace())
        .collect();
    if letters.is_empty() {
        return None;
    }

    let mut total = 0u32;
    let mut previous = u32::MAX;
    for c in letters {
        let value = value_of(c)?;
        if value > previous {
            return None;
        }
        previous = value;
        total = total.checked_add(value)?;
    }
    Some(total)
}

/// Read whichever way the number was written — `121`, `קכא`, `קכ"א`.
#[must_use]
pub fn parse(s: &str) -> Option<u32> {
    let trimmed = s.trim();
    if let Ok(n) = trimmed.parse::<u32>() {
        return Some(n);
    }
    parse_hebrew(trimmed)
}

/// Write a number the way a sefer does. 121 → `קכ"א`.
///
/// The gershayim goes before the last letter, or a geresh after a single one,
/// which is how it is printed. `15` and `16` come out `טו` and `טז` because the
/// alternative spells a Name.
///
/// # A round thousand is written as digits, deliberately
///
/// The notation for a thousand is the letter for *how many* thousands followed
/// by a geresh — so 1,000 is `א'`, which is also how 1 is written. In prose a
/// reader disambiguates from context; a ref has no context, and a citation that
/// might mean siman 1 or siman 1,000 is exactly the ambiguity this crate
/// refuses to resolve by guessing.
///
/// It costs nothing to avoid: the longest masechta is 176 dafim and the largest
/// siman count in the corpus is Orach Chayim's 697, so no address level ever
/// reaches four digits. Above 999 the digits are written out, which is
/// unambiguous and round-trips.
#[must_use]
pub fn to_hebrew(n: u32) -> String {
    if n == 0 {
        return String::new();
    }
    if n >= 1000 {
        return n.to_string();
    }

    let letters = to_bare_letters(n);

    // The mark lands inside the numeral, which is what distinguishes it from a
    // word: `קכ"א` is a number, `קכא` is a typo.
    let chars: Vec<char> = letters.chars().collect();
    match chars.len() {
        0 => String::new(),
        1 => format!("{}'", chars[0]),
        _ => {
            let (head, last) = chars.split_at(chars.len() - 1);
            format!("{}\"{}", head.iter().collect::<String>(), last[0])
        }
    }
}

fn to_bare_letters(mut n: u32) -> String {
    let mut out = String::new();
    // 15 and 16 would come out יה and יו, which are read as a Name, so they are
    // written from 9 instead: ט+ו and ט+ז.
    while n > 0 {
        if n == 15 {
            out.push_str("טו");
            break;
        }
        if n == 16 {
            out.push_str("טז");
            break;
        }
        let Some((value, letter)) = WRITING.iter().find(|(v, _)| *v <= n) else {
            break;
        };
        out.push(*letter);
        n -= value;
    }
    out
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace bans these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_siman_number_reads_the_way_it_is_printed() {
        assert_eq!(parse("קכ\"א"), Some(121));
        assert_eq!(parse("קכא"), Some(121));
        assert_eq!(parse("121"), Some(121));
        assert_eq!(parse("א'"), Some(1));
        assert_eq!(parse("תרצ\"ז"), Some(697)); // the last siman of Orach Chayim
    }

    #[test]
    fn fifteen_and_sixteen_are_written_the_way_they_are_written() {
        assert_eq!(to_hebrew(15), "ט\"ו");
        assert_eq!(to_hebrew(16), "ט\"ז");
        assert_eq!(parse("ט\"ו"), Some(15));
        assert_eq!(parse("ט\"ז"), Some(16));
    }

    #[test]
    fn a_final_letter_is_worth_what_its_ordinary_form_is_worth() {
        assert_eq!(parse_hebrew("ך"), Some(20));
        assert_eq!(parse_hebrew("ם"), Some(40));
    }

    #[test]
    fn gershayim_written_any_of_its_ways_does_not_change_the_number() {
        assert_eq!(parse("קכ\"א"), parse("קכ״א"));
        assert_eq!(parse("קכ\"א"), parse("קכ”א"));
    }

    #[test]
    fn a_word_is_not_read_as_a_number() {
        // Every one of these sums to something. A resolver that summed them
        // would resolve `ברכות שבת` to Berakhot siman 702 — a citation that
        // opens a page, and the wrong one.
        for word in ["שבת", "ברכות", "אמת", "סימן", "סעיף", "משה", "תורה"]
        {
            assert_eq!(parse(word), None, "{word} was read as a number");
        }
        assert_eq!(parse("Berakhot"), None);
        assert_eq!(parse(""), None);
        assert_eq!(parse("  "), None);
    }

    #[test]
    fn a_numeral_is_still_a_numeral() {
        // The descending rule must not throw out the thing it is protecting.
        for (written, n) in [
            ("א", 1),
            ("קכא", 121),
            ("תרצז", 697),
            ("טו", 15),
            ("תת", 800),
            ("תתקצט", 999),
        ] {
            assert_eq!(parse(written), Some(n), "{written}");
        }
    }

    #[test]
    fn every_number_a_sefer_could_have_survives_a_round_trip() {
        // Shulchan Arukh Orach Chayim runs to siman 697 and 4,171 se'ifim in
        // total; the longest masechta is 176 dafim. 5,000 is well past anything
        // a citation can carry.
        for n in 1..=5000u32 {
            let written = to_hebrew(n);
            assert_eq!(parse(&written), Some(n), "{n} was written {written}");
        }
    }
}
