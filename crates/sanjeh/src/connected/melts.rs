//! The rule engine (docs/product/melt-plan.md, phase 2): where the words of
//! a line melt into each other, by the twelve rules, from what the
//! pronouncing table says each word sounds like in the learner's accent.
//!
//! A line is cut into phrases at its punctuation — a speaker pauses there —
//! and nothing melts across one. Fixed fusions (*going to* → *gonna*) are
//! found first and keep their words. Then each joint between two words
//! takes the first rule that fits it, in this order: yod, *'em*, the silent
//! h, the American soft t, the British linking r, linking, a dropped t or d,
//! the British cut-off t, assimilation. Rules of one word — a weak form,
//! -in', the soft t inside a word — are found beside them. Links in a row
//! are one melt: *pick it up* → *pi-ki-tup*.
//!
//! The sound is written in plain letters by moving a word's last consonant
//! onto the next word, read from the table's IPA and the word's spelling;
//! where the spelling doesn't plainly end in that consonant, the words are
//! only hyphened.

use std::ops::Range;

use super::connected_speech;
use crate::{Accent, Pronounced, pronounce};

/// One place in a line where words melt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Melt {
    /// The rule, by its id in the data: `linking`.
    pub rule: &'static str,
    /// Which words of the line, by their place in it.
    pub words: Range<usize>,
    /// Where they are in the line, in UTF-16 units, as the app counts.
    pub span: Range<usize>,
    /// How they sound, in plain letters: `pi-ki-tup`.
    pub sound: String,
    /// Their IPA in the accent, `‿` at a joint.
    pub ipa: String,
}

/// Every melt in `line`, as a speaker of `accent` makes them, in the
/// order of the line.
pub fn melts(line: &str, accent: Accent) -> Vec<Melt> {
    let words = words(line, accent);
    let mut found = Vec::new();
    let mut taken = vec![false; words.len()];

    fusions(&words, accent, &mut taken, &mut found);
    let dropped = joints(&words, accent, &taken, &mut found);
    for (at, word) in words.iter().enumerate() {
        if taken[at] {
            continue;
        }
        let last_in_phrase = words
            .get(at + 1)
            .is_none_or(|next| next.phrase != word.phrase);
        let mut alone = vec![g_dropping(word)];
        if accent == Accent::Us {
            alone.push(soft_t_inside(word));
        }
        if accent == Accent::Gb && last_in_phrase {
            alone.push(cut_off_t(word, None));
        }
        if !last_in_phrase && !dropped.contains(&at) {
            alone.push(weak_form(&words, at, accent));
        }
        for mut melt in alone.into_iter().flatten() {
            melt.words = at..at + 1;
            found.push(melt);
        }
    }

    found.sort_by_key(|melt| (melt.words.start, melt.words.end));
    for melt in &mut found {
        let (from, to) = (&words[melt.words.start], &words[melt.words.end - 1]);
        melt.span = utf16(line, from.bytes.start)..utf16(line, to.bytes.end);
    }
    found
}

// ── the line's words ────────────────────────────────────────────────────

struct Word<'a> {
    text: &'a str,
    /// Lower case, every apostrophe a straight one.
    lower: String,
    bytes: Range<usize>,
    /// Which phrase of the line: punctuation starts the next.
    phrase: usize,
    /// What the table says it sounds like; none for a word it doesn't
    /// know, or reads two ways.
    ipa: Option<String>,
}

/// The line's words as the engine counts them — a melt's `words` are
/// places in this list — by their bytes in the line.
pub fn line_words(line: &str) -> Vec<Range<usize>> {
    phrased(line).into_iter().map(|(bytes, _)| bytes).collect()
}

