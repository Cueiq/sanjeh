//! Words that sound like a word (decided 2026-10-19): which words a learner
//! knows could be heard instead of one that was said — *too* and *two* for
//! *to*, *used* for *use*, *wood* and *world* for *would*. A hearing
//! exercise needs them as its wrong answers: a random wrong word tests
//! nothing, while one the ear could truly take for the right one does.
//!
//! Two words are compared by their sounds, not their letters (`pronounce`):
//! each reading is cut into its sounds, and the distance between two words
//! is the cheapest way to turn one's sounds into the other's, a sound
//! changed costing by how far apart the two sounds are — voicing, place and
//! manner for consonants, height, backness, rounding and length for vowels
//! — and a sound dropped or added costing least at the end of a word, where
//! fast speech drops them. Stress is left out. American and British readings
//! both count, the nearer of them, unless one accent is asked for. Only
//! words the CEFR lists know (`levels`) are candidates: a word a learner has
//! never met is no test of their ear.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::levels::{Level, listed_level};
use crate::pronounce::every_word;
use crate::{Accent, readings, term_key};

/// How a word sounds like another, in the order they come on a tie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlikeKind {
    /// The same sound, spelled otherwise: *to*, *too*, *two*.
    Same,
    /// The same word with an ending left off or added — *-s*, *-ed*,
    /// *-ing*, *'ve*, *n't*, *'ll*, *'d*: *use*, *used*; *would*, *would've*.
    Ending,
    /// A sound or two apart: *would*, *world*; *ship*, *sheep*.
    Near,
}

impl AlikeKind {
    pub const ALL: [AlikeKind; 3] = [AlikeKind::Same, AlikeKind::Ending, AlikeKind::Near];

    pub fn parse(text: &str) -> Option<AlikeKind> {
        AlikeKind::ALL
            .into_iter()
            .find(|kind| format!("{kind:?}").eq_ignore_ascii_case(text.trim()))
    }
}

/// A word that sounds like the one asked about.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Alike {
    pub word: String,
    pub kind: AlikeKind,
    /// How far its sound is from the word's: 0 for the same sound, about a
    /// quarter for one near sound changed (*t* for *d*), about one for a
    /// sound changed into one unlike it.
    pub distance: f32,
    /// Its reading that came nearest, in IPA.
    pub sound: String,
    /// What differs, in its sounds: `/ɝː/ for /ʊ/`, `+ /d/`, `− /t/`.
    pub why: String,
}

/// What `alikes` is asked for.
#[derive(Debug, Clone)]
pub struct AlikeOptions {
    /// One accent's readings only; none, both, the nearer counting.
    pub accent: Option<Accent>,
    /// The learner's level: candidates up to the level above it. None, any
    /// word the lists know.
    pub level: Option<Level>,
    /// The kinds wanted; empty, all of them.
    pub kinds: Vec<AlikeKind>,
    /// How many, the nearest first.
    pub most: usize,
}

impl Default for AlikeOptions {
    fn default() -> Self {
        AlikeOptions {
            accent: None,
            level: None,
            kinds: vec![],
            most: 10,
        }
    }
}

