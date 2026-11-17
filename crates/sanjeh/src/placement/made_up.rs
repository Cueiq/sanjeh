//! Made-up words that look and sound like English but are not: proat,
//! strimble — never xqzt, never a real word. The vocabulary yes/no part of
//! the placement (`docs/placement.md`, "Vocabulary yes/no") shows them
//! beside real words, so that answering yes to everything scores nothing.
//! In the spirit of Wuggy (Keuleers and Brysbaert, 2010): built from real
//! words' letter chunks, refused when the word is real, listed, one change
//! from a listed word, or whose base form is real, and matched to real
//! words' length and syllable count, band by band, so length gives nothing
//! away.
//!
//! The generator is built only from words the level lists grade
//! (`listed_level`), never the CMU table's names, places and abbreviations
//! (`saskatchewan`, `vp`, `mhm`): a learner would turn those down on looks
//! alone. A candidate is the head half of one listed word joined to the
//! tail half of another of the same length, each cut at a chunk boundary,
//! so neither word shows through (`ompilations`, `reliberate`). Length and
//! syllable count are drawn from a real source word, and a candidate is
//! kept only when its length and vowel runs match, so the output's spread
//! of lengths and syllables follows the lists', not a flat pick over 2–12.
//! Candidates are generated lazily, so the count asked for is the count
//! returned no matter how many are refused.

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::levels::{base_forms, listed_level};
use crate::pronounce::every_word;

use super::Seed;

/// Every word the pronouncing table knows, for refusing anything real:
/// the candidate itself and any of its base forms. Built once, like `alike`
/// builds its candidates.
static REAL: LazyLock<HashSet<&'static str>> = LazyLock::new(|| every_word().map(|(word, _, _)| word).collect());

/// The listed words a made-up word is built from, sorted into bands by
/// letter length: `BANDS[5]` is every five-letter word the level lists
/// grade. A candidate joins the head half of one band's word to the tail
/// half of another of the same band, so it keeps English spelling and
/// sound patterns and neither word shows through. Only words the lists
/// have (`listed_level`), never the CMU table's surnames, places and
/// abbreviations (`diguglielmo`, `saskatchewan`, `vp`, `mhm`): the lists'
/// only rude word is *damn*. Plain `a` to `z` words of two to twelve
/// letters: what a made-up word should look like.
static BANDS: LazyLock<Vec<Vec<&'static str>>> = LazyLock::new(|| {
    let mut bands = vec![Vec::new(); 13];
    for (word, _, _) in every_word() {
        if (2..=12).contains(&word.len())
            && word.bytes().all(|byte| byte.is_ascii_lowercase())
            && listed_level(word).is_some()
        {
            bands[word.len()].push(word);
        }
    }
    bands
});

/// The listed words as one flat list, for picking a source word by
/// real-word weight: a random source word's length and vowel runs are the
/// band and syllable count a candidate must match, so the output's spread
/// of lengths and syllables follows the lists', not a flat pick over 2–12.
/// Every word in `BANDS`, in band order.
static LISTED: LazyLock<Vec<&'static str>> = LazyLock::new(|| BANDS.iter().flatten().copied().collect());

/// The vowel letters, for counting vowel runs (a cheap stand-in for
/// syllables) and for refusing a word with no vowel at all.
const VOWEL_LETTERS: &[u8] = b"aeiouy";

/// Offensive substrings to refuse: a short list of the parts rude words
/// share, so a test of fifteen to twenty words is not every sixth one
/// vulgar. The list is small on purpose, so it doesn't read like a slur
/// roll, and checked over thousands of words in `no_word_is_offensive`.
/// Narrowed where a bare short string would catch real words (`ass` would
/// catch *class*, *glass*): `assb`/`assh`/`assw` catch the rude compounds
/// without catching those.
const OFFENSIVE: &[&str] = &[
    "arse", "assb", "assh", "assw", "boob", "cock", "crap", "cunt", "dick", "fagg", "fuck", "jizz", "kike", "nigg",
    "rape", "shit", "slut", "spic", "tits", "twat", "wank",
];