/// The line's words, each with its phrase: punctuation starts the next.
fn phrased(line: &str) -> Vec<(Range<usize>, usize)> {
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let letter = |at: usize| chars.get(at).is_some_and(|(_, c)| c.is_alphanumeric());
    let mut words = Vec::new();
    let mut phrase = 0;
    let mut paused = false;
    let mut at = 0;
    while at < chars.len() {
        let (start, c) = chars[at];
        if c.is_alphanumeric() {
            let mut end = at + 1;
            // An apostrophe or a hyphen inside a word is part of it.
            while end < chars.len()
                && (letter(end) || (matches!(chars[end].1, '\'' | '’' | '-') && letter(end + 1)))
            {
                end += 1;
            }
            if paused && !words.is_empty() {
                phrase += 1;
            }
            paused = false;
            words.push((
                start..chars.get(end).map_or(line.len(), |(at, _)| *at),
                phrase,
            ));
            at = end;
        } else {
            if !c.is_whitespace() {
                paused = true;
            }
            at += 1;
        }
    }
    words
}

fn words(line: &str, accent: Accent) -> Vec<Word<'_>> {
    phrased(line)
        .into_iter()
        .map(|(bytes, phrase)| {
            let text = &line[bytes.clone()];
            // A word in capitals is letters said one by one (GPS, TV):
            // the table's sound for it isn't the word's.
            let spelt_out = text.chars().filter(|c| c.is_alphabetic()).count() > 1
                && !text.chars().any(char::is_lowercase);
            Word {
                text,
                lower: text.to_lowercase().replace('’', "'"),
                bytes,
                phrase,
                ipa: match pronounce(text, accent) {
                    Pronounced::Known(sound) if !spelt_out => Some(sound),
                    _ => None,
                },
            }
        })
        .collect()
}

fn utf16(line: &str, byte: usize) -> usize {
    line[..byte].encode_utf16().count()
}

// ── sounds ──────────────────────────────────────────────────────────────

const VOWELS: &str = "iɪeɛæaʌɑɒɔoʊuəɚɝɜ";

fn is_vowel(c: char) -> bool {
    VOWELS.contains(c)
}

fn is_mark(c: char) -> bool {
    matches!(c, 'ˈ' | 'ˌ' | 'ː' | '\u{0329}')
}

fn first_sound(ipa: &str) -> Option<char> {
    ipa.chars().find(|c| !is_mark(*c))
}

fn last_sound(ipa: &str) -> Option<char> {
    ipa.chars().rev().find(|c| !is_mark(*c))
}

fn starts_with_vowel(ipa: &str) -> bool {
    first_sound(ipa).is_some_and(is_vowel)
}

fn starts_with_consonant(ipa: &str) -> bool {
    first_sound(ipa).is_some_and(|c| !is_vowel(c))
}

/// The word's last sound when it is a consonant, `tʃ` and `dʒ` as one.
fn final_consonant(ipa: &str) -> Option<&str> {
    let trimmed = ipa.trim_end_matches(is_mark);
    if trimmed.ends_with("tʃ") || trimmed.ends_with("dʒ") {
        return Some(&trimmed[trimmed.len() - "tʃ".len()..]);
    }
    let last = trimmed.chars().last()?;
    (!is_vowel(last)).then(|| &trimmed[trimmed.len() - last.len_utf8()..])
}

/// The sound before the word's last one.
fn sound_before_last(ipa: &str) -> Option<char> {
    let mut sounds = ipa.chars().rev().filter(|c| !is_mark(*c));
    sounds.next()?;
    sounds.next()
}

/// The IPA without its last sound.
fn without_last(ipa: &str) -> &str {
    let trimmed = ipa.trim_end_matches(is_mark);
    let cut = final_consonant(trimmed).map_or_else(
        || trimmed.chars().last().map_or(0, char::len_utf8),
        str::len,
    );
    &trimmed[..trimmed.len() - cut]
}

/// Syllables, counted by their vowels: a long vowel ends at its length
/// mark (*doing*, /ˈduːɪŋ/, has two), and a diphthong followed by another
/// vowel is two (*going*, /ˈgoʊɪŋ/).
fn syllables(ipa: &str) -> usize {
    let mut count = 0;
    let mut run = 0;
    for c in ipa.chars() {
        if is_vowel(c) {
            run += 1;
            if run == 1 || run == 3 {
                count += 1;
            }
        } else if c != 'ˈ' && c != 'ˌ' {
            run = 0;
        }
    }
    count
}

// ── letters ─────────────────────────────────────────────────────────────

