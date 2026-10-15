//! The level of an English word, from two open lists that grade words by the
//! CEFR: the CEFR-J Wordlist (A1–B2) and the Octanove Vocabulary Profile
//! (C1–C2), in `data/levels/` with their notices. They grade words, not
//! meanings, and a word under each part of speech it has; a word takes the
//! lowest level any of its forms is listed at.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

/// A CEFR level, ordered A1 < … < C2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Level {
    A1,
    A2,
    B1,
    B2,
    C1,
    C2,
}

impl Level {
    pub const ALL: [Level; 6] = [Level::A1, Level::A2, Level::B1, Level::B2, Level::C1, Level::C2];

    /// Its place on the scale the measures use: A1 is 1, C2 is 6.
    pub fn rank(self) -> f64 {
        (self as u8 + 1) as f64
    }

    /// The level a place on the scale falls in: 3.0 up to 4.0 is B1.
    pub fn of_rank(rank: f64) -> Level {
        let at = (rank.floor() as i64 - 1).clamp(0, 5) as usize;
        Level::ALL[at]
    }

    pub fn parse(text: &str) -> Option<Level> {
        Level::ALL
            .into_iter()
            .find(|level| format!("{level:?}").eq_ignore_ascii_case(text.trim()))
    }
}

const CEFRJ: &str = include_str!("../../../data/levels/cefrj-vocabulary-profile-1.5.csv");
const OCTANOVE: &str = include_str!("../../../data/levels/octanove-vocabulary-profile-c1c2-1.0.csv");

static WORDS: LazyLock<HashMap<String, Level>> = LazyLock::new(|| {
    let mut words = HashMap::new();
    for list in [CEFRJ, OCTANOVE] {
        for row in list.lines().skip(1) {
            // Only later columns are ever quoted: the headword, its part of
            // speech and its level are the first three.
            let mut fields = row.splitn(4, ',');
            let (Some(heads), Some(_), Some(level)) = (fields.next(), fields.next(), fields.next()) else {
                continue;
            };
            let Some(level) = Level::parse(level) else { continue };
            for head in heads.split('/') {
                let head = head.trim().to_lowercase();
                if !head.is_empty() {
                    let at = words.entry(head).or_insert(level);
                    *at = (*at).min(level);
                }
            }
        }
    }
    words
});

/// Past forms the suffix rules can't undo, and plurals and comparisons.
const IRREGULAR: &[(&str, &str)] = &[
    ("was", "be"),
    ("were", "be"),
    ("been", "be"),
    ("am", "be"),
    ("is", "be"),
    ("are", "be"),
    ("did", "do"),
    ("done", "do"),
    ("does", "do"),
    ("had", "have"),
    ("has", "have"),
    ("went", "go"),
    ("gone", "go"),
    ("got", "get"),
    ("gotten", "get"),
    ("made", "make"),
    ("said", "say"),
    ("saw", "see"),
    ("seen", "see"),
    ("came", "come"),
    ("took", "take"),
    ("taken", "take"),
    ("knew", "know"),
    ("known", "know"),
    ("thought", "think"),
    ("told", "tell"),
    ("found", "find"),
    ("gave", "give"),
    ("given", "give"),
    ("felt", "feel"),
    ("left", "leave"),
    ("kept", "keep"),
    ("brought", "bring"),
    ("bought", "buy"),
    ("began", "begin"),
    ("begun", "begin"),
    ("ran", "run"),
    ("sat", "sit"),
    ("stood", "stand"),
    ("understood", "understand"),
    ("lost", "lose"),
    ("paid", "pay"),
    ("met", "meet"),
    ("heard", "hear"),
    ("meant", "mean"),
    ("wore", "wear"),
    ("worn", "wear"),
    ("wrote", "write"),
    ("written", "write"),
    ("ate", "eat"),
    ("eaten", "eat"),
    ("drank", "drink"),
    ("drunk", "drink"),
    ("spoke", "speak"),
    ("spoken", "speak"),
    ("broke", "break"),
    ("broken", "break"),
    ("chose", "choose"),
    ("chosen", "choose"),
    ("forgot", "forget"),
    ("forgotten", "forget"),
    ("fell", "fall"),
    ("fallen", "fall"),
    ("flew", "fly"),
    ("flown", "fly"),
    ("grew", "grow"),
    ("grown", "grow"),
    ("threw", "throw"),
    ("thrown", "throw"),
    ("drove", "drive"),
    ("driven", "drive"),
    ("rode", "ride"),
    ("ridden", "ride"),
    ("slept", "sleep"),
    ("built", "build"),
    ("sent", "send"),
    ("spent", "spend"),
    ("caught", "catch"),
    ("taught", "teach"),
    ("fought", "fight"),
    ("sold", "sell"),
    ("held", "hold"),
    ("led", "lead"),
    ("hid", "hide"),
    ("hidden", "hide"),
    ("won", "win"),
    ("woke", "wake"),
    ("children", "child"),
    ("men", "man"),
    ("women", "woman"),
    ("people", "person"),
    ("feet", "foot"),
    ("teeth", "tooth"),
    ("mice", "mouse"),
    ("better", "good"),
    ("best", "good"),
    ("worse", "bad"),
    ("worst", "bad"),
];

