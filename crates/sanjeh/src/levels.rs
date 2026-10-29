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

/// Every word the lists have and the part of speech it is listed as, with
/// its level.
static LISTED: LazyLock<Vec<(String, String, Level)>> = LazyLock::new(|| {
    let mut listed = vec![];
    for list in [CEFRJ, OCTANOVE] {
        for row in list.lines().skip(1) {
            let mut fields = row.splitn(4, ',');
            let (Some(heads), Some(pos), Some(level)) = (fields.next(), fields.next(), fields.next()) else {
                continue;
            };
            let Some(level) = Level::parse(level) else { continue };
            for head in heads.split('/') {
                let head = head.trim().to_lowercase();
                if !head.is_empty() {
                    listed.push((head, pos.trim().to_string(), level));
                }
            }
        }
    }
    listed
});

/// Number words and the levels the vocabulary lists give them.
static NUMBER_WORDS: LazyLock<HashMap<String, Level>> = LazyLock::new(|| {
    let mut numbers = HashMap::new();

    for (head, pos, level) in LISTED.iter() {
        if pos != "number" {
            continue;
        }

        let at = numbers.entry(head.clone()).or_insert(*level);
        *at = (*at).min(*level);
    }

    numbers
});

/// Every word the lists have, and every form its part of speech takes by
/// the spelling rules — a noun's plural, a verb's *-s*, *-ed* and *-ing*,
/// an adjective's *-er* and *-est* — and the irregular forms below, each at
/// the lowest level it comes from.
static FORMS: LazyLock<HashMap<String, Level>> = LazyLock::new(|| {
    let mut forms: HashMap<String, Level> = HashMap::new();
    let put = |forms: &mut HashMap<String, Level>, form: String, level: Level| {
        let at = forms.entry(form).or_insert(level);
        *at = (*at).min(level);
    };
    for (head, pos, level) in LISTED.iter() {
        put(&mut forms, head.clone(), *level);
        if head.contains(' ') {
            continue;
        }
        let made: Vec<String> = match pos.as_str() {
            "noun" => vec![with_s(head)],
            "verb" => [vec![with_s(head)], with_ending(head, "ed"), with_ending(head, "ing")].concat(),
            "adjective" => [with_ending(head, "er"), with_ending(head, "est")].concat(),
            _ => vec![],
        };
        for form in made {
            put(&mut forms, form, *level);
        }
    }
    for (form, base) in IRREGULAR {
        if let Some(level) = forms.get(*base).copied() {
            put(&mut forms, form.to_string(), level);
        }
    }
    forms
});

fn is_vowel(c: u8) -> bool {
    matches!(c, b'a' | b'e' | b'i' | b'o' | b'u')
}

/// Whether a word ends consonant, vowel, consonant, as *stop* and *big*
/// do, and may double its last letter before an ending that starts with a
/// vowel. Whether it does depends on its stress (*admitted*, *visited*),
/// which the spelling doesn't say: both are made, and a spelling no word
/// has is never asked for.
fn may_double(word: &str) -> bool {
    let b = word.as_bytes();
    let n = b.len();
    n >= 3
        && !is_vowel(b[n - 1])
        && !matches!(b[n - 1], b'w' | b'x' | b'y')
        && is_vowel(b[n - 2])
        && !is_vowel(b[n - 3])
}

fn ends_consonant_y(word: &str) -> bool {
    let b = word.as_bytes();
    b.len() >= 2 && b[b.len() - 1] == b'y' && !is_vowel(b[b.len() - 2])
}

fn with_s(word: &str) -> String {
    if ends_consonant_y(word) {
        format!("{}ies", &word[..word.len() - 1])
    } else if ["s", "x", "z", "ch", "sh", "o"].iter().any(|end| word.ends_with(end)) {
        format!("{word}es")
    } else {
        format!("{word}s")
    }
}

