//! A line's features, the facts behind them, and from those facts whether
//! it can be practised and what to say about it.

use std::collections::BTreeMap;

use crate::connected::{connected_speech, line_words, melts};
use crate::levels::{Level, word_level};
use crate::{Pronounced, pronounce, readings};

use super::heard::{Hearing, WordKind, hearing};
use super::{Entry, LineInput, Reason, Skill, WhyNot};
/// A line's features by name, the numbers a model weighs. Their names are
/// the contract with the models in `models/line/`:
///
/// | name | what |
/// |---|---|
/// | `words`, `syllables` | how many |
/// | `sps` | syllables a second |
/// | `sentences` | how many sentences it holds |
/// | `vocab` | its hardest word's level (1–6), an entry's level counted; one hard word in an easy line counts half a level less |
/// | `hard_words` | words above B1 |
/// | `common` | the share of its words at A1–B1 |
/// | `names` | names in it |
/// | `ear_melts` | melts that hide words from the ear: fusion, yod, the soft t, a dropped t or d, a dropped h, *'em*, assimilation, the cut-off t, *-in'* |
/// | `melts`, `weak_forms` | every melt; the small words said lightly |
/// | `hard_sounds` | `th`, `w`, an s before a consonant, three consonants at a word's end |
/// | `contractions` | words with an apostrophe |
/// | `question`, `exclaim` | 1 if it ends so |
/// | `formula` | 1 if it opens with an everyday frame (*Can I…*, *I don't know*) |
/// | `entries` | words and phrases a guide teaches in it |
/// | `heard` | 1 if the weak ear heard it |
/// | `miss_tiny`, `miss_base`, `miss_small`, `unsure` | the weak ear, per word ([`super::heard`]) |
pub type Features = BTreeMap<String, f64>;

/// What a line was found to hold.
#[derive(Debug, Clone)]
pub struct Facts {
    pub features: Features,
    pub tokens: Vec<String>,
    pub whole_sentence: bool,
    pub voices: u8,
    pub duration_ms: u32,
    pub syllables: usize,
    pub names: Vec<String>,
    pub said_wrong: Vec<String>,
    pub hearing: Option<Hearing>,
    /// rule, the words it joins, how they sound.
    pub melts: Vec<(String, String, String)>,
    pub written: Vec<(String, String)>,
    pub hard_words: Vec<(String, Level)>,
    pub hard_sounds: Vec<(String, String)>,
    pub teaches: Vec<Entry>,
    pub common: f64,
    pub question: bool,
    pub exclaim: bool,
}

/// Melts that hide a word from the ear, rather than only joining two.
const EAR_MELTS: &[&str] = &[
    "fusion",
    "yod",
    "flap-t",
    "elision",
    "h-dropping",
    "them",
    "assimilation",
    "glottal-t",
    "g-dropping",
];

/// Words said for sound or for time, not for their meaning: never graded.
const FILLERS: &[&str] = &[
    "oh", "uh", "um", "ah", "hmm", "huh", "hey", "yeah", "yes", "no", "okay", "ok", "well", "wow", "whoa", "so", "like",
];

/// Everyday frames a learner can reuse.
const FRAMES: &[&str] = &[
    "can i",
    "could you",
    "can you",
    "would you",
    "do you want",
    "i'm going to",
    "i'm gonna",
    "let's",
    "i don't know",
    "i think",
    "what are you",
    "what do you",
    "how do you",
    "i need",
    "i want",
    "you have to",
    "you know what",
    "why don't",
    "there's no",
    "i can't",
    "i was just",
    "it's not",
    "i didn't",
    "are you",
    "is it",
    "don't",
    "i'll",
];

/// The short forms a contraction stands for.
fn expand(token: &str) -> Vec<String> {
    let special: &[(&str, &[&str])] = &[
        ("won't", &["will", "not"]),
        ("can't", &["can", "not"]),
        ("shan't", &["shall", "not"]),
        ("ain't", &["am", "not"]),
        ("gonna", &["going", "to"]),
        ("wanna", &["want", "to"]),
        ("gotta", &["got", "to"]),
        ("y'all", &["you", "all"]),
        ("let's", &["let", "us"]),
        ("c'mon", &["come", "on"]),
    ];
    if let Some((_, full)) = special.iter().find(|(form, _)| *form == token) {
        return full.iter().map(|w| w.to_string()).collect();
    }
    for (short, full) in [
        ("n't", "not"),
        ("'re", "are"),
        ("'m", "am"),
        ("'ll", "will"),
        ("'ve", "have"),
        ("'d", "would"),
        ("'s", "is"),
    ] {
        if let Some(stem) = token.strip_suffix(short).filter(|stem| !stem.is_empty()) {
            return vec![stem.to_string(), full.to_string()];
        }
    }
    vec![token.to_string()]
}

