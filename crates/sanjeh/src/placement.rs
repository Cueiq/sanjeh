//! Made-up words that look and sound like English but are not: proat,
//! strimble — never xqzt, never a real word. The vocabulary yes/no part of
//! the placement (docs/placement.md, "Vocabulary yes/no") shows them beside
//! real words, so that answering yes to everything scores nothing. In the
//! spirit of Wuggy (Keuleers and Brysbaert, 2010): built from real words'
//! letter chunks, refused when the word is real, listed, or one change from
//! a common word, and matched to real words' length, band by band, so
//! length gives nothing away.

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::levels::{Level, listed_level};
use crate::pronounce::every_word;

/// SplitMix64: a whole generator so the same seed gives the same
/// words and no run ever depends on hidden randomness.
struct Seed(u64);

impl Seed {
    /// The next number, always the same for the same seed.
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Every word the pronouncing table knows, for refusing anything real.
static REAL: LazyLock<HashSet<&'static str>> = LazyLock::new(|| every_word().map(|(word, _, _)| word).collect());

/// The words the lists grade A1 or A2: what "common" means here, for the
/// near-word filter (*tabel* is too near *table*). Built once, like
/// `alike` builds its candidates.
static COMMON: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    every_word()
        .map(|(word, _, _)| word)
        .filter(|word| matches!(listed_level(word), Some(Level::A1) | Some(Level::A2)))
        .collect()
});

/// The words a made-up word is built from, sorted into bands by letter
/// length: `BANDS[5]` is every five-letter word of the table. A candidate
/// joins chunks of words of one band, so it comes out near its band's
/// length, and length gives nothing away. Plain `a` to `z` words of two to
/// twelve letters: what a made-up word should look like.
static BANDS: LazyLock<Vec<Vec<&'static str>>> = LazyLock::new(|| {
    let mut bands = vec![Vec::new(); 13];
    for (word, _, _) in every_word() {
        if (2..=12).contains(&word.len()) && word.bytes().all(|byte| byte.is_ascii_lowercase()) {
            bands[word.len()].push(word);
        }
    }
    bands
});

/// A made-up word must not be a word: not in the pronouncing table, not in
/// the word lists, and not one change from a common word.
fn is_refused(word: &str) -> bool {
    REAL.contains(word) || listed_level(word).is_some() || COMMON.iter().any(|common| one_change_apart(word, common))
}

/// Whether the two words are one *change* apart: a letter replaced, added
/// or dropped, or two neighbouring letters swapped (*tabel*, *table*).
fn one_change_apart(a: &str, b: &str) -> bool {
    let (x, y): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let (x, y) = if x.len() > y.len() {
        (&y[..], &x[..])
    } else {
        (&x[..], &y[..])
    };
    if y.len() - x.len() > 1 {
        return false;
    }
    let mut at = 0;
    while at < x.len() {
        if x[at] == y[at] {
            at += 1;
            continue;
        }
        if y.len() > x.len() {
            // one letter added
            return x[at..] == y[at + 1..];
        }
        if at + 1 < x.len() && x[at] == y[at + 1] && x[at + 1] == y[at] {
            // two neighbouring letters swapped
            return x[at + 2..] == y[at + 2..];
        }

        // one letter replaced
        return x[at + 1..] == y[at + 1..];
    }

    // the longer is the shorter plus one letter
    y.len() > x.len()
}

const VOWEL_LETTERS: &str = "aeiouy";

/// A word's chunks, cut after each run of vowels: *table* → *ta*, *ble*.
/// A made-up word is a chunk of one real word joined to a chunk of another
/// of the same band, so it keeps English spelling patterns. Consonants
/// after the last vowel are a chunk of their own (*blick* → *bli*, *ck*).
fn chunks(word: &str) -> Vec<&str> {
    let vowel = |c: u8| VOWEL_LETTERS.as_bytes().contains(&c);
    let bytes = word.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut at = 0;
    while at < bytes.len() {
        if vowel(bytes[at]) {
            while at < bytes.len() && vowel(bytes[at]) {
                at += 1;
            }
            out.push(&word[start..at]);
            start = at;
        } else {
            at += 1;
        }
    }
    if start < word.len() {
        // trailing consonants: *blick*'s *ck*
        out.push(&word[start..]);
    }
    out
}

/// `n` made-up words, the same for the same seed.
pub fn made_up_words(n: usize, seed: u64) -> Vec<String> {
    let mut seed = Seed(seed);
    let mut seen = HashSet::new();
    // More candidates than are asked for, since the filters refuse some
    // and no word is given twice.
    candidates(n * 20, &mut seed)
        .into_iter()
        .filter(|word| !is_refused(word) && seen.insert(word.clone()))
        .take(n)
        .collect()
}

/// Made-up candidates, in the order the seed gives them: pick a band, pick
/// two words of it, and join the first chunk of one to the rest of the
/// other. Both parts come from words of the band, so the whole comes out
/// near the band's length; every join sits where a real word's chunks meet,
/// so it keeps English spelling and sound patterns; and a join longer than
/// twelve letters is refused, since no band's words are that long, so there
/// the length would give it away.
fn candidates(most: usize, seed: &mut Seed) -> Vec<String> {
    let mut out = Vec::new();
    while out.len() < most {
        let band = &BANDS[2 + (seed.next() % 11) as usize];
        let a = band[(seed.next() % band.len() as u64) as usize];
        let b = band[(seed.next() % band.len() as u64) as usize];
        let b_chunks = chunks(b);
        if a == b || b_chunks.len() < 2 {
            // b is a again or has nothing after its first chunk
            continue;
        }
        let head = chunks(a)[0];
        let tail = b_chunks[1..].join("");
        if head.len() + tail.len() > 12 {
            // longer than any band's words: length would give it away
            continue;
        }
        let mut word = String::from(head);
        word.push_str(&tail);
        out.push(word);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_numbers() {
        let (mut a, mut b) = (Seed(7), Seed(7));
        let first: Vec<u64> = (0..5).map(|_| a.next()).collect();
        let again: Vec<u64> = (0..5).map(|_| b.next()).collect();
        assert_eq!(first, again);
    }

    #[test]
    fn a_different_seed_gives_other_numbers() {
        let (mut a, mut b) = (Seed(7), Seed(8));
        assert_ne!(a.next(), b.next());
    }

    #[test]
    fn one_change_apart_kinds() {
        assert!(one_change_apart("tabel", "table")); // two letters swapped
        assert!(one_change_apart("blick", "black")); // one letter replaced
        assert!(one_change_apart("table", "tables")); // one letter added
        assert!(one_change_apart("blick", "lick")); // one letter dropped
        assert!(!one_change_apart("blick", "trick")); // two changes
        assert!(!one_change_apart("blick", "blick")); // no change at all
    }

    #[test]
    fn chunks_cut_after_vowel_runs() {
        assert_eq!(chunks("table"), vec!["ta", "ble"]);
        assert_eq!(chunks("blick"), vec!["bli", "ck"]);
        assert_eq!(chunks("strimble"), vec!["stri", "mble"]);
    }

    #[test]
    fn the_filters_refuse_words_and_near_words() {
        assert!(is_refused("table")); // a real word
        assert!(is_refused("tabel")); // one change from one
        assert!(!is_refused("strimble")); // far from every common word
    }

    #[test]
    fn bands_group_words_by_length() {
        assert!(BANDS[5].contains(&"table"));
        assert!(BANDS[5].iter().all(|word| word.len() == 5));
        for len in 2..=12 {
            assert!(!BANDS[len].is_empty(), "no words of {len} letters");
        }
        assert_eq!(BANDS.len(), 13);
    }
}