/// The words a learner knows that sound like `word`, the most alike first:
/// by distance, then kind (`AlikeKind`), then the lower level, then the
/// word. A word not in the pronouncing table has none.
pub fn alikes(word: &str, options: &AlikeOptions) -> Vec<Alike> {
    let key = term_key(word);
    let key = key.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'');
    let accents: &[Accent] = match options.accent {
        Some(Accent::Us) => &[Accent::Us],
        Some(Accent::Gb) => &[Accent::Gb],
        None => &[Accent::Us, Accent::Gb],
    };
    // Each reading of the word, and whether it is the dictionary's first.
    let mut own: Vec<(Vec<u8>, bool)> = vec![];
    for accent in accents {
        for (at, reading) in readings(key, *accent).into_iter().enumerate() {
            if let Some(sounds) = sounds_of(&reading)
                && !own.iter().any(|(other, _)| *other == sounds)
            {
                own.push((sounds, at == 0));
            }
        }
    }
    if own.is_empty() {
        return vec![];
    }
    let most_level = options.level.map(|level| Level::ALL[(level as usize + 1).min(5)]);
    let wanted = |kind: AlikeKind| options.kinds.is_empty() || options.kinds.contains(&kind);
    let mut found: Vec<(Alike, Level)> = vec![];
    for candidate in CANDIDATES.iter() {
        if candidate.word == key || most_level.is_some_and(|most| candidate.level > most) {
            continue;
        }
        // The nearest pair of readings: a reading that isn't the
        // dictionary's first — *to* said *tə* — counts a little farther.
        let mut best: Option<(f32, f32, &Reading, &[u8])> = None;
        for reading in candidate
            .readings
            .iter()
            .filter(|reading| accents.contains(&reading.accent))
        {
            for (sounds, first) in &own {
                if sounds.len().abs_diff(reading.sounds.len()) > MOST_LENGTH_APART {
                    continue;
                }
                let apart = distance(sounds, &reading.sounds);
                let ranked =
                    apart + if *first { 0.0 } else { OTHER_READING } + if reading.first { 0.0 } else { OTHER_READING };
                if best.is_none_or(|(nearest, ..)| ranked < nearest) {
                    best = Some((ranked, apart, reading, sounds));
                }
            }
        }
        let Some((ranked, apart, reading, sounds)) = best else {
            continue;
        };
        let distance = if apart == 0.0 { 0.0 } else { ranked };
        let kind = if apart == 0.0 {
            AlikeKind::Same
        } else if ending_apart(key, candidate.word) && distance <= MOST_FOR_ENDING {
            AlikeKind::Ending
        } else if distance <= MOST_FOR_NEAR
            && distance <= NEAR_PER_SOUND * sounds.len().max(reading.sounds.len()) as f32
        {
            AlikeKind::Near
        } else {
            continue;
        };
        if !wanted(kind) {
            continue;
        }
        let why = if kind == AlikeKind::Same {
            "the same sound".to_string()
        } else {
            differences(sounds, &reading.sounds)
        };
        found.push((
            Alike {
                word: candidate.word.to_string(),
                kind,
                distance,
                sound: reading.text.to_string(),
                why,
            },
            candidate.level,
        ));
    }
    found.sort_by(|(a, a_level), (b, b_level)| {
        a.distance
            .total_cmp(&b.distance)
            .then(a.kind.cmp(&b.kind))
            .then(a_level.cmp(b_level))
            .then(a.word.cmp(&b.word))
    });
    found.into_iter().take(options.most).map(|(alike, _)| alike).collect()
}

/// Reads the candidates now, rather than on the first word asked about:
/// call it as a program starts, off its main thread.
pub fn warm_alikes() {
    LazyLock::force(&CANDIDATES);
}

/// The farthest a near word may be.
const MOST_FOR_NEAR: f32 = 1.5;
/// … and for each of the longer word's sounds: two short words three sounds
/// apart aren't alike.
const NEAR_PER_SOUND: f32 = 0.5;
/// The farthest the same word with another ending may be: *would*,
/// *wouldn't* is three sounds added.
const MOST_FOR_ENDING: f32 = 2.0;
/// What a reading other than the dictionary's first adds to a distance.
const OTHER_READING: f32 = 0.15;
/// Words more sounds apart than this in length are never compared.
const MOST_LENGTH_APART: usize = 3;

/// A word the lists know, with its readings cut into sounds.
struct Candidate {
    word: &'static str,
    level: Level,
    readings: Vec<Reading>,
}

struct Reading {
    accent: Accent,
    text: &'static str,
    sounds: Vec<u8>,
    /// The dictionary's first reading for its accent.
    first: bool,
}

static CANDIDATES: LazyLock<Vec<Candidate>> = LazyLock::new(|| {
    every_word()
        .filter(|(word, ..)| word.chars().all(|c| c.is_ascii_lowercase() || c == '\'') && !word.starts_with('\''))
        .filter_map(|(word, us, gb)| {
            let level = known_level(word)?;
            let readings: Vec<Reading> = [(Accent::Us, us), (Accent::Gb, gb)]
                .into_iter()
                .flat_map(|(accent, side)| {
                    side.split('|')
                        .filter(|text| !text.is_empty())
                        .enumerate()
                        .map(move |(at, text)| (accent, at, text))
                })
                .filter_map(|(accent, at, text)| {
                    Some(Reading {
                        accent,
                        text,
                        sounds: sounds_of(text)?,
                        first: at == 0,
                    })
                })
                .collect();
            (!readings.is_empty()).then_some(Candidate { word, level, readings })
        })
        .collect()
});