fn known(word: &str, accent: crate::Accent) -> bool {
    !matches!(pronounce(word, accent), Pronounced::Unknown)
}

/// A word's sound: the table's, or its first reading where its readings
/// differ (*with*), as both have the same syllables and hard sounds.
fn ipa(word: &str, accent: crate::Accent) -> Option<String> {
    match pronounce(word, accent) {
        Pronounced::Known(sound) => Some(sound),
        Pronounced::Ambiguous => readings(word, accent).into_iter().next(),
        Pronounced::Unknown => None,
    }
}

const IPA_VOWELS: &str = "aeiouæɑɒɔəɚɛɜɝɪʊʌɐᵻy";

/// How many syllables a word has: its sound's vowels, or its spelling's.
fn syllables(word: &str, accent: crate::Accent) -> usize {
    let count = |text: &str, vowels: &str| {
        let mut groups = 0;
        let mut inside = false;
        for c in text.chars() {
            let vowel = vowels.contains(c) || c == '\u{0329}';
            if vowel && !inside {
                groups += 1;
            }
            inside = vowel;
        }
        groups
    };
    match ipa(word, accent) {
        Some(sound) => count(&sound, IPA_VOWELS).max(1),
        None => count(word.trim_end_matches('e'), "aeiouy").max(1),
    }
}

/// The words the line's `w`s could stand for, said for an r or an l
/// (*weawy*: *really*).
fn w_readings(word: &str) -> Vec<String> {
    let parts: Vec<&str> = word.split('w').collect();
    let mut out = vec![parts[0].to_string()];
    for part in &parts[1..] {
        out = out
            .iter()
            .flat_map(|head| ["w", "r", "l", "ll"].map(|c| format!("{head}{c}{part}")))
            .collect();
    }
    out
}

fn hard_sounds_of(word: &str, accent: crate::Accent) -> Vec<String> {
    let Some(sound) = ipa(word, accent) else { return vec![] };
    let sound: String = sound.chars().filter(|c| !matches!(c, 'ˈ' | 'ˌ')).collect();
    let mut found = vec![];
    let mut chars = sound.chars();
    if chars.next() == Some('s') && chars.next().is_some_and(|c| "ptkmnlw".contains(c)) {
        found.push("s-cluster".to_string());
    }
    let common_th = ["the", "this", "that", "they", "them", "there", "then", "than"];
    if (sound.contains('θ') || sound.contains('ð')) && !common_th.contains(&word) {
        found.push("th".to_string());
    }
    if sound.contains('w') {
        found.push("w".to_string());
    }
    let tail = sound
        .chars()
        .rev()
        .take_while(|c| !IPA_VOWELS.contains(*c) && *c != 'ː')
        .count();
    if tail >= 3 {
        found.push("end-cluster".to_string());
    }
    found
}