/// How a consonant may be spelt at the end of a word, longest first.
fn spellings(sound: &str) -> &'static [&'static str] {
    match sound {
        "k" => &["que", "ck", "k", "c"],
        "t" => &["'t", "tt", "ed", "t"],
        "d" => &["'d", "dd", "ed", "d"],
        "s" => &["'s", "ss", "ce", "se", "s"],
        "z" => &["'s", "zz", "se", "ze", "s", "z"],
        "l" => &["'ll", "ll", "le", "l"],
        "m" => &["'m", "mm", "mb", "me", "m"],
        "n" => &["nn", "ne", "n"],
        "p" => &["pp", "pe", "p"],
        "b" => &["bb", "be", "b"],
        "g" => &["gue", "gg", "g"],
        "f" => &["ff", "gh", "fe", "f"],
        "v" => &["'ve", "ve", "v", "f"],
        "θ" | "ð" => &["the", "th"],
        "ʃ" => &["sh"],
        "tʃ" => &["tch", "ch"],
        "dʒ" => &["dge", "ge"],
        "ʒ" => &["ge"],
        "ŋ" => &["ng"],
        "r" => &["'re", "rr", "re", "r"],
        _ => &[],
    }
}

/// A consonant sound of the table's IPA as a name the spellings know.
fn static_sound(sound: &str) -> &'static str {
    const KNOWN: [&str; 24] = [
        "k", "t", "d", "s", "z", "l", "m", "n", "p", "b", "g", "f", "v", "θ", "ð", "ʃ", "tʃ", "dʒ",
        "ʒ", "ŋ", "r", "w", "j", "h",
    ];
    KNOWN
        .into_iter()
        .find(|known| *known == sound)
        .unwrap_or("")
}

/// A consonant as it is written when it moves onto the next word.
fn written(sound: &str) -> &'static str {
    match sound {
        "θ" | "ð" => "th",
        "ʃ" => "sh",
        "tʃ" => "ch",
        "dʒ" => "j",
        "ʒ" => "zh",
        "ŋ" => "ng",
        "ɡ" | "g" => "g",
        "k" => "k",
        "t" => "t",
        "d" => "d",
        "s" => "s",
        "z" => "z",
        "l" => "l",
        "m" => "m",
        "n" => "n",
        "p" => "p",
        "b" => "b",
        "f" => "f",
        "v" => "v",
        "r" => "r",
        "w" => "w",
        "j" => "y",
        _ => "",
    }
}

/// The letters without the ones that spell their last consonant, when
/// they plainly do; a silent e after it goes too (*like* → *li*).
fn without_final<'a>(letters: &'a str, sound: &str) -> Option<&'a str> {
    let strip = |letters: &'a str| {
        let spelling = spellings(sound)
            .iter()
            .find(|spelling| letters.ends_with(**spelling))?;
        let rest = &letters[..letters.len() - spelling.len()];
        (!rest.is_empty() && !rest.ends_with('-')).then_some(rest)
    };
    strip(letters).or_else(|| strip(letters.strip_suffix('e')?))
}

/// `chunk` joined to `next`, which starts with a vowel: the chunk's last
/// consonant moves over (`pick`, `it` → `pi-kit`), written as `moved`; where
/// its letters don't plainly spell it, the two are only hyphened. With no
/// consonant of its own (an r slipped in, *law-rand*), `moved` is added.
fn link(chunk: &str, consonant: &str, moved: &str, next: &str) -> String {
    if consonant.is_empty() {
        return format!("{chunk}-{moved}{next}");
    }
    // An x is k and s: the s moves, the k stays (*fix it* → *fik-sit*).
    if consonant == "s"
        && let Some(rest) = chunk.strip_suffix('x')
    {
        return format!("{rest}k-s{next}");
    }
    match without_final(chunk, consonant) {
        Some(rest) => format!("{rest}-{moved}{next}"),
        None => format!("{chunk}-{next}"),
    }
}

// ── fusions ─────────────────────────────────────────────────────────────

/// Words after which *going to* and the like are a place, not a verb to
/// come: *going to the shop* isn't *gonna*.
const NOT_A_VERB: [&str; 18] = [
    "the", "a", "an", "my", "your", "his", "her", "its", "our", "their", "this", "that", "these",
    "those", "school", "work", "bed", "church",
];