/// The contractions a word may take, after its apostrophe.
const CONTRACTIONS: [&str; 6] = ["s", "ve", "ll", "d", "re", "m"];
/// The words *'s* is *is* or *has* after, never a noun's possessive: *it's*,
/// not *wood's*.
const TAKES_S: [&str; 13] = [
    "it", "he", "she", "that", "there", "here", "what", "who", "where", "when", "why", "how", "let",
];
/// Negatives not written as the word and *n't*.
const NEGATIVES: [(&str, &str); 3] = [("will", "won't"), ("can", "can't"), ("shall", "shan't")];

/// A word's level by the lists (`listed_level`), and a contraction's by its
/// word: *would've* is *would*'s.
fn known_level(word: &str) -> Option<Level> {
    if let Some(level) = listed_level(word) {
        return Some(level);
    }
    if let Some((plain, _)) = NEGATIVES.iter().find(|(_, negative)| *negative == word) {
        return listed_level(plain);
    }
    if let Some(stem) = word.strip_suffix("n't") {
        return listed_level(stem);
    }
    let (stem, tail) = word.split_once('\'')?;
    let takes = if tail == "s" {
        TAKES_S.contains(&stem)
    } else {
        CONTRACTIONS.contains(&tail)
    };
    takes.then(|| listed_level(stem)).flatten()
}

/// The endings a word is said with, as they are written, and what goes back
/// in their place: *tries* is *try*, *liked* is *like*.
const ENDINGS: [(&str, &str); 9] = [
    ("ies", "y"),
    ("ied", "y"),
    ("es", ""),
    ("s", ""),
    ("ed", ""),
    ("ed", "e"),
    ("d", ""),
    ("ing", ""),
    ("ing", "e"),
];

/// Whether two words are one word with another ending, either way round.
fn ending_apart(a: &str, b: &str) -> bool {
    a != b && (ends_from(a, b) || ends_from(b, a))
}

/// Whether `long` is `short` with an ending added: *used*, *use*; *would've*,
/// *would*; *stopped*, *stop*.
fn ends_from(long: &str, short: &str) -> bool {
    if NEGATIVES.contains(&(short, long)) || long.strip_suffix("n't") == Some(short) {
        return true;
    }
    if let Some((stem, tail)) = long.split_once('\'')
        && stem == short
        && CONTRACTIONS.contains(&tail)
    {
        return true;
    }
    ENDINGS.iter().any(|(ending, put)| {
        let Some(stem) = long.strip_suffix(ending).filter(|stem| stem.len() >= 2) else {
            return false;
        };
        let doubled =
            put.is_empty() && stem.len() > 2 && stem.as_bytes()[stem.len() - 1] == stem.as_bytes()[stem.len() - 2];
        format!("{stem}{put}") == short || (doubled && &stem[..stem.len() - 1] == short)
    })
}

// ---- Sounds ----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Manner {
    Stop,
    Affricate,
    Fricative,
    Nasal,
    Lateral,
    Approximant,
}

#[derive(Debug, Clone, Copy)]
struct Vowel {
    /// 0 close (*iː*) to 6 open (*ɑː*).
    height: f32,
    /// 0 front (*iː*) to 2 back (*uː*).
    back: f32,
    round: bool,
    long: bool,
    rhotic: bool,
}

#[derive(Debug, Clone, Copy)]
enum Sound {
    /// `place`: 0 lips, 1 lip and teeth, 2 teeth, 3 ridge, 4 behind it,
    /// 5 palate, 6 velum, 7 glottis.
    Consonant {
        place: u8,
        manner: Manner,
        voiced: bool,
    },
    Vowel(Vowel),
    Diphthong(Vowel, Vowel),
}

const fn c(place: u8, manner: Manner, voiced: bool) -> Sound {
    Sound::Consonant { place, manner, voiced }
}

