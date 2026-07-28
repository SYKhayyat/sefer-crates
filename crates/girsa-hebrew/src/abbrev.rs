//! Abbreviations and rabbinic acronyms, expanded in both directions.
//!
//! A sefer is written one way in its own title page and another way in every
//! footnote that cites it. `שולחן ערוך`, `שו"ע`, `שוע` and `ש"ע` are one book,
//! and a reader who types any of them means the same thing.
//!
//! # Where this table comes from
//!
//! Written from the grammar and from how these are actually printed.
//! `Sefaria-ElasticSearch` has a table like this one and is GPL-3.0; these
//! crates compile into Ksav, which is MIT OR Apache-2.0, so that table may be
//! read to understand *what* is abbreviated and never copied (BUILDER.md T7).
//!
//! This is the small hand-built layer spec.md §9.4 calls for — a few hundred
//! entries that matter in learning, rather than a morphological analyser there
//! is no rabbinic-Hebrew implementation of.
//!
//! # It is not the resolver
//!
//! Turning `שו"ע או"ח א' א'` into a ref is `girsa-ref`'s job, seeded from
//! Sefaria's `titles[]` and `match_templates[]`, which carry 44 variants for
//! that sefer alone. This table answers a smaller question — *what else could
//! this word be written as* — and it answers it for search, where there is no
//! citation to parse.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::normalize::normalize;

/// A table of abbreviations, usable in both directions.
pub struct AbbreviationTable {
    entries: &'static [(&'static str, &'static [&'static str])],
    index: OnceLock<HashMap<String, Vec<&'static str>>>,
}

impl AbbreviationTable {
    /// Build a table over a caller-supplied list. The personal layer uses this
    /// for a reader's own shorthand.
    #[must_use]
    pub const fn new(entries: &'static [(&'static str, &'static [&'static str])]) -> Self {
        Self {
            entries,
            index: OnceLock::new(),
        }
    }

    /// The table as written, for tooling that needs to check it against
    /// something — `examples/harvest.rs` uses this to assert that every entry
    /// is a form the seforim actually use, rather than one somebody assumed.
    #[must_use]
    pub const fn entries(&self) -> &'static [(&'static str, &'static [&'static str])] {
        self.entries
    }

    /// Normalized term → every other way of writing it, in both directions.
    fn index(&self) -> &HashMap<String, Vec<&'static str>> {
        self.index.get_or_init(|| {
            let mut index: HashMap<String, Vec<&'static str>> = HashMap::new();
            for (short, longs) in self.entries {
                for long in *longs {
                    index.entry(normalize(short)).or_default().push(long);
                    index.entry(normalize(long)).or_default().push(short);
                }
            }
            index
        })
    }

    /// Everything `term` could be written as instead.
    ///
    /// The term is matched by its normal form, so `שו"ע` and `שו״ע` are the same
    /// lookup. What comes back is the table's own spelling, verbatim — the
    /// caller is going to show it to a reader, and `שולחן ערוך` reads better
    /// than `שולחנ ערוכ`.
    #[must_use]
    pub fn expansions_of(&self, term: &str) -> Vec<String> {
        let key = normalize(term);
        let mut out: Vec<String> = Vec::new();
        for candidate in self.index().get(&key).into_iter().flatten() {
            let candidate = (*candidate).to_string();
            if normalize(&candidate) != key && !out.contains(&candidate) {
                out.push(candidate);
            }
        }
        out
    }
}

/// [`AbbreviationTable::expansions_of`] against the shipped table.
#[must_use]
pub fn expansions_of(term: &str) -> Vec<String> {
    ABBREVIATIONS.expansions_of(term)
}

/// The shipped table.
pub static ABBREVIATIONS: AbbreviationTable = AbbreviationTable::new(ENTRIES);