fn fusions(words: &[Word], accent: Accent, taken: &mut [bool], found: &mut Vec<Melt>) {
    let data = connected_speech();
    let mut at = 0;
    while at < words.len() {
        let fused = data.fusions.iter().find_map(|fusion| {
            let parts: Vec<&str> = fusion.words.split(' ').collect();
            let end = at + parts.len();
            let these = words.get(at..end)?;
            let same = these
                .iter()
                .zip(&parts)
                .all(|(word, part)| word.lower == *part)
                && these.iter().all(|word| word.phrase == these[0].phrase);
            // *to* before a place or a name is going somewhere.
            let to_a_verb = parts.last() != Some(&"to")
                || words
                    .get(end)
                    .filter(|next| next.phrase == these[0].phrase)
                    .is_none_or(|next| {
                        !NOT_A_VERB.contains(&next.lower.as_str())
                            && !(next.text.starts_with(char::is_uppercase) && next.lower != "i")
                    });
            (same && to_a_verb).then_some((fusion, end))
        });
        match fused {
            Some((fusion, end)) => {
                taken[at..end].iter_mut().for_each(|taken| *taken = true);
                found.push(Melt {
                    rule: "fusion",
                    words: at..end,
                    span: 0..0,
                    sound: fusion.sound.clone(),
                    ipa: fusion.ipa.get(accent).unwrap_or_default().to_owned(),
                });
                at = end;
            }
            None => at += 1,
        }
    }
}

// ── joints ──────────────────────────────────────────────────────────────

const MODALS: [&str; 7] = ["would", "could", "should", "must", "might", "may", "will"];

/// What happens at the joint between two words.
enum Joint {
    /// The next word starts with its vowel: the consonant moves over, or a
    /// w or y slips in (`glide`).
    Link {
        rule: &'static str,
        /// The consonant whose letters move; none for one slipped in.
        consonant: &'static str,
        moved: &'static str,
        glide: Option<&'static str>,
    },
    Yod,
    /// *them*, *he*, *him*, *her*, *his*, *have* losing their start.
    Dropped {
        rule: &'static str,
    },
    Elided,
    CutOff,
    Assimilated {
        into: &'static str,
        letter: &'static str,
    },
}

fn joint(prev: &Word, next: &Word, accent: Accent) -> Option<Joint> {
    let (prev_ipa, next_ipa) = (prev.ipa.as_deref()?, next.ipa.as_deref()?);
    let last = final_consonant(prev_ipa);
    // An -ing word is said -in' (its own melt), and links as that.
    if says_in(prev) {
        return None;
    }
    let before = sound_before_last(prev_ipa);
    let vowel_before = before.is_some_and(|c| is_vowel(c) || c == 'r');

    if matches!(next.lower.as_str(), "you" | "your" | "you're")
        && matches!(last, Some("t" | "d" | "s" | "z"))
    {
        return Some(Joint::Yod);
    }
    if next.lower == "them" {
        return Some(Joint::Dropped { rule: "them" });
    }
    let h_word = matches!(next.lower.as_str(), "he" | "him" | "her" | "his")
        || (next.lower == "have" && MODALS.contains(&prev.lower.as_str()));
    if h_word && last.is_some() {
        return Some(Joint::Dropped { rule: "h-dropping" });
    }
    if starts_with_vowel(next_ipa) {
        if accent == Accent::Us && last == Some("t") && vowel_before {
            return Some(Joint::Link {
                rule: "flap-t",
                consonant: "t",
                moved: "d",
                glide: None,
            });
        }
        if accent == Accent::Gb && last.is_none() {
            let spelt_r = prev.lower.ends_with('r') || prev.lower.ends_with("re");
            let open = last_sound(prev_ipa).is_some_and(|c| matches!(c, 'ə' | 'ɑ' | 'ɔ' | 'ɜ'));
            let small = connected_speech()
                .weak_forms
                .iter()
                .any(|form| form.word == prev.lower);
            if spelt_r || (open && !small) {
                return Some(Joint::Link {
                    rule: "linking-r",
                    consonant: if spelt_r { "r" } else { "" },
                    moved: "r",
                    glide: None,
                });
            }
        }
        if let Some(consonant) = last {
            return Some(Joint::Link {
                rule: "linking",
                consonant: static_sound(consonant),
                moved: written(consonant),
                glide: None,
            });
        }
        let glide = match last_sound(prev_ipa)? {
            'i' | 'ɪ' => "y",
            'u' | 'ʊ' => "w",
            _ => return None,
        };
        return Some(Joint::Link {
            rule: "linking",
            consonant: "",
            moved: "",
            glide: Some(glide),
        });
    }
    if !starts_with_consonant(next_ipa) {
        return None;
    }
    if matches!(last, Some("t" | "d")) && before.is_some_and(|c| !is_vowel(c)) {
        return Some(Joint::Elided);
    }
    if accent == Accent::Gb && last == Some("t") && before.is_some_and(is_vowel) {
        return Some(Joint::CutOff);
    }
    let first = first_sound(next_ipa)?;
    let (into, letter) = match (last?, first) {
        ("n", 'p' | 'b' | 'm') => ("m", "m"),
        ("n", 'k' | 'g' | 'ɡ') => ("ŋ", "ng"),
        ("t", 'p' | 'b' | 'm') if accent == Accent::Us => ("p", "p"),
        ("t", 'k' | 'g' | 'ɡ') if accent == Accent::Us => ("k", "k"),
        ("d", 'p' | 'b' | 'm') => ("b", "b"),
        ("d", 'k' | 'g' | 'ɡ') => ("g", "g"),
        _ => return None,
    };
    Some(Joint::Assimilated { into, letter })
}