const fn v(height: f32, back: f32, round: bool, long: bool, rhotic: bool) -> Vowel {
    Vowel {
        height,
        back,
        round,
        long,
        rhotic,
    }
}

const I: Vowel = v(0.0, 0.0, false, false, false);
const SMALL_I: Vowel = v(1.0, 0.3, false, false, false);
const E: Vowel = v(3.0, 0.0, false, false, false);
const A: Vowel = v(6.0, 0.5, false, false, false);
const O: Vowel = v(2.0, 2.0, true, false, false);
const OPEN_O: Vowel = v(4.0, 2.0, true, false, false);
const SMALL_U: Vowel = v(1.0, 1.7, true, false, false);
const SCHWA: Vowel = v(3.0, 1.0, false, false, false);

/// Every sound the table writes, with the IPA it is written in: two letters
/// before one, so *tʃ* is one sound and *iː* one long vowel.
const SOUNDS: &[(&str, Sound)] = &[
    ("tʃ", c(4, Manner::Affricate, false)),
    ("dʒ", c(4, Manner::Affricate, true)),
    ("eɪ", Sound::Diphthong(E, SMALL_I)),
    ("aɪ", Sound::Diphthong(A, SMALL_I)),
    ("ɔɪ", Sound::Diphthong(OPEN_O, SMALL_I)),
    ("aʊ", Sound::Diphthong(A, SMALL_U)),
    ("oʊ", Sound::Diphthong(O, SMALL_U)),
    ("əʊ", Sound::Diphthong(SCHWA, SMALL_U)),
    ("ɪə", Sound::Diphthong(SMALL_I, SCHWA)),
    ("eə", Sound::Diphthong(E, SCHWA)),
    ("ʊə", Sound::Diphthong(SMALL_U, SCHWA)),
    ("iː", Sound::Vowel(v(0.0, 0.0, false, true, false))),
    ("uː", Sound::Vowel(v(0.0, 2.0, true, true, false))),
    ("ɑː", Sound::Vowel(v(6.0, 2.0, false, true, false))),
    ("ɔː", Sound::Vowel(v(4.0, 2.0, true, true, false))),
    ("ɜː", Sound::Vowel(v(3.0, 1.0, false, true, false))),
    ("ɝː", Sound::Vowel(v(3.0, 1.0, false, true, true))),
    ("p", c(0, Manner::Stop, false)),
    ("b", c(0, Manner::Stop, true)),
    ("t", c(3, Manner::Stop, false)),
    ("d", c(3, Manner::Stop, true)),
    ("k", c(6, Manner::Stop, false)),
    ("g", c(6, Manner::Stop, true)),
    ("f", c(1, Manner::Fricative, false)),
    ("v", c(1, Manner::Fricative, true)),
    ("θ", c(2, Manner::Fricative, false)),
    ("ð", c(2, Manner::Fricative, true)),
    ("s", c(3, Manner::Fricative, false)),
    ("z", c(3, Manner::Fricative, true)),
    ("ʃ", c(4, Manner::Fricative, false)),
    ("ʒ", c(4, Manner::Fricative, true)),
    ("h", c(7, Manner::Fricative, false)),
    ("m", c(0, Manner::Nasal, true)),
    ("n", c(3, Manner::Nasal, true)),
    ("ŋ", c(6, Manner::Nasal, true)),
    ("l", c(3, Manner::Lateral, true)),
    ("r", c(4, Manner::Approximant, true)),
    ("j", c(5, Manner::Approximant, true)),
    ("w", c(0, Manner::Approximant, true)),
    ("i", Sound::Vowel(I)),
    ("ɪ", Sound::Vowel(SMALL_I)),
    ("e", Sound::Vowel(E)),
    ("æ", Sound::Vowel(v(5.0, 0.0, false, false, false))),
    ("a", Sound::Vowel(A)),
    ("ʌ", Sound::Vowel(v(4.0, 1.5, false, false, false))),
    ("ɑ", Sound::Vowel(v(6.0, 2.0, false, false, false))),
    ("ɒ", Sound::Vowel(v(5.5, 2.0, true, false, false))),
    ("ɔ", Sound::Vowel(OPEN_O)),
    ("o", Sound::Vowel(O)),
    ("ʊ", Sound::Vowel(SMALL_U)),
    ("u", Sound::Vowel(v(0.0, 2.0, true, false, false))),
    ("ə", Sound::Vowel(SCHWA)),
    ("ɚ", Sound::Vowel(v(3.0, 1.0, false, false, true))),
    ("ɜ", Sound::Vowel(SCHWA)),
    ("ɝ", Sound::Vowel(v(3.0, 1.0, false, false, true))),
];