/// Endings taken off to find the form a word is listed under, and what
/// goes back in their place.
const SUFFIXES: &[(&str, &str)] = &[
    ("ies", "y"),
    ("ied", "y"),
    ("es", ""),
    ("s", ""),
    ("ed", ""),
    ("ed", "e"),
    ("d", ""),
    ("ing", ""),
    ("ing", "e"),
    ("er", ""),
    ("est", ""),
    ("ly", ""),
    ("ier", "y"),
    ("iest", "y"),
    ("ily", "y"),
    ("ness", ""),
    ("ful", ""),
];

/// The word and the forms it may be listed under: *stopped* → *stop*,
/// *going* → *go*, *went* → *go*.
pub fn base_forms(word: &str) -> Vec<String> {
    let word = word.to_lowercase();
    let mut forms = vec![word.clone()];
    if let Some((_, base)) = IRREGULAR.iter().find(|(form, _)| *form == word) {
        forms.push(base.to_string());
    }
    for (suffix, put) in SUFFIXES {
        // A stem of two letters is a word (go-ing, be-ing), never one.
        if let Some(stem) = word.strip_suffix(suffix).filter(|stem| stem.chars().count() >= 2) {
            let stem = format!("{stem}{put}");
            let doubled =
                put.is_empty() && stem.len() > 2 && stem.as_bytes()[stem.len() - 1] == stem.as_bytes()[stem.len() - 2];
            if doubled {
                forms.push(stem[..stem.len() - 1].to_string());
            }
            forms.push(stem);
        }
    }
    forms
}

/// The level a word is listed at, by its lowest-listed form; `None` for a
/// word the lists don't have (a name, slang, a rare word).
pub fn word_level(word: &str) -> Option<Level> {
    base_forms(word)
        .iter()
        .filter_map(|form| WORDS.get(form))
        .min()
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_takes_the_level_of_its_lowest_listed_form() {
        assert_eq!(word_level("going"), Some(Level::A1));
        assert_eq!(word_level("stopped"), Some(Level::A1));
        assert_eq!(word_level("went"), Some(Level::A1));
        assert_eq!(word_level("Saving"), Some(Level::A1));
        assert_eq!(word_level("grieving"), Some(Level::B2));
        assert_eq!(word_level("Pasadena"), None);
    }

    #[test]
    fn a_rank_falls_in_its_level() {
        assert_eq!(Level::of_rank(3.0), Level::B1);
        assert_eq!(Level::of_rank(3.99), Level::B1);
        assert_eq!(Level::of_rank(4.0), Level::B2);
        assert_eq!(Level::of_rank(0.2), Level::A1);
        assert_eq!(Level::of_rank(9.0), Level::C2);
    }
}
