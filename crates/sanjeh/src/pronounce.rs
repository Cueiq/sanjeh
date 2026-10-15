//! How an English word or phrase sounds, in IPA, from two open pronouncing
//! dictionaries bundled with Sanjeh (decided 2026-10-05): Britfone for
//! British English, the CMU Pronouncing Dictionary for American, rewritten
//! the way learners' dictionaries write them by `scripts/pronouncing/build.py`
//! (their notices are in `data/pronouncing/NOTICE.md`). The accent is the
//! caller's to choose.

use std::io::Read;
use std::sync::LazyLock;

use flate2::read::GzDecoder;
use serde::Deserialize;

use crate::term_key;

const TABLE: &[u8] = include_bytes!("../../../data/pronouncing/pronouncing.tsv.gz");

/// The accent a sound is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    Us,
    Gb,
}

/// The table as it is bundled — one line a word, `word<TAB>us<TAB>gb<TAB>
/// differs`, sorted by the word's bytes — and where each line starts. Words
/// are found by halving the lines, never copied out one by one: reading
/// 126,000 words into a map took a twentieth of a second and tens of
/// megabytes (review, 2026-10-06).
struct Table {
    text: String,
    lines: Vec<u32>,
}

impl Table {
    fn line(&self, at: usize) -> &str {
        let start = self.lines[at] as usize;
        let end = self
            .lines
            .get(at + 1)
            .map_or(self.text.len(), |next| *next as usize);
        self.text[start..end].trim_end_matches('\n')
    }

    fn find(&self, word: &str) -> Option<Entry<'_>> {
        let at = self
            .lines
            .binary_search_by(|start| {
                let rest = &self.text[*start as usize..];
                rest[..rest.find('\t').unwrap_or(rest.len())].cmp(word)
            })
            .ok()?;
        let mut fields = self.line(at).split('\t').skip(1);
        Some(Entry {
            us: fields.next().unwrap_or(""),
            gb: fields.next().unwrap_or(""),
            differs: fields.next() == Some("1"),
        })
    }
}

/// A word's line: its sounds in each accent, the dictionary's first first,
/// `|` between them; and whether its readings say different things.
struct Entry<'a> {
    us: &'a str,
    gb: &'a str,
    differs: bool,
}

static WORDS: LazyLock<Table> = LazyLock::new(|| {
    let mut text = String::new();
    GzDecoder::new(TABLE)
        .read_to_string(&mut text)
        .expect("the pronouncing table is gzip of UTF-8");
    let lines = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(at, _)| at + 1))
        .filter(|start| *start < text.len())
        .map(|start| u32::try_from(start).expect("the pronouncing table is under 4 GB"))
        .collect();
    Table { text, lines }
});

/// Reads the table now, rather than on the first word asked about: call it
/// as a program starts, off its main thread.
pub fn warm_pronouncing() {
    LazyLock::force(&WORDS);
}

/// What the table says of a word or phrase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pronounced {
    /// Its sound, every word found.
    Known(String),
    /// A word of it has readings that say different things — *read* now
    /// and before, *perfect* the adjective and the verb — which the table
    /// can't choose between for a line (build.py's `readings_differ`).
    Ambiguous,
    /// A word of it isn't in the table: a name, slang, a number.
    Unknown,
}

/// The sound of a word or phrase in an accent, as IPA without slashes, its
/// words' sounds one space apart. It is read by its key (`term_key`):
/// composed, case, every apostrophe and hyphen alike. The
/// punctuation around a word doesn't matter; a full stop inside one does
/// (*U.S.*). A hyphenated word not in the table is its parts, each with its
/// own stress, as a phrase's words are.
pub fn pronounce(text: &str, accent: Accent) -> Pronounced {
    let key = term_key(text);
    let words: Vec<&str> = key
        .split(' ')
        .map(|word| {
            word.trim_start_matches(|c: char| !c.is_alphanumeric() && c != '\'')
                .trim_end_matches(|c: char| !c.is_alphanumeric() && !matches!(c, '\'' | '.'))
        })
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .collect();
    if words.is_empty() {
        return Pronounced::Unknown;
    }
    let mut sounds = Vec::with_capacity(words.len());
    for word in words {
        match word_sound(word, accent) {
            Found::Sound(sound) => sounds.push(sound),
            Found::Ambiguous => return Pronounced::Ambiguous,
            Found::Missing => return Pronounced::Unknown,
        }
    }
    Pronounced::Known(sounds.join(" "))
}