/// A reading cut into its sounds, each its place in `SOUNDS`; stress marks
/// and the spaces between a phrase's words left out. None for a reading
/// with a letter the table never writes.
fn sounds_of(ipa: &str) -> Option<Vec<u8>> {
    let mut out = vec![];
    let mut rest = ipa;
    while let Some(first) = rest.chars().next() {
        if matches!(first, 'ˈ' | 'ˌ' | ' ' | '.') {
            rest = &rest[first.len_utf8()..];
            continue;
        }
        let (at, (symbol, _)) = SOUNDS
            .iter()
            .enumerate()
            .find(|(_, (symbol, _))| rest.starts_with(symbol))?;
        out.push(at as u8);
        rest = &rest[symbol.len()..];
    }
    Some(out)
}

fn sound(at: u8) -> Sound {
    SOUNDS[at as usize].1
}

fn symbol(at: u8) -> &'static str {
    SOUNDS[at as usize].0
}

/// What turning one sound into another costs: 0 for the same, up to 1.
fn changed(a: u8, b: u8) -> f32 {
    if a == b {
        return 0.0;
    }
    match (sound(a), sound(b)) {
        (
            Sound::Consonant {
                place: pa,
                manner: ma,
                voiced: va,
            },
            Sound::Consonant {
                place: pb,
                manner: mb,
                voiced: vb,
            },
        ) => {
            let voicing = if va == vb { 0.0 } else { 0.25 };
            let place = if pa == pb {
                0.0
            } else {
                0.3 + 0.05 * (pa.abs_diff(pb) - 1) as f32
            };
            (voicing + place + manners_apart(ma, mb)).min(1.0)
        }
        (Sound::Vowel(x), Sound::Vowel(y)) => vowels_apart(x, y),
        (Sound::Diphthong(x1, x2), Sound::Diphthong(y1, y2)) => (vowels_apart(x1, y1) + vowels_apart(x2, y2)) / 2.0,
        // A vowel against a diphthong: halfway between its two ends, and
        // the glide on top: *to* is no *toy*.
        (Sound::Vowel(x), Sound::Diphthong(y1, y2)) | (Sound::Diphthong(y1, y2), Sound::Vowel(x)) => {
            ((vowels_apart(x, y1) + vowels_apart(x, y2)) / 2.0 + 0.25).min(1.0)
        }
        (Sound::Consonant { .. }, _) => glide(a, b),
        (_, Sound::Consonant { .. }) => glide(b, a),
    }
}

/// A consonant against a vowel: a glide against the vowel it glides from
/// (*j* and *iː*, *w* and *uː*, *r* and *ɚ*) is near; else as far as it gets.
fn glide(consonant: u8, vowel: u8) -> f32 {
    let Sound::Vowel(v) = sound(vowel) else { return 1.0 };
    match symbol(consonant) {
        "j" if v.height <= 1.0 && v.back <= 0.5 => 0.6,
        "w" if v.height <= 1.0 && v.back >= 1.5 => 0.6,
        "r" if v.rhotic => 0.5,
        _ => 1.0,
    }
}

fn manners_apart(a: Manner, b: Manner) -> f32 {
    use Manner::*;
    match (a, b) {
        _ if a == b => 0.0,
        (Stop, Affricate) | (Affricate, Stop) | (Affricate, Fricative) | (Fricative, Affricate) => 0.2,
        (Lateral, Approximant) | (Approximant, Lateral) => 0.25,
        (Stop, Fricative) | (Fricative, Stop) => 0.35,
        (Nasal, Stop) | (Stop, Nasal) => 0.4,
        _ => 0.5,
    }
}