/// A word with an ending that starts with a vowel — *-ed*, *-ing*, *-er*,
/// *-est* — by the spelling rules: *liked*, *tried*, *lying*, *bigger*.
fn with_ending(word: &str, ending: &str) -> Vec<String> {
    let mut made = vec![];
    if ending == "ing" {
        if let Some(stem) = word.strip_suffix("ie") {
            made.push(format!("{stem}ying"));
        } else if word.ends_with('e') && !word.ends_with("ee") && word.len() > 2 {
            made.push(format!("{}ing", &word[..word.len() - 1]));
        } else {
            made.push(format!("{word}ing"));
        }
    } else if word.ends_with('e') {
        made.push(format!("{word}{}", &ending[1..]));
    } else if ends_consonant_y(word) {
        made.push(format!("{}i{ending}", &word[..word.len() - 1]));
    } else {
        made.push(format!("{word}{ending}"));
    }
    if may_double(word) {
        made.push(format!("{word}{}{ending}", &word[word.len() - 1..]));
    }
    made
}

/// The level of a word the lists have as it is written, or as a form its
/// part of speech takes (`FORMS`): stricter than `word_level`, which takes
/// any ending off any word — *cannes* is no plural of *can*, nor *yous* of
/// *you*. `None` for any other word.
pub fn listed_level(word: &str) -> Option<Level> {
    FORMS.get(&word.to_lowercase()).copied()
}

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

/// The level of a hyphenated number whose components are listed as numbers.
/// Take the hardest component's level.
fn hyphenated_number_level(word: &str) -> Option<Level> {
    let mut parts = word.split('-');
    let mut level = *NUMBER_WORDS.get(parts.next()?)?;
    let mut hyphenated = false;

    for part in parts {
        level = level.max(*NUMBER_WORDS.get(part)?);
        hyphenated = true;
    }

    hyphenated.then_some(level)
}

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

/// The level of a word by its lowest-listed form, or of a hyphenated
/// number by the highest level of its components.
/// `None` for a word the lists don't have (a name, slang, a rare word).
pub fn word_level(word: &str) -> Option<Level> {
    let word = word.to_lowercase();

    hyphenated_number_level(&word).or_else(|| {
        base_forms(&word)
            .iter()
            .filter_map(|form| WORDS.get(form))
            .min()
            .copied()
    })
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
    fn a_listed_level_takes_only_the_forms_a_words_part_of_speech_has() {
        assert_eq!(listed_level("used"), Some(Level::A1));
        assert_eq!(listed_level("seemed"), listed_level("seem"));
        assert_eq!(listed_level("stopped"), Some(Level::A1));
        assert_eq!(listed_level("bigger"), Some(Level::A1));
        assert_eq!(listed_level("cities"), Some(Level::A1));
        assert_eq!(listed_level("went"), Some(Level::A1));
        assert_eq!(listed_level("cans"), Some(Level::A2), "the noun's plural");
        assert_eq!(listed_level("cannes"), None);
        assert_eq!(listed_level("yous"), None, "a pronoun takes no -s");
        assert_eq!(listed_level("Pasadena"), None);
    }

    #[test]
    fn a_rank_falls_in_its_level() {
        assert_eq!(Level::of_rank(3.0), Level::B1);
        assert_eq!(Level::of_rank(3.99), Level::B1);
        assert_eq!(Level::of_rank(4.0), Level::B2);
        assert_eq!(Level::of_rank(0.2), Level::A1);
        assert_eq!(Level::of_rank(9.0), Level::C2);
    }

    #[test]
    fn hyphenated_number_words_take_the_level_of_their_hardest_part() {
        assert_eq!(word_level("ninety"), Some(Level::A1));
        assert_eq!(word_level("nine"), Some(Level::A1));
        assert_eq!(word_level("ninety-nine"), Some(Level::A1));
        assert_eq!(word_level("twenty-one"), Some(Level::A1));
        assert_eq!(word_level("one-hundred"), Some(Level::A2));

        // Ordinary hyphenated words must retain their existing level.
        assert_eq!(word_level("first-floor"), Some(Level::B1));
    }
}