/// Abbreviation → every way it is written out.
///
/// Where a sefer has several spellings in circulation, all of them are listed;
/// the table is symmetric, so any entry finds any other.
const ENTRIES: &[(&str, &[&str])] = &[
    // --- seforim ---------------------------------------------------------
    ("שו\"ע", &["שולחן ערוך", "שלחן ערוך", "ש\"ע"]),
    ("או\"ח", &["אורח חיים"]),
    ("יו\"ד", &["יורה דעה"]),
    ("אה\"ע", &["אבן העזר"]),
    ("חו\"מ", &["חושן משפט"]),
    ("מ\"ב", &["משנה ברורה"]),
    ("ב\"י", &["בית יוסף"]),
    ("ט\"ז", &["טורי זהב"]),
    ("ש\"ך", &["שפתי כהן"]),
    ("מג\"א", &["מגן אברהם"]),
    ("פמ\"ג", &["פרי מגדים"]),
    ("ערוה\"ש", &["ערוך השולחן", "ערוך השלחן"]),
    ("קצוה\"ח", &["קצות החושן"]),
    ("נתיה\"מ", &["נתיבות המשפט"]),
    ("שעה\"צ", &["שער הציון"]),
    ("בה\"ל", &["ביאור הלכה"]),
    ("מ\"ת", &["משנה תורה", "יד החזקה"]),
    ("סמ\"ג", &["ספר מצוות גדול"]),
    ("סמ\"ק", &["ספר מצוות קטן"]),
    ("תוס'", &["תוספות"]),
    ("גמ'", &["גמרא"]),
    ("מתני'", &["מתניתין"]),
    ("ברייתא", &["ברייתות"]),
    ("ירו'", &["ירושלמי", "תלמוד ירושלמי"]),
    ("בבלי", &["תלמוד בבלי"]),
    ("מדר\"ר", &["מדרש רבה"]),
    ("ילק\"ש", &["ילקוט שמעוני"]),
    ("שו\"ת", &["שאלות ותשובות"]),
    // --- rabbinic acronyms ------------------------------------------------
    ("רמב\"ם", &["רבינו משה בן מיימון", "רבי משה בן מימון"]),
    ("רמב\"ן", &["רבינו משה בן נחמן", "רבי משה בן נחמן"]),
    ("רש\"י", &["רבי שלמה יצחקי", "רבינו שלמה יצחקי"]),
    ("רשב\"א", &["רבי שלמה בן אדרת"]),
    ("ריטב\"א", &["רבי יום טוב בן אברהם"]),
    ("רשב\"ם", &["רבי שמואל בן מאיר"]),
    ("ראב\"ד", &["רבי אברהם בן דוד"]),
    ("רא\"ש", &["רבינו אשר"]),
    ("ר\"ן", &["רבינו ניסים"]),
    ("רי\"ף", &["רבי יצחק אלפסי"]),
    ("מהרש\"א", &["מורנו הרב שמואל אליעזר"]),
    ("מהר\"ם", &["מורנו הרב מאיר"]),
    ("מהרש\"ל", &["מורנו הרב שלמה לוריא"]),
    ("גר\"א", &["הגאון רבי אליהו", "גאון רבי אליהו"]),
    ("חת\"ם סופר", &["חתם סופר"]),
    ("אדמו\"ר", &["אדוננו מורנו ורבינו"]),
    ("רמ\"א", &["רבי משה איסרליש"]),
    ("שך", &["שפתי כהן"]),
    // --- everyday shorthand in the seforim themselves ----------------------
    ("הקב\"ה", &["הקדוש ברוך הוא"]),
    ("הש\"י", &["השם יתברך"]),
    ("ב\"ה", &["ברוך השם", "בית הלל"]),
    ("ז\"ל", &["זכרונו לברכה"]),
    ("זצ\"ל", &["זכר צדיק לברכה"]),
    ("שליט\"א", &["שיחיה לאורך ימים טובים אמן"]),
    ("ע\"ה", &["עליו השלום"]),
    ("וכו'", &["וכולי"]),
    ("וגו'", &["וגומר"]),
    ("כמ\"ש", &["כמו שכתוב", "כמו שאמר"]),
    ("אחז\"ל", &["אמרו חכמינו זכרונם לברכה"]),
    ("חז\"ל", &["חכמינו זכרונם לברכה"]),
    ("ע\"פ", &["על פי"]),
    ("ע\"י", &["על ידי"]),
    ("עי\"ז", &["על ידי זה"]),
    ("עי'", &["עיין"]),
    ("ע\"ש", &["עיין שם"]),
    ("ד\"ה", &["דיבור המתחיל"]),
    ("ס\"ד", &["סלקא דעתך"]),
    ("ג\"כ", &["גם כן"]),
    ("משא\"כ", &["מה שאין כן"]),
    ("אח\"כ", &["אחר כך"]),
    ("ולפ\"ז", &["ולפי זה"]),
    ("עוה\"ז", &["העולם הזה"]),
    ("עוה\"ב", &["העולם הבא"]),
    ("עש\"ק", &["ערב שבת קודש"]),
    ("שב\"ק", &["שבת קודש"]),
    ("ר\"ה", &["ראש השנה"]),
    ("יו\"ט", &["יום טוב"]),
    ("יו\"כ", &["יום כיפור", "יום הכיפורים"]),
    ("ר\"ח", &["ראש חודש"]),
    ("ק\"ש", &["קריאת שמע"]),
    ("ת\"ת", &["תלמוד תורה"]),
    ("בנ\"י", &["בני ישראל"]),
    ("ארה\"ק", &["ארץ הקודש"]),
    ("בעה\"ב", &["בעל הבית"]),
    ("ב\"ד", &["בית דין"]),
    ("בי\"ד", &["בית דין"]),
    ("ד\"ת", &["דברי תורה"]),
    ("ל\"ת", &["לא תעשה"]),
    ("מ\"ע", &["מצות עשה"]),
    ("פ\"ד", &["פרק ד"]),
    ("סי'", &["סימן"]),
    ("סע'", &["סעיף"]),
    ("הל'", &["הלכות"]),
    ("פ'", &["פרשת", "פרק"]),
    ("מס'", &["מסכת"]),
    ("דף", &["דף"]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_runs_in_both_directions() {
        assert!(expansions_of("שו\"ע").contains(&"שולחן ערוך".to_string()));
        assert!(expansions_of("שולחן ערוך").contains(&"שו\"ע".to_string()));
    }

    #[test]
    fn a_lookup_does_not_care_which_gershayim_was_typed() {
        assert_eq!(expansions_of("שו\"ע"), expansions_of("שו״ע"));
        assert_eq!(expansions_of("רמב\"ם"), expansions_of("רמב״ם"));
    }

    #[test]
    fn a_final_letter_does_not_hide_an_entry() {
        // רמב"ם ends in a final mem; the table is keyed on the normal form.
        assert!(!expansions_of("רמב\"ם").is_empty());
    }

    #[test]
    fn an_unknown_term_expands_to_nothing_rather_than_a_guess() {
        assert!(expansions_of("קרןאורה").is_empty());
    }

    #[test]
    fn two_seforim_sharing_an_abbreviation_both_come_back() {
        // ב"ה is both "ברוך השם" and "בית הלל". Returning one of them would be
        // a guess, which is the thing this system is not allowed to do.
        let both = expansions_of("ב\"ה");
        assert!(both.contains(&"ברוך השם".to_string()));
        assert!(both.contains(&"בית הלל".to_string()));
    }
}