fn vowels_apart(a: Vowel, b: Vowel) -> f32 {
    let flag = |x: bool, y: bool| if x == y { 0.0 } else { 0.15 };
    (0.1 * (a.height - b.height).abs()
        + 0.18 * (a.back - b.back).abs()
        + flag(a.round, b.round)
        + flag(a.long, b.long)
        + flag(a.rhotic, b.rhotic))
    .min(1.0)
}

/// What dropping or adding a sound costs: least for a weak vowel (*ə*) and
/// *h*, which speech drops anywhere, and for a sound at the end of a word,
/// where fast speech drops it (*las' night*); most for a vowel inside one.
fn dropped(at: u8, at_end: bool) -> f32 {
    match (symbol(at), sound(at)) {
        ("ə" | "ɚ" | "h", _) => 0.35,
        (_, Sound::Consonant { .. }) if at_end => 0.45,
        (_, Sound::Consonant { .. }) => 0.75,
        _ if at_end => 0.6,
        _ => 0.85,
    }
}

#[derive(Clone, Copy)]
enum Step {
    Kept,
    Changed,
    Dropped,
    Added,
}

/// The cheapest way to turn `a`'s sounds into `b`'s, and its steps.
fn steps(a: &[u8], b: &[u8]) -> (f32, Vec<Vec<(f32, Step)>>) {
    let (n, m) = (a.len(), b.len());
    let mut d = vec![vec![(0.0_f32, Step::Kept); m + 1]; n + 1];
    // Dropped before anything of `b`: at the end only when `b` is nothing.
    for i in 1..=n {
        d[i][0] = (d[i - 1][0].0 + dropped(a[i - 1], m == 0), Step::Dropped);
    }
    for j in 1..=m {
        d[0][j] = (d[0][j - 1].0 + dropped(b[j - 1], n == 0), Step::Added);
    }
    for i in 1..=n {
        for j in 1..=m {
            let change = changed(a[i - 1], b[j - 1]);
            let options = [
                (
                    d[i - 1][j - 1].0 + change,
                    if change == 0.0 { Step::Kept } else { Step::Changed },
                ),
                (d[i - 1][j].0 + dropped(a[i - 1], j == m), Step::Dropped),
                (d[i][j - 1].0 + dropped(b[j - 1], i == n), Step::Added),
            ];
            d[i][j] = options
                .into_iter()
                .min_by(|x, y| x.0.total_cmp(&y.0))
                .expect("three options");
        }
    }
    (d[n][m].0, d)
}

fn distance(a: &[u8], b: &[u8]) -> f32 {
    // Rounded, so a sum that should be 0.75 never ranks below one that is.
    (steps(a, b).0 * 1000.0).round() / 1000.0
}