/// The melts at the joints between words; returns the words whose start a
/// joint dropped (*her* in *tell her*), which aren't weak forms besides.
fn joints(words: &[Word], accent: Accent, taken: &[bool], found: &mut Vec<Melt>) -> Vec<usize> {
    let mut dropped = Vec::new();
    // Links in a row, of one rule, are one melt.
    let mut open: Option<Melt> = None;
    let close = |open: &mut Option<Melt>, found: &mut Vec<Melt>| {
        if let Some(melt) = open.take() {
            found.push(melt);
        }
    };
    for at in 0..words.len().saturating_sub(1) {
        let (prev, next) = (&words[at], &words[at + 1]);
        if prev.phrase != next.phrase || taken[at] || taken[at + 1] {
            close(&mut open, found);
            continue;
        }
        let Some(joint) = joint(prev, next, accent) else {
            close(&mut open, found);
            continue;
        };
        let (Some(prev_ipa), Some(next_ipa)) = (prev.ipa.as_deref(), next.ipa.as_deref()) else {
            continue;
        };
        match joint {
            Joint::Link {
                rule,
                consonant,
                moved,
                glide,
            } => {
                let continued = open
                    .as_ref()
                    .is_some_and(|melt| melt.rule == rule && melt.words.end == at + 1);
                if !continued {
                    close(&mut open, found);
                    open = Some(Melt {
                        rule,
                        words: at..at + 1,
                        span: 0..0,
                        sound: prev.lower.clone(),
                        ipa: prev_ipa.to_owned(),
                    });
                }
                let melt = open.as_mut().expect("a melt is open");
                // A silent h isn't written once the sound runs into it:
                // *to be honest* → *be-yonest*.
                let next_letters = next.lower.strip_prefix('h').unwrap_or(&next.lower);
                melt.sound = match glide {
                    Some(glide) => format!("{}-{glide}{next_letters}", melt.sound),
                    None => link(&melt.sound, consonant, moved, next_letters),
                };
                melt.ipa = match (rule, glide) {
                    ("flap-t", _) => format!("{}ɾ‿{next_ipa}", without_last(&melt.ipa)),
                    ("linking-r", _) => format!("{}r‿{next_ipa}", melt.ipa),
                    (_, Some(glide)) => {
                        // The glide starts the next syllable, after its stress mark.
                        let glide = if glide == "y" { "j" } else { "w" };
                        let (stress, rest) = match next_ipa.chars().next() {
                            Some(mark @ ('ˈ' | 'ˌ')) => {
                                (mark.to_string(), &next_ipa[mark.len_utf8()..])
                            }
                            _ => (String::new(), next_ipa),
                        };
                        format!("{}‿{stress}{glide}{rest}", melt.ipa)
                    }
                    _ => format!("{}‿{next_ipa}", melt.ipa),
                };
                melt.words.end = at + 2;
            }
            other => {
                close(&mut open, found);
                if let Some(mut melt) = joined(prev, next, prev_ipa, next_ipa, other, accent) {
                    melt.words = at..at + 2;
                    if matches!(melt.rule, "them" | "h-dropping" | "yod") {
                        dropped.push(at + 1);
                    }
                    found.push(melt);
                }
            }
        }
    }
    close(&mut open, found);
    dropped
}

