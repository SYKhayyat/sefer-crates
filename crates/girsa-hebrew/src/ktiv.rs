//! Ktiv male ↔ ktiv chaser.
//!
//! `כהן` and `כוהן` are one word spelled two ways: the `ו` is a *mater lectionis*
//! — a consonant standing in for a vowel because the vowel itself was not
//! written. Whether a given edition spells it in or out is an editorial habit,
//! not a difference in the word, and the corpus is inconsistent about it in
//! exactly the way that makes a search silently miss.
//!
//! Without nikud there is no way to tell a mater lectionis from a real `ו`
//! (`שור`, an ox, is not `שר`, a minister). So this generates *candidates* and
//! never rewrites: a spelling that no sefer contains simply matches nothing.

use crate::marks::is_hebrew_letter;

/// The two letters that do double duty as vowels.
const MATRES: [char; 2] = ['ו', 'י'];

/// Long enough that adding a letter is plausible; short enough that the
/// candidate set stays small. A word this long is a compound or a mistake.
const MAX_LETTERS: usize = 12;

/// Every spelling of a word that differs only in its matres lectionis.
///
/// Both directions: `כהן` yields `כוהן`, and `כוהן` yields `כהן`.
///
/// A first or last letter is never touched. A leading `ו` is a conjunction and
/// belongs to [`crate::peelings`]; a trailing `ו` or `י` is usually a suffix
/// that carries meaning — `רבו`, *his teacher*, is not `רב*`.
pub(crate) fn spellings(word: &str) -> Vec<String> {
    let letters: Vec<char> = word.chars().collect();
    if letters.len() < 3 || letters.len() > MAX_LETTERS {
        return Vec::new();
    }
    if !letters.iter().all(|c| is_hebrew_letter(*c)) {
        return Vec::new();
    }

    let mut out: Vec<String> = Vec::new();

    // Chaser: take an internal mater out.
    for i in 1..letters.len() - 1 {
        if MATRES.contains(&letters[i]) {
            let mut candidate = letters.clone();
            candidate.remove(i);
            push_unique(&mut out, candidate.into_iter().collect(), word);
        }
    }

    // Male: put one in.
    for i in 1..letters.len() {
        for m in MATRES {
            // Doubling an existing mater produces a spelling nobody writes.
            if letters[i - 1] == m || letters.get(i) == Some(&m) {
                continue;
            }
            let mut candidate = letters.clone();
            candidate.insert(i, m);
            push_unique(&mut out, candidate.into_iter().collect(), word);
        }
    }

    out
}

fn push_unique(out: &mut Vec<String>, candidate: String, original: &str) {
    if candidate != original && !out.contains(&candidate) {
        out.push(candidate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_male_spelling_is_reachable_from_the_chaser_one() {
        assert!(spellings("כהנ").contains(&"כוהנ".to_string()));
    }

    #[test]
    fn the_chaser_spelling_is_reachable_from_the_male_one() {
        assert!(spellings("כוהנ").contains(&"כהנ".to_string()));
    }

    #[test]
    fn a_trailing_letter_is_left_alone() {
        // רבו is "his teacher". Dropping the ו changes the word.
        assert!(!spellings("רבו").contains(&"רב".to_string()));
    }

    #[test]
    fn a_word_carrying_punctuation_is_not_a_candidate() {
        // שו"ע is an abbreviation. Its ו is not a mater lectionis, and
        // rewriting it would be nonsense.
        assert!(spellings("שו\"ע").is_empty());
    }
}