/// Every reading the table gives one word in an accent, the dictionary's
/// first first — even a word whose readings say different things (*with*:
/// θ or ð), for what holds whichever is said: its syllables, its hard
/// sounds. None for a word it doesn't have.
pub fn readings(word: &str, accent: Accent) -> Vec<String> {
    let key = term_key(word);
    let Some(entry) = WORDS.find(key.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'')) else {
        return vec![];
    };
    let side = match accent {
        Accent::Us => entry.us,
        Accent::Gb => entry.gb,
    };
    side.split('|').filter(|sound| !sound.is_empty()).map(str::to_owned).collect()
}

enum Found {
    Sound(String),
    Ambiguous,
    Missing,
}

/// One word's sound: as written; without the full stops that end it
/// (*end.*); inside quotes ('*hello*'), or without a plural's possessive
/// apostrophe (*dogs'*) — never without one that begins a word, which
/// stands for letters left out ('*cause* is not *cause*); else a hyphenated
/// word's parts.
fn word_sound(word: &str, accent: Accent) -> Found {
    if let Some(entry) = WORDS.find(word) {
        let side = match accent {
            Accent::Us => entry.us,
            Accent::Gb => entry.gb,
        };
        if let Some(first) = side.split('|').find(|sound| !sound.is_empty()) {
            return if entry.differs {
                Found::Ambiguous
            } else {
                Found::Sound(first.to_owned())
            };
        }
    }
    let unstopped = word.trim_end_matches('.');
    if unstopped != word && !unstopped.is_empty() {
        return word_sound(unstopped, accent);
    }
    let quoted = word.len() > 2 && word.starts_with('\'') && word.ends_with('\'');
    if quoted {
        return word_sound(&word[1..word.len() - 1], accent);
    }
    if word.ends_with('\'') && !word.starts_with('\'') {
        return word_sound(word.trim_end_matches('\''), accent);
    }
    if word.contains('-') {
        let mut sounds = Vec::new();
        for part in word.split('-').filter(|part| !part.is_empty()) {
            match word_sound(part, accent) {
                Found::Sound(sound) => sounds.push(sound),
                other => return other,
            }
        }
        return if sounds.is_empty() {
            Found::Missing
        } else {
            Found::Sound(sounds.join(" "))
        };
    }
    Found::Missing
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known(text: &str, accent: Accent) -> Option<String> {
        match pronounce(text, accent) {
            Pronounced::Known(sound) => Some(sound),
            _ => None,
        }
    }

    #[test]
    fn a_word_sounds_in_either_accent_with_stress_before_its_syllable() {
        assert_eq!(known("fancy", Accent::Us).as_deref(), Some("ˈfænsi"));
        assert_eq!(known("control", Accent::Us).as_deref(), Some("kənˈtroʊl"));
        assert_eq!(known("control", Accent::Gb).as_deref(), Some("kənˈtrəʊl"));
        assert_eq!(known("car", Accent::Us).as_deref(), Some("kɑːr"));
        assert_eq!(known("car", Accent::Gb).as_deref(), Some("kɑː"));
        assert_eq!(known("music", Accent::Gb).as_deref(), Some("ˈmjuːzɪk"));
    }

    #[test]
    fn a_word_has_one_main_stress_its_onsets_hold_and_unstressed_long_vowels_are_short() {
        // CMU marks both of okay's syllables main.
        assert_eq!(known("OK", Accent::Us).as_deref(), Some("ˌoʊˈkeɪ"));
        // No syllable begins /sf/ inside a word; a word's first may.
        assert_eq!(
            known("transfusion", Accent::Us).as_deref(),
            Some("trænsˈfjuːʒən")
        );
        assert_eq!(
            known("atmosphere", Accent::Gb).as_deref(),
            Some("ˈætməsˌfɪə")
        );
        assert_eq!(known("spherical", Accent::Us).as_deref(), Some("ˈsferɪkəl"));
        assert_eq!(known("influence", Accent::Us).as_deref(), Some("ˈɪnfluəns"));
    }

    #[test]
    fn a_phrase_is_its_words_and_punctuation_and_case_dont_matter() {
        assert_eq!(known("Near mint", Accent::Us).as_deref(), Some("nɪr mɪnt"));
        assert_eq!(known("No way!", Accent::Gb).as_deref(), Some("nəʊ weɪ"));
        assert_eq!(known("Don’t", Accent::Us).as_deref(), Some("doʊnt"));
        assert_eq!(known("'em", Accent::Gb).as_deref(), Some("əm"));
        assert_eq!(
            pronounce("'Hello'", Accent::Us),
            pronounce("hello", Accent::Us)
        );
        // Every apostrophe read as one, and composed.
        assert_eq!(known("don‛t", Accent::Us), known("don't", Accent::Us));
        assert_eq!(known("don`t", Accent::Us), known("don't", Accent::Us));
        assert_eq!(known("don´t", Accent::Us), known("don't", Accent::Us));
    }

    #[test]
    fn full_stops_inside_a_word_are_its_own() {
        assert_eq!(known("the U.S.", Accent::Us).as_deref(), Some("ðə ˌjuːˈes"));
        assert_eq!(known("a.m.", Accent::Us).as_deref(), Some("ˌeɪˈem"));
        assert_eq!(known("e.g.", Accent::Us).as_deref(), Some("ˌiːˈgiː"));
        assert_eq!(known("The end.", Accent::Gb), known("the end", Accent::Gb));
    }

    #[test]
    fn an_apostrophe_that_begins_a_word_stands_for_letters_left_out() {
        // Britfone has no 'cause: it is not cause.
        assert_eq!(pronounce("'cause", Accent::Gb), Pronounced::Unknown);
        assert_eq!(known("'cause", Accent::Us).as_deref(), Some("kəz"));
        // A plural's possessive sounds as the plural.
        assert_eq!(
            known("the girls'", Accent::Gb),
            known("the girls", Accent::Gb)
        );
    }

    #[test]
    fn british_contractions_and_possessives_are_made_from_the_word() {
        assert_eq!(known("I'm", Accent::Gb).as_deref(), Some("aɪm"));
        assert_eq!(known("it's", Accent::Gb).as_deref(), Some("ɪts"));
        assert_eq!(known("we're", Accent::Gb).as_deref(), Some("wɪə"));
        assert_eq!(
            known("the horse's", Accent::Gb).as_deref(),
            Some("ðə ˈhɔːsɪz")
        );
    }

    #[test]
    fn a_hyphenated_word_not_in_the_table_is_its_parts_each_with_its_stress() {
        assert_eq!(known("bike-shed", Accent::Gb).as_deref(), Some("baɪk ʃed"));
        assert_eq!(
            known("twenty-two", Accent::Gb).as_deref(),
            Some("ˈtwenti tuː")
        );
        // The hyphens a subtitle may write instead.
        assert_eq!(
            known("twenty\u{2011}two", Accent::Gb),
            known("twenty-two", Accent::Gb)
        );
        assert_eq!(
            known("twenty\u{2010}two", Accent::Gb),
            known("twenty-two", Accent::Gb)
        );
    }

    #[test]
    fn a_word_whose_readings_differ_is_ambiguous_and_an_unknown_word_unknowns_the_phrase() {
        assert_eq!(pronounce("read", Accent::Gb), Pronounced::Ambiguous);
        assert_eq!(pronounce("I read it", Accent::Us), Pronounced::Ambiguous);
        // Its stress moves with its meaning, in either accent.
        for word in ["perfect", "suspect", "increase", "upset", "record", "live"] {
            assert_eq!(pronounce(word, Accent::Us), Pronounced::Ambiguous, "{word}");
            assert_eq!(pronounce(word, Accent::Gb), Pronounced::Ambiguous, "{word}");
        }
        // A weak form is no second reading.
        assert_eq!(known("to", Accent::Us).as_deref(), Some("tuː"));
        assert_eq!(pronounce("the zxqv", Accent::Us), Pronounced::Unknown);
        assert_eq!(pronounce("Hector", Accent::Gb), Pronounced::Unknown);
        assert_eq!(pronounce("…", Accent::Us), Pronounced::Unknown);
        assert_eq!(pronounce(" - ", Accent::Us), Pronounced::Unknown);
        // A dash or dots against a word are not of it.
        assert_eq!(known("—well-", Accent::Us), known("well", Accent::Us));
        assert_eq!(known("…well", Accent::Us), known("well", Accent::Us));
    }
}