/// A joint's melt, other than a link.
fn joined(
    prev: &Word,
    next: &Word,
    prev_ipa: &str,
    next_ipa: &str,
    joint: Joint,
    accent: Accent,
) -> Option<Melt> {
    let last = final_consonant(prev_ipa);
    // Its words are the joint's, set by the caller.
    let melt = |rule: &'static str, sound: String, ipa: String| Melt {
        rule,
        words: 0..2,
        span: 0..0,
        sound,
        ipa,
    };
    match joint {
        Joint::Yod => {
            let last = last?;
            let rest = without_final(&prev.lower, last)?;
            let (letters, sound) = match last {
                "t" => ("tch", "tʃ"),
                "d" => ("dj", "dʒ"),
                "s" => ("sh", "ʃ"),
                _ => ("zh", "ʒ"),
            };
            let (tail, tail_ipa) = match (next.lower.as_str(), last) {
                ("you", "t" | "d") => ("a", "ə".to_owned()),
                ("you", _) => ("oo", "uː".to_owned()),
                _ => (
                    "er",
                    if accent == Accent::Us { "ɚ" } else { "ə" }.to_owned(),
                ),
            };
            Some(melt(
                "yod",
                format!("{rest}{letters}{tail}"),
                format!("{}{sound}{tail_ipa}", without_last(prev_ipa)),
            ))
        }
        Joint::Dropped { rule } => {
            let (shrunk, shrunk_ipa) = match next.lower.as_str() {
                "them" => ("em", "əm"),
                "he" => ("ee", "i"),
                "him" => ("im", "ɪm"),
                "her" => ("er", if accent == Accent::Us { "ɚ" } else { "ə" }),
                "his" => ("iz", "ɪz"),
                _ => ("uv", "əv"),
            };
            let sound = match last {
                Some(consonant) => link(&prev.lower, consonant, written(consonant), shrunk),
                None => format!("{}-{shrunk}", prev.lower),
            };
            Some(melt(rule, sound, format!("{prev_ipa}‿{shrunk_ipa}")))
        }
        Joint::Elided => {
            let short = if prev.lower.ends_with("ed") && prev.lower.len() > 3 {
                format!("{}'", &prev.lower[..prev.lower.len() - 2])
            } else if prev.lower.ends_with('t') || prev.lower.ends_with('d') {
                prev.lower[..prev.lower.len() - 1].to_owned()
            } else {
                return None;
            };
            Some(melt(
                "elision",
                format!("{short} {}", next.lower),
                format!("{} {next_ipa}", without_last(prev_ipa)),
            ))
        }
        Joint::CutOff => cut_off_t(prev, Some((next, next_ipa))),
        Joint::Assimilated { into, letter } => {
            let changed = prev.lower.strip_suffix(['n', 't', 'd'])?;
            Some(melt(
                "assimilation",
                format!("{changed}{letter} {}", next.lower),
                format!("{}{into} {next_ipa}", without_last(prev_ipa)),
            ))
        }
        Joint::Link { .. } => None,
    }
}

// ── one word ────────────────────────────────────────────────────────────

