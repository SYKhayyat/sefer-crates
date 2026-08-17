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
/// The first rule is how numerals are *written*: **high to low**. `קכ"א` is
/// 100, 20, 1. `תרצ"ז` is 400, 200, 90, 7. A numeral never goes back up, and a
/// word does almost immediately — `שבת` is 300, 2, 400, and the 2 gives it
/// away.
///
/// # Descending is necessary and it is not sufficient
///
/// Plenty of words descend. `ייחוד` is 10, 10, 8, 6, 4 and never goes back up,
/// so it read as 38 — and `ברכות שער ייחוד המעשה ב'` resolved to
/// `38:המעשה:2`, which is the exact failure this crate exists to refuse. The
/// descent rule let it through because it was written to allow an equal pair,
/// for one case: 800 is `תת`.
///
/// The second rule closes it, and it is a stronger statement of the same idea:
/// **a numeral is the canonical spelling of its own value.** Twenty is `כ`, so
/// `יו"ד` — 10, 6, 4 — is not twenty written some other way, it is not a
/// numeral at all. Nobody reaches for a smaller letter when a bigger one covers
/// the amount, which is what makes the spelling of a number unique and what
/// makes this test decide rather than guess.
///
/// | said | sums to | written | read as |
/// |---|---|---|---|
/// | `יו"ד` | 20 | `כ'` | nothing — it is Yoreh De'ah |
/// | `ייחוד` | 38 | `ל"ח` | nothing |
/// | `בא` | 3 | `ג'` | nothing — it is the parsha |
/// | `תת` | 800 | `תת` | 800 |
/// | `ט"ו` | 15 | `ט"ו` | 15 |
///
/// By construction it cannot reject a real numeral: every number this crate
/// writes with [`to_hebrew`] is canonical, and `every_number_a_sefer_could_have`
/// walks 1 to 20,000 through both directions to say so.
///
/// # What it still does not settle
///
/// `נח` is 50, 8 — which is exactly how 58 is written. The parsha and the
/// number are the same string, and no rule about spelling can separate them;
/// that needs to know which strings are words, which is a lexicon. Stated here
/// so the next reader does not go looking for a rule that cannot exist.
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
///
/// Both rules, in the order that costs least: the descent is a comparison per
/// letter and throws out most words before anything is allocated, and the
/// canonical check then throws out the ones that descend anyway.
///
/// The comparison is on **values** rather than on characters, so that a final
/// letter compares equal to its medial form — `ך` is twenty and `כ` is how
/// twenty is written, and they are the same numeral said two ways.
fn read_descending(s: &str) -> Option<u32> {
    let letters: Vec<char> = s
        .chars()
        .filter(|c| *c != '\'' && *c != '"' && !c.is_whitespace())
        .collect();
    if letters.is_empty() {
        return None;
    }

    let mut said = Vec::with_capacity(letters.len());
    let mut total = 0u32;
    let mut previous = u32::MAX;
    for c in letters {
        let value = value_of(c)?;
        if value > previous {
            return None;
        }
        previous = value;
        total = total.checked_add(value)?;
        said.push(value);
    }

    let canonical: Vec<u32> = to_bare_letters(total)
        .chars()
        .filter_map(value_of)
        .collect();
    if canonical != said {
        return None;
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

/// The value at which the thousands notation starts.
pub const THOUSAND: u32 = 1000;

/// Write a number the way a sefer does. 121 → `קכ"א`, 1,999 → `א'תתקצ"ט`.
///
/// The gershayim goes before the last letter, or a geresh after a single one,
/// which is how it is printed. `15` and `16` come out `טו` and `טז` because the
/// alternative spells a Name.
///
/// # Thousands
///
/// The notation is the letter for *how many* thousands, a geresh, then the
/// remainder: `א'תתקצ"ט` is 1,999. `parse_hebrew` has always read that form; this
/// function used to give up at 1,000 and return the Arabic digits, so a citation
/// went from `סימן תתקצ"ט` to `סימן 1000` with no indication that the alphabet had
/// changed underneath it — in a formatter whose whole promise is *"how it is
/// written in a sefer"*.
///
/// **The reasoning that justified giving up was sound and its premise was wrong.**
/// It said no address level ever reaches four digits, because the longest masechta
/// is 176 dafim and the largest siman count in the corpus is Orach Chayim's 697.
/// Measured over the real corpus instead: **43,076 of 5,000,545 addresses (0.86%)
/// carry a component ≥ 1,000**, the first of them
/// `girsa:bavli/maadaney-yom-tov-on-berakhot/1000`.
///
/// # A round thousand is still written as digits, and that is not a lapse
///
/// 1,000 in the notation is `א'` — which is also how 1 is written. Hebrew does not
/// disambiguate the two; a reader uses context, and a ref has none. This crate's
/// governing rule is that a wrong ref is worse than no ref, and a citation that
/// might mean siman 1 or siman 1,000 is precisely the ambiguity it refuses to
/// resolve by guessing. So a round thousand — and anything from a million up, where
/// the thousands count would itself need the notation — is written in digits, which
/// is unambiguous and round-trips. That is one address in a thousand of the 43,076
/// rather than all of them.
///
/// [`is_written_in_digits`] answers *which* of the two a number gets, so a caller
/// that wants to say so can.
#[must_use]
pub fn to_hebrew(n: u32) -> String {
    if n == 0 {
        return String::new();
    }
    if is_written_in_digits(n) {
        return n.to_string();
    }
    if n < THOUSAND {
        return with_marks(&to_bare_letters(n));
    }
    // The geresh resets the descent, which is what lets `parse_hebrew` read the two
    // halves separately — and it has read them separately all along.
    format!(
        "{}'{}",
        to_bare_letters(n / THOUSAND),
        with_marks(&to_bare_letters(n % THOUSAND))
    )
}

/// Whether a number comes out as digits rather than as letters, and why.
///
/// Two cases, both of them ambiguity rather than laziness: a round thousand reads
/// as its own thousands count (1,000 and 1 are both `א'`), and from a million up
/// the thousands count would need the notation itself, which nests and stops being
/// readable.
#[must_use]
pub fn is_written_in_digits(n: u32) -> bool {
    n >= THOUSAND && (n % THOUSAND == 0 || n / THOUSAND >= THOUSAND)
}

/// The mark that distinguishes a numeral from a word.
///
/// `קכ"א` is a number, `קכא` is a typo. Inside the numeral for two letters or
/// more, after it for one.
fn with_marks(letters: &str) -> String {
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

    /// The words that descend, which the descent rule alone let through.
    ///
    /// `ייחוד` is 10, 10, 8, 6, 4 — it never goes back up, so it summed to 38
    /// and `ברכות שער ייחוד המעשה ב'` resolved to `38:המעשה:2`. `יו"ד` is 10,
    /// 6, 4 and summed to 20, so Yoreh De'ah arrived as a number and the
    /// corpus needed a guard to put it back. Neither is written the way its
    /// own total is written, and that is the whole of the test.
    #[test]
    fn a_word_that_descends_is_still_not_a_number() {
        for (word, sums_to, written) in [
            ("ייחוד", 38, "ל\"ח"),
            ("יו\"ד", 20, "כ'"),
            ("יוד", 20, "כ'"),
            ("בא", 3, "ג'"),
            ("שממה", 385, "שפ\"ה"),
        ] {
            assert_eq!(parse(word), None, "{word} was read as {sums_to}");
            // And the number it summed to is written some other way, which is
            // the reason the word is not it.
            assert_eq!(to_hebrew(sums_to), written);
        }
    }

    /// The one this deliberately cannot separate, so nobody goes looking.
    ///
    /// `נח` is 50, 8 — exactly how 58 is written. The parsha and the number
    /// are the same string and no rule about spelling tells them apart.
    #[test]
    fn a_word_written_exactly_as_its_own_number_is_read_as_the_number() {
        assert_eq!(parse("נח"), Some(58));
        assert_eq!(to_hebrew(58), "נ\"ח");
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
        // The four-digit range is not hypothetical. Measured over the real corpus,
        // 43,076 of 5,000,545 addresses carry a component ≥ 1,000, the first of
        // them `girsa:bavli/maadaney-yom-tov-on-berakhot/1000`. Twenty thousand
        // covers every one of them with room to spare.
        for n in 1..=20_000u32 {
            let written = to_hebrew(n);
            assert_eq!(parse(&written), Some(n), "{n} was written {written}");
        }
    }

    /// Above a thousand the citation stays in Hebrew, which it did not.
    ///
    /// `to_hebrew` gave up at 1,000 and returned the Arabic digits, so a citation
    /// went from `מעדני יום טוב על ברכות סימן תתקצ"ט` to `… סימן 1000` with the
    /// alphabet changing underneath it and nothing saying so.
    #[test]
    fn a_number_past_a_thousand_is_still_written_in_letters() {
        assert_eq!(to_hebrew(999), "תתקצ\"ט");
        assert_eq!(to_hebrew(1001), "א'א'");
        assert_eq!(to_hebrew(1005), "א'ה'");
        assert_eq!(to_hebrew(1121), "א'קכ\"א");
        assert_eq!(to_hebrew(1999), "א'תתקצ\"ט");
        assert_eq!(to_hebrew(2500), "ב'ת\"ק");
        assert_eq!(to_hebrew(5786), "ה'תשפ\"ו"); // this year, as a sefer writes it
                                                 // And no Latin digit anywhere in any of them.
        for n in [1001, 1005, 1121, 1999, 2500, 5786] {
            let written = to_hebrew(n);
            assert!(
                !written.chars().any(char::is_numeric),
                "{n} came out as {written}"
            );
        }
    }

    /// The one case that stays in digits, and the reason it does.
    ///
    /// 1,000 in the notation is `א'`, which is also how 1 is written. Hebrew does
    /// not disambiguate them and a ref has no context to disambiguate from, so this
    /// crate refuses rather than guesses — the same rule that stops it inventing an
    /// abbreviation or a level word.
    #[test]
    fn a_round_thousand_is_ambiguous_and_is_therefore_written_in_digits() {
        assert_eq!(to_hebrew(1), "א'");
        assert_eq!(parse("א'"), Some(1));
        assert!(is_written_in_digits(1000));
        assert_eq!(to_hebrew(1000), "1000");
        assert_eq!(to_hebrew(2000), "2000");
        assert_eq!(parse("1000"), Some(1000));
        // Not the round thousands, and not below the ceiling.
        assert!(!is_written_in_digits(999));
        assert!(!is_written_in_digits(1001));
        // And a million up, where the thousands count would need the notation
        // itself and stop being readable.
        assert!(is_written_in_digits(1_000_001));
        assert_eq!(to_hebrew(1_000_001), "1000001");
    }
}