/// What differs between two words' sounds, in plain IPA: `/ɝː/ for /ʊ/`,
/// `+ /l/`, `− /t/`.
fn differences(a: &[u8], b: &[u8]) -> String {
    let (_, d) = steps(a, b);
    let (mut i, mut j) = (a.len(), b.len());
    let mut said = vec![];
    while i > 0 || j > 0 {
        let step = if i == 0 {
            Step::Added
        } else if j == 0 {
            Step::Dropped
        } else {
            d[i][j].1
        };
        match step {
            Step::Kept => {
                i -= 1;
                j -= 1;
            }
            Step::Changed => {
                said.push(format!("/{}/ for /{}/", symbol(b[j - 1]), symbol(a[i - 1])));
                i -= 1;
                j -= 1;
            }
            Step::Dropped => {
                said.push(format!("− /{}/", symbol(a[i - 1])));
                i -= 1;
            }
            Step::Added => {
                said.push(format!("+ /{}/", symbol(b[j - 1])));
                j -= 1;
            }
        }
    }
    said.reverse();
    said.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(word: &str, options: &AlikeOptions) -> Vec<(String, AlikeKind)> {
        alikes(word, options)
            .into_iter()
            .map(|alike| (alike.word, alike.kind))
            .collect()
    }

    fn has(found: &[(String, AlikeKind)], word: &str, kind: AlikeKind) -> bool {
        found.iter().any(|(w, k)| w == word && *k == kind)
    }

    #[test]
    fn sounds_are_cut_whole() {
        let cut = |ipa: &str| sounds_of(ipa).unwrap().into_iter().map(symbol).collect::<Vec<_>>();
        assert_eq!(cut("tʃɝːtʃ"), ["tʃ", "ɝː", "tʃ"]);
        assert_eq!(cut("ˈwʊdəv"), ["w", "ʊ", "d", "ə", "v"]);
        assert_eq!(cut("gəʊ"), ["g", "əʊ"]);
        assert_eq!(sounds_of("wʊdx"), None, "a letter the table never writes");
    }

    #[test]
    fn the_same_sound_comes_first() {
        let found = words("to", &AlikeOptions::default());
        assert!(
            has(&found[..2], "too", AlikeKind::Same) && has(&found[..2], "two", AlikeKind::Same),
            "{found:?}"
        );
        let found = words("their", &AlikeOptions::default());
        assert!(
            has(&found, "there", AlikeKind::Same) && has(&found, "they're", AlikeKind::Same),
            "{found:?}"
        );
        assert!(has(&words("right", &AlikeOptions::default()), "write", AlikeKind::Same));
        assert!(has(&words("would", &AlikeOptions::default()), "wood", AlikeKind::Same));
    }

    #[test]
    fn endings_are_found_either_way() {
        let options = AlikeOptions {
            kinds: vec![AlikeKind::Ending],
            most: 20,
            ..AlikeOptions::default()
        };
        assert!(has(&words("use", &options), "used", AlikeKind::Ending));
        assert!(has(&words("used", &options), "use", AlikeKind::Ending));
        assert!(has(&words("seem", &options), "seemed", AlikeKind::Ending));
        let would = words("would", &options);
        assert!(
            has(&would, "would've", AlikeKind::Ending) && has(&would, "wouldn't", AlikeKind::Ending),
            "{would:?}"
        );
        assert!(has(&words("can", &options), "can't", AlikeKind::Ending));
    }

    #[test]
    fn near_words_are_a_sound_or_two_apart() {
        let options = AlikeOptions {
            most: 40,
            ..AlikeOptions::default()
        };
        assert!(has(&words("would", &options), "word", AlikeKind::Near));
        assert!(has(&words("ship", &options), "sheep", AlikeKind::Near));
        assert!(has(&words("think", &options), "sink", AlikeKind::Near));
        assert!(has(&words("walk", &options), "work", AlikeKind::Near));
    }

    #[test]
    fn a_word_is_never_its_own_alike() {
        assert!(
            !words("would", &AlikeOptions::default())
                .iter()
                .any(|(w, _)| w == "would")
        );
    }

    #[test]
    fn both_accents_count_unless_one_is_asked_for() {
        // *caught* and *court* are one sound in British English only.
        assert!(has(
            &words("caught", &AlikeOptions::default()),
            "court",
            AlikeKind::Same
        ));
        let us = AlikeOptions {
            accent: Some(Accent::Us),
            most: 40,
            ..AlikeOptions::default()
        };
        assert!(!has(&words("caught", &us), "court", AlikeKind::Same));
    }

    #[test]
    fn no_candidate_is_above_the_level_after_the_learners() {
        let options = AlikeOptions {
            level: Some(Level::A2),
            most: 50,
            ..AlikeOptions::default()
        };
        for alike in alikes("would", &options) {
            assert!(
                known_level(&alike.word).is_some_and(|level| level <= Level::B1),
                "{}",
                alike.word
            );
        }
    }

    #[test]
    fn a_word_the_table_lacks_has_none() {
        assert!(alikes("Pasadenaish", &AlikeOptions::default()).is_empty());
        assert!(alikes("", &AlikeOptions::default()).is_empty());
    }

    #[test]
    fn what_differs_is_said_in_sounds() {
        let options = AlikeOptions {
            most: 40,
            ..AlikeOptions::default()
        };
        let used = alikes("use", &options).into_iter().find(|a| a.word == "used").unwrap();
        assert_eq!(used.why, "+ /d/");
        let wood = alikes("would", &options)
            .into_iter()
            .find(|a| a.word == "wood")
            .unwrap();
        assert_eq!(wood.why, "the same sound");
    }
}