/// The facts of a line.
pub fn facts(input: &LineInput) -> Facts {
    let text = input.text.trim();
    let accent = input.accent;
    let ranges = line_words(text);
    let tokens: Vec<String> = ranges
        .iter()
        .map(|r| text[r.clone()].to_lowercase().replace('’', "'"))
        .collect();
    let known_names: Vec<String> = input.names.iter().map(|n| n.to_lowercase()).collect();

    // A name: a word with a capital where no sentence starts, a word known
    // as a name in the episode, or a word nothing knows.
    let capital: Vec<bool> = ranges
        .iter()
        .map(|r| {
            let word = &text[r.clone()];
            let before = text[..r.start].trim_end();
            word.starts_with(char::is_uppercase)
                && !matches!(word, "I" | "I'm" | "I'll" | "I've" | "I'd" | "OK")
                && before.chars().last().is_some_and(|c| !".?!-\"“…".contains(c))
        })
        .collect();
    let said_wrong: Vec<String> = tokens
        .iter()
        .enumerate()
        .filter(|(at, t)| {
            !capital[*at]
                && !known(t, accent)
                && word_level(t).is_none()
                && (1..=3).contains(&t.matches('w').count())
                && w_readings(t)
                    .iter()
                    .any(|r| known(r, accent) || word_level(r).is_some())
        })
        .map(|(_, t)| t.clone())
        .collect();
    let is_name: Vec<bool> = tokens
        .iter()
        .enumerate()
        .map(|(at, t)| {
            !said_wrong.contains(t)
                && (capital[at]
                    || (known_names.contains(t) && word_level(t).is_none())
                    || (!known(t, accent) && word_level(t).is_none() && !t.chars().all(|c| c.is_ascii_digit())))
        })
        .collect();
    let names: Vec<String> = tokens
        .iter()
        .zip(&is_name)
        .filter(|(_, n)| **n)
        .map(|(t, _)| t.clone())
        .collect();

    // Words and their levels.
    let teaches: Vec<Entry> = input.entries.iter().filter(|e| e.learnt()).cloned().collect();
    let mut levels: Vec<(f64, String)> = vec![];
    let mut hard_words: Vec<(String, Level)> = vec![];
    for (at, token) in tokens.iter().enumerate() {
        if is_name[at] || said_wrong.contains(token) {
            continue;
        }
        for word in expand(token) {
            if FILLERS.contains(&word.as_str()) {
                continue;
            }
            let level = word_level(&word).or_else(|| known(&word, accent).then_some(Level::C1));
            if let Some(level) = level {
                levels.push((level.rank(), word.clone()));
                if level > Level::B1 && !hard_words.iter().any(|(w, _)| *w == word) {
                    hard_words.push((word, level));
                }
            }
        }
    }
    let common = if levels.is_empty() {
        1.0
    } else {
        levels.iter().filter(|(l, _)| *l <= 3.0).count() as f64 / levels.len() as f64
    };
    for entry in &teaches {
        if let Some(level) = entry.level {
            levels.push((level.rank(), entry.term.clone()));
        }
    }
    levels.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut vocab = levels.first().map_or(1.0, |l| l.0);
    if levels.len() > 1 && levels[0].0 - levels[1].0 >= 2.0 {
        vocab -= 0.5;
    }

    // Sound.
    let syllable_count: usize = tokens.iter().map(|t| syllables(t, accent)).sum();
    let seconds = (input.duration_ms.max(400) as f64) / 1000.0;
    let found = melts(text, accent);
    let melt_list: Vec<(String, String, String)> = found
        .iter()
        .filter_map(|m| {
            let first = ranges.get(m.words.start)?;
            let last = ranges.get(m.words.end.checked_sub(1)?)?;
            Some((
                m.rule.to_string(),
                text[first.start..last.end].to_string(),
                m.sound.clone(),
            ))
        })
        .collect();
    let written: Vec<(String, String)> = tokens
        .iter()
        .filter_map(|t| {
            connected_speech()
                .fusions
                .iter()
                .find(|f| f.sound == *t)
                .map(|f| (t.clone(), f.words.clone()))
        })
        .collect();
    let hard_sounds: Vec<(String, String)> = tokens
        .iter()
        .enumerate()
        .filter(|(at, _)| !is_name[*at])
        .flat_map(|(_, t)| hard_sounds_of(t, accent).into_iter().map(move |s| (s, t.clone())))
        .collect();
    let ear_melts = melt_list
        .iter()
        .filter(|(rule, _, _)| EAR_MELTS.contains(&rule.as_str()))
        .count()
        + written.len();

    let hearing = input
        .heard
        .as_ref()
        .map(|heard| hearing(&tokens, |at| is_name[at], heard));

    let lower = text.to_lowercase().replace('’', "'");
    let lower = lower.trim_start_matches(['"', '“']);
    let frame = FRAMES.iter().any(|f| {
        lower
            .strip_prefix(f)
            .is_some_and(|rest| rest.is_empty() || !rest.starts_with(char::is_alphanumeric))
    });
    let end = text.trim_end_matches(['"', '”']);
    let question = end.ends_with('?');
    let exclaim = end.ends_with('!');
    let whole_sentence = text
        .trim_start_matches(['"', '“'])
        .starts_with(|c: char| c.is_uppercase() || c.is_ascii_digit())
        && (end.ends_with(['.', '?', '!']))
        && !end.ends_with("...")
        && !end.ends_with('…')
        && !end.ends_with("--");
    let sentences = {
        let chars: Vec<char> = text.chars().collect();
        (0..chars.len())
            .filter(|&i| {
                ".?!".contains(chars[i]) && chars.get(i + 1).is_none_or(|c| c.is_whitespace() || "\"”".contains(*c))
            })
            .count()
            .max(1)
    };

    let mut features = Features::new();
    let mut set = |name: &str, value: f64| {
        features.insert(name.to_string(), value);
    };
    set("words", tokens.len() as f64);
    set("syllables", syllable_count as f64);
    set("sps", syllable_count as f64 / seconds);
    set("sentences", sentences as f64);
    set("vocab", vocab);
    set("hard_words", hard_words.len() as f64);
    set("common", common);
    set("names", names.len() as f64);
    set("ear_melts", ear_melts as f64);
    set("melts", melt_list.len() as f64);
    set(
        "weak_forms",
        melt_list.iter().filter(|(r, _, _)| r == "weak-form").count() as f64,
    );
    set("hard_sounds", hard_sounds.len() as f64);
    set(
        "contractions",
        tokens.iter().filter(|t| t.contains('\'')).count() as f64,
    );
    set("question", f64::from(u8::from(question)));
    set("exclaim", f64::from(u8::from(exclaim)));
    set("formula", f64::from(u8::from(frame)));
    set("entries", teaches.len() as f64);
    set("heard", f64::from(u8::from(hearing.is_some())));
    let h = hearing.clone().unwrap_or_default();
    set("miss_tiny", h.miss_tiny);
    set("miss_base", h.miss_base);
    set("miss_small", h.miss_small);
    set("unsure", h.unsure);

    Facts {
        features,
        tokens,
        whole_sentence,
        voices: input.voices,
        duration_ms: input.duration_ms,
        syllables: syllable_count,
        names,
        said_wrong,
        hearing,
        melts: melt_list,
        written,
        hard_words,
        hard_sounds,
        teaches,
        common,
        question,
        exclaim,
    }
}