/// A British t cut off with a catch in the throat: before a consonant
/// (`next`), or before a pause.
fn cut_off_t(word: &Word, next: Option<(&Word, &str)>) -> Option<Melt> {
    let ipa = word.ipa.as_deref()?;
    if final_consonant(ipa) != Some("t") || !sound_before_last(ipa).is_some_and(is_vowel) {
        return None;
    }
    let letters = word
        .lower
        .strip_suffix("tt")
        .or_else(|| word.lower.strip_suffix('t'))?;
    let (sound, glottal) = (format!("{letters}'"), format!("{}ʔ", without_last(ipa)));
    let (sound, ipa) = match next {
        Some((next, next_ipa)) => (
            format!("{sound} {}", next.lower),
            format!("{glottal} {next_ipa}"),
        ),
        None => (sound, glottal),
    };
    Some(Melt {
        rule: "glottal-t",
        words: 0..1,
        span: 0..0,
        sound,
        ipa,
    })
}

/// Whether the word's -ing is said -in': one of two syllables or more.
fn says_in(word: &Word) -> bool {
    word.ipa.as_deref().is_some_and(|ipa| {
        word.lower.ends_with("ing") && ipa.ends_with("ɪŋ") && syllables(ipa) >= 2
    })
}

/// -ing said -in'.
fn g_dropping(word: &Word) -> Option<Melt> {
    if !says_in(word) {
        return None;
    }
    let ipa = word.ipa.as_deref()?;
    Some(Melt {
        rule: "g-dropping",
        words: 0..1,
        span: 0..0,
        sound: format!("{}'", &word.lower[..word.lower.len() - 1]),
        ipa: format!("{}n", &ipa[..ipa.len() - "ŋ".len()]),
    })
}

/// An American t between two vowel sounds inside a word, the second
/// unstressed: *water* → *wadder*.
fn soft_t_inside(word: &Word) -> Option<Melt> {
    let ipa = word.ipa.as_deref()?;
    let sounds: Vec<char> = ipa.chars().collect();
    let at = (1..sounds.len().saturating_sub(1)).find(|&at| {
        let before = sounds[..at].iter().rev().find(|c| !matches!(c, 'ː'));
        // Before /ən/ the t is held, not softened: *button*, *important*.
        let held = sounds[at + 1] == 'ə'
            && sounds.get(at + 2) == Some(&'n')
            && sounds.get(at + 3).is_none_or(|c| !is_vowel(*c));
        sounds[at] == 't'
            && !matches!(sounds[at - 1], 'ˈ' | 'ˌ')
            && before.is_some_and(|c| is_vowel(*c) || *c == 'r')
            && is_vowel(sounds[at + 1])
            && !held
    })?;
    let letters = if word.lower.contains("tt") {
        word.lower.replacen("tt", "dd", 1)
    } else {
        let chars: Vec<char> = word.lower.chars().collect();
        let t = (1..chars.len().saturating_sub(1))
            .find(|&at| chars[at] == 't' && "aeiouylr".contains(chars[at + 1]))?;
        chars
            .iter()
            .enumerate()
            .map(|(at, c)| if at == t { 'd' } else { *c })
            .collect()
    };
    let mut flapped = sounds.clone();
    flapped[at] = 'ɾ';
    Some(Melt {
        rule: "flap-t",
        words: 0..1,
        span: 0..0,
        sound: letters,
        ipa: flapped.into_iter().collect(),
    })
}

/// A small word said lightly: *for* → *fer*. *do* and *does* only as
/// helpers before a pronoun, *have* only after a modal.
fn weak_form(words: &[Word], at: usize, accent: Accent) -> Option<Melt> {
    let word = &words[at];
    let form = connected_speech()
        .weak_forms
        .iter()
        .find(|form| form.word == word.lower)?;
    const PRONOUNS: [&str; 7] = ["i", "you", "he", "she", "it", "we", "they"];
    let next = words.get(at + 1).map(|next| next.lower.as_str());
    let prev = at.checked_sub(1).map(|prev| words[prev].lower.as_str());
    let helper = match word.lower.as_str() {
        "do" | "does" => next.is_some_and(|next| PRONOUNS.contains(&next)),
        // A helper only after a modal (*could have*): *do you have a bed*
        // has it whole.
        "have" => prev.is_some_and(|prev| MODALS.contains(&prev)),
        _ => true,
    };
    helper.then(|| Melt {
        rule: "weak-form",
        words: 0..1,
        span: 0..0,
        sound: form.sound.clone(),
        ipa: form.ipa.get(accent).unwrap_or_default().to_owned(),
    })
}