/// A made-up word must not be a word, must not be one change from a listed
/// word, must not have a real base form, must have a vowel, and must not
/// show an offensive substring. Built from the lists' own forms
/// (`listed_level`), so a near-word at any level — *reliberate* (deliberate,
/// B2), *hircing* (hiring, A2) — is caught, not only A1 and A2.
fn is_refused(word: &str) -> bool {
    if REAL.contains(word) || listed_level(word).is_some() {
        return true;
    }
    if word.bytes().all(|byte| !VOWEL_LETTERS.contains(&byte)) {
        return true; // no vowel: *mhmw*, *vpk*, *gpsk*
    }
    if OFFENSIVE.iter().any(|bad| word.contains(bad)) {
        return true;
    }
    if one_change_from_a_listed_word(word) {
        return true;
    }
    // Inflected real words get through the table: *proddings* (prodding),
    // *dippiest* (dippy), *goriness* (goriness). Also refuse a word when
    // any of its base forms is real or listed, as `levels::base_forms`
    // undoes endings.
    if base_forms(word)
        .iter()
        .any(|form| REAL.contains(form.as_str()) || listed_level(form).is_some())
    {
        return true;
    }
    false
}

/// Whether the word is one *change* — a letter replaced, added, dropped, or
/// two neighbouring letters swapped — from some listed word. Generates each
/// one-change variant of the candidate and looks it up with
/// `listed_level`, so the lists' own forms do the work and `COMMON` and
/// `one_change_apart` are not needed. Replaces a scan of every listed word
/// (15,779 of them) with about 300 lookups for a ten-letter candidate.
fn one_change_from_a_listed_word(word: &str) -> bool {
    let letters: Vec<char> = word.chars().collect();
    let len = letters.len();
    // One letter replaced: *tabel* → *table*.
    for at in 0..len {
        let mut variant = letters.clone();
        for c in 'a'..='z' {
            if c == letters[at] {
                continue;
            }
            variant[at] = c;
            if listed_level(&variant.iter().collect::<String>()).is_some() {
                return true;
            }
        }
    }
    // One letter added: *table* → *tables*.
    for at in 0..=len {
        for c in 'a'..='z' {
            let mut variant = letters.clone();
            variant.insert(at, c);
            if listed_level(&variant.iter().collect::<String>()).is_some() {
                return true;
            }
        }
    }
    // One letter dropped: *blick* → *lick*.
    for at in 0..len {
        let mut variant = letters.clone();
        variant.remove(at);
        if listed_level(&variant.iter().collect::<String>()).is_some() {
            return true;
        }
    }
    // Two neighbouring letters swapped: *tabel* → *table*.
    for at in 0..len.saturating_sub(1) {
        if letters[at] == letters[at + 1] {
            continue;
        }
        let mut variant = letters.clone();
        variant.swap(at, at + 1);
        if listed_level(&variant.iter().collect::<String>()).is_some() {
            return true;
        }
    }
    false
}

/// A word's vowel runs: each maximal run of vowels is one syllable, as a
/// cheap stand-in for the pronouncing table's syllable count. *table* → 2
/// (*a*, *e* — the *e* is silent but the run counts), *blick* → 1, *strimble*
/// → 2 (*i* and *e*). Used both to cut a word into chunks and to match a
/// candidate's syllable count to a real word's.
fn vowel_runs(word: &str) -> usize {
    let bytes = word.as_bytes();
    let mut runs = 0;
    let mut in_run = false;
    for &byte in bytes {
        if VOWEL_LETTERS.contains(&byte) {
            if !in_run {
                runs += 1;
                in_run = true;
            }
        } else {
            in_run = false;
        }
    }
    runs
}