impl Facts {
    /// Why the line can't be practised for a skill; none, if it can.
    pub fn why_not(&self, skill: Skill) -> Vec<WhyNot> {
        let mut why = vec![];
        if !self.whole_sentence {
            why.push(WhyNot::NotASentence);
        }
        if self.voices > 1 {
            why.push(WhyNot::TwoVoices);
        }
        if self.names.len() > 1 {
            why.push(WhyNot::Names {
                words: self.names.clone(),
            });
        }
        if !self.said_wrong.is_empty() {
            why.push(WhyNot::SaidWrong {
                words: self.said_wrong.clone(),
            });
        }
        if self.hearing.as_ref().is_some_and(|h| h.miss_small >= 0.3) {
            why.push(WhyNot::NotClear);
        }
        let words = self.tokens.len();
        let (fewest, most) = match skill {
            Skill::Ear => (4, 15),
            Skill::Mouth => (3, 15),
        };
        if words < fewest {
            why.push(WhyNot::TooShort { words });
        }
        if words > most {
            why.push(WhyNot::TooLong { words });
        }
        if skill == Skill::Mouth && !(1_000..=7_000).contains(&self.duration_ms) {
            why.push(WhyNot::Spoken { ms: self.duration_ms });
        }
        why
    }

    /// What makes the line hard, and worth it, for a skill — for an app to
    /// put in words; it chooses which to show.
    pub fn reasons(&self, skill: Skill) -> Vec<Reason> {
        let mut out = vec![Reason::Pace {
            words: self.tokens.len(),
            seconds: (self.duration_ms as f64 / 100.0).round() / 10.0,
            syllables: self.syllables,
        }];
        for (word, full) in &self.written {
            out.push(Reason::Written {
                word: word.clone(),
                full: full.clone(),
            });
        }
        for (rule, text, sound) in &self.melts {
            let ear_rule = EAR_MELTS.contains(&rule.as_str());
            if skill == Skill::Mouth || ear_rule {
                out.push(Reason::Melt {
                    rule: rule.clone(),
                    text: text.clone(),
                    sound: sound.clone(),
                });
            }
        }
        if skill == Skill::Ear {
            if let Some(h) = &self.hearing {
                for w in &h.words {
                    let missed = !w.base || !w.small || (!w.tiny && w.p.is_some_and(|p| p < 0.6));
                    let unsure = w.small && w.p.is_some_and(|p| p < 0.5);
                    if w.kind == WordKind::Word
                        && (missed || unsure)
                        && !out
                            .iter()
                            .any(|r| matches!(r, Reason::WeakEar { word } if *word == w.word))
                    {
                        out.push(Reason::WeakEar { word: w.word.clone() });
                    }
                }
            }
        }
        if skill == Skill::Mouth {
            for (sound, word) in &self.hard_sounds {
                out.push(Reason::HardSound {
                    sound: sound.clone(),
                    word: word.clone(),
                });
            }
        }
        for (word, level) in &self.hard_words {
            out.push(Reason::HardWord {
                word: word.clone(),
                level: *level,
            });
        }
        for entry in &self.teaches {
            out.push(Reason::Teaches {
                term: entry.term.clone(),
                kind: entry.kind.clone(),
            });
        }
        out.push(Reason::Everyday {
            share: (self.common * 100.0).round() / 100.0,
        });
        if skill == Skill::Mouth && (self.question || self.exclaim) {
            out.push(Reason::Tune {
                question: self.question,
            });
        }
        out
    }
}