/// A word's chunks, cut after each run of vowels: *table* → *ta*, *ble*.
/// A made-up word is the head half of one real word's chunks joined to the
/// tail half of another's, each cut at a chunk boundary, so it keeps
/// English spelling patterns and neither word shows through. Consonants
/// after the last vowel are a chunk of their own (*blick* → *bli*, *ck*).
fn chunks(word: &str) -> Vec<&str> {
    let bytes = word.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut at = 0;
    while at < bytes.len() {
        if VOWEL_LETTERS.contains(&bytes[at]) {
            while at < bytes.len() && VOWEL_LETTERS.contains(&bytes[at]) {
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

/// `n` made-up words, the same for the same seed. Generated lazily, so the
/// count asked for is the count returned no matter how many candidates the
/// filters refuse. `candidate` returns a word that already passes every
/// filter (length, syllables, real, listed, one-change, base forms,
/// vowel-less, offensive); here only the duplicate check remains, so the
/// output's length and syllable spread follows the source words' — the
/// listed words' — not whichever band happens to pass.
pub fn made_up_words(n: usize, seed: u64) -> Vec<String> {
    if n == 0 {
        return Vec::new();
    }
    let mut seed = Seed(seed);
    let mut seen: HashSet<String> = HashSet::new();
    std::iter::from_fn(|| {
        // `candidate` may return a word already given (the seed's space is
        // large, so this is rare); a bound guards against a pathological
        // seed. Far more tries than any n needs; a test would catch a
        // shortfall.
        for _ in 0..10_000 {
            let Some(word) = candidate(&mut seed) else {
                continue;
            };
            if seen.insert(word.clone()) {
                return Some(word);
            }
        }
        None
    })
    .take(n)
    .collect()
}

/// One made-up candidate, fully filtered, in the order the seed gives it.
/// Pick a source word by real-word weight (a random listed word), so its
/// length is the band and its vowel runs are the syllable count to match;
/// pick two words of that band; find a split of `a`'s chunks (a head) and
/// a split of `b`'s chunks (a tail) whose lengths add up to the band's
/// length, each cut at a chunk boundary; keep the candidate only when its
/// vowel runs are within one of the source's and it passes `is_refused`.
/// Returning a fully filtered word (not just length- and syllable-matched)
/// keeps the output's spread on the source words' spread: when `is_refused`
/// was in the caller, a refused candidate threw the band away and the
/// caller drew a new source word, so short bands (four letters, where the
/// one-change refusal passes only ~2% of candidates) all but vanished.
///
/// Short bands whose every candidate is one change from a listed word
/// (two- and three-letter words, where the dictionary is dense) yield
/// nothing: the retries run out and `None` is returned, the caller draws
/// another source word, and those bands are ~2% of the listed words — a
/// shortfall the spread test's tolerance covers, and honest about what the
/// one-change refusal does.
fn candidate(seed: &mut Seed) -> Option<String> {
    let total = LISTED.len() as u64;
    if total == 0 {
        return None;
    }
    let source = LISTED[(seed.next() % total) as usize];
    let band_len = source.len();
    let source_runs = vowel_runs(source);
    let band = &BANDS[band_len];
    if band.len() < 2 {
        return None;
    }
    for _ in 0..100 {
        let a = band[(seed.next() % band.len() as u64) as usize];
        let b = band[(seed.next() % band.len() as u64) as usize];
        if a == b {
            continue;
        }
        let a_chunks = chunks(a);
        let b_chunks = chunks(b);
        if a_chunks.len() < 2 || b_chunks.len() < 2 {
            continue;
        }
        // Each (split_a, split_b) pair is a head from `a` (its first
        // `split_a` chunks) and a tail from `b` (its last chunks from
        // `split_b` onward), each cut at a chunk boundary. Find every pair
        // whose head and tail lengths add up to the band's length, so the
        // candidate is the band's length by construction; then pick one at
        // random, so the join varies and neither word shows through.
        let mut options: Vec<(usize, usize)> = Vec::new();
        for split_a in 1..a_chunks.len() {
            let head_len: usize = a_chunks[..split_a].iter().map(|c| c.len()).sum();
            for split_b in 1..b_chunks.len() {
                let tail_len: usize = b_chunks[split_b..].iter().map(|c| c.len()).sum();
                if head_len + tail_len == band_len {
                    options.push((split_a, split_b));
                }
            }
        }
        if options.is_empty() {
            continue;
        }
        let (split_a, split_b) = options[(seed.next() % options.len() as u64) as usize];
        let head: String = a_chunks[..split_a].join("");
        let tail: String = b_chunks[split_b..].join("");
        let word: String = format!("{head}{tail}");
        // The length is the band's by construction; the syllable count
        // (vowel runs) must be within one of the source's, so the output's
        // spread of syllables follows the lists'. Within one, not exact,
        // so enough candidates pass.
        let runs = vowel_runs(&word) as isize;
        if (runs - source_runs as isize).abs() > 1 {
            continue;
        }
        // The full refusal check: real, listed, one change from a listed
        // word, a real base form, vowel-less, or offensive. Checked here,
        // in the retry loop over the same band, so a refused candidate
        // doesn't throw the band away.
        if is_refused(&word) {
            continue;
        }
        return Some(word);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_words() {
        assert_eq!(made_up_words(50, 1), made_up_words(50, 1));
    }

    #[test]
    fn a_different_seed_gives_other_words() {
        assert_ne!(made_up_words(50, 1), made_up_words(50, 2));
    }

    #[test]
    fn vowel_runs_count_maximal_vowel_sequences() {
        assert_eq!(vowel_runs("table"), 2);
        assert_eq!(vowel_runs("blick"), 1);
        assert_eq!(vowel_runs("strimble"), 2); // i and e, two runs
        assert_eq!(vowel_runs("beautiful"), 3); // eau, u, i — three runs
        assert_eq!(vowel_runs("mhmw"), 0);
    }

    #[test]
    fn chunks_cut_after_vowel_runs() {
        assert_eq!(chunks("table"), vec!["ta", "ble"]);
        assert_eq!(chunks("blick"), vec!["bli", "ck"]);
        assert_eq!(chunks("strimble"), vec!["stri", "mble"]);
    }

    #[test]
    fn the_filters_refuse_words_near_words_and_base_forms() {
        assert!(is_refused("table")); // a real word
        assert!(is_refused("tabel")); // one change from a listed word
        assert!(is_refused("proddings")); // base form prodding is real
        assert!(is_refused("dippiest")); // base form dippy is real
        assert!(is_refused("mhmw")); // no vowel
        assert!(is_refused("pellshitting")); // offensive substring
        assert!(!is_refused("strimble")); // far from every listed word
    }

    #[test]
    fn bands_hold_only_listed_words() {
        for band in BANDS.iter().skip(2).take(11) {
            for word in band {
                assert!(listed_level(word).is_some(), "{word} is not in the lists");
            }
        }
        for len in 2..=12 {
            assert!(!BANDS[len].is_empty(), "no listed words of {len} letters");
        }
    }

    #[test]
    fn as_many_as_asked_even_when_n_is_large() {
        assert!(made_up_words(0, 1).is_empty());
        for n in [1, 5, 50, 200] {
            assert_eq!(made_up_words(n, 1).len(), n, "asked for {n}");
        }
        for seed in 1..=3 {
            assert_eq!(made_up_words(200, seed).len(), 200, "seed {seed}");
        }
    }

    /// The output's length spread follows the listed words', band by band,
    /// so length gives nothing away. Each length's share of the output is
    /// within five percentage points of its share of `LISTED`; with 1,800
    /// words over six seeds, the variance is small. The old test only
    /// checked 2–12, which the generator already guaranteed, so it could
    /// not fail.
    #[test]
    fn lengths_follow_the_lists_spread() {
        let mut listed = [0usize; 13];
        for word in LISTED.iter() {
            listed[word.len()] += 1;
        }
        let listed_total = LISTED.len() as f64;
        let mut made = [0usize; 13];
        let words: Vec<String> = (1..=6).flat_map(|seed| made_up_words(300, seed)).collect();
        for word in &words {
            made[word.len()] += 1;
        }
        let made_total = words.len() as f64;
        for len in 2..=12 {
            let listed_share = listed[len] as f64 / listed_total;
            let made_share = made[len] as f64 / made_total;
            let listed_pct = listed_share * 100.0;
            let made_pct = made_share * 100.0;
            assert!(
                (made_share - listed_share).abs() <= 0.05,
                "length {len}: made-up {made_pct:.0}% vs listed {listed_pct:.0}%"
            );
        }
    }

    /// The output's syllable spread (vowel runs, a cheap stand-in) follows
    /// the listed words', so syllable count gives nothing away either.
    /// Each run count's share of the output is within eight percentage
    /// points of its share of `LISTED`; the wider tolerance covers the
    /// ±1-run acceptance in `candidate`.
    #[test]
    fn syllables_follow_the_lists_spread() {
        let mut listed = [0usize; 8];
        for word in LISTED.iter() {
            let runs = vowel_runs(word).min(7);
            listed[runs] += 1;
        }
        let listed_total = LISTED.len() as f64;
        let mut made = [0usize; 8];
        let words: Vec<String> = (1..=6).flat_map(|seed| made_up_words(300, seed)).collect();
        for word in &words {
            let runs = vowel_runs(word).min(7);
            made[runs] += 1;
        }
        let made_total = words.len() as f64;
        for runs in 1..=6 {
            let listed_share = listed[runs] as f64 / listed_total;
            let made_share = made[runs] as f64 / made_total;
            let listed_pct = listed_share * 100.0;
            let made_pct = made_share * 100.0;
            assert!(
                (made_share - listed_share).abs() <= 0.08,
                "runs {runs}: made-up {made_pct:.0}% vs listed {listed_pct:.0}%"
            );
        }
    }
}
