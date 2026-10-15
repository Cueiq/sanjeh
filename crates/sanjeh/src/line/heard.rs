//! The weak ear: a line heard by speech recognisers of three strengths
//! (whisper's tiny, base and small English models), each one's words lined
//! up with the line's. Learners' transcription mistakes rise where speech
//! recognition errs (Mirzaei, Meshgi and Kawahara, 2016–2018): a word only
//! the weakest model missed is a mild sign, one the strongest missed a
//! strong one — or a sign the audio, not the ear, is at fault.

use serde::{Deserialize, Serialize};

/// What the three recognisers heard of a line.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Heard {
    pub tiny: Vec<HeardWord>,
    pub base: Vec<HeardWord>,
    pub small: Vec<HeardWord>,
}

/// A word heard, and how sure the recogniser was of it (its least sure
/// token), where it says.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeardWord {
    pub word: String,
    #[serde(default)]
    pub p: Option<f64>,
}

/// What a line's words came to with the three ears.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Hearing {
    /// Each of the line's words (`words_of`), whether each model heard it.
    pub words: Vec<WordHeard>,
    /// Missed words, weighed by kind (a word 1, a name ½, a filler 0), per
    /// word of the line that isn't a filler.
    pub miss_tiny: f64,
    pub miss_base: f64,
    pub miss_small: f64,
    /// Words the strongest model heard but was less than half sure of, per word.
    pub unsure: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordHeard {
    pub word: String,
    pub kind: WordKind,
    pub tiny: bool,
    pub base: bool,
    pub small: bool,
    pub p: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WordKind {
    Word,
    Name,
    /// *um*, *oh*, *you know*: whisper leaves them out on purpose, so missing
    /// one says nothing of the line.
    Filler,
}

const FILLERS: &[&str] = &["um", "uh", "er", "erm", "oh", "ah", "hmm", "mm", "like", "well", "so"];

/// Forms written two ways: either matches the other.
const SAME: &[(&str, &str)] = &[
    ("ok", "okay"),
    ("alright", "all right"),
    ("gonna", "going to"),
    ("wanna", "want to"),
    ("gotta", "got to"),
    ("kinda", "kind of"),
    ("sorta", "sort of"),
    ("outta", "out of"),
    ("lotta", "lot of"),
    ("dunno", "don't know"),
    ("ya", "you"),
    ("mr", "mister"),
    ("mrs", "missus"),
    ("til", "until"),
];

/// A word as it is compared: lower case, one apostrophe, no hyphens or
/// slashes, its common other spelling — and a run-together form its words.
pub fn normal(word: &str) -> Vec<String> {
    let word = word.to_lowercase().replace(['’', '‘'], "'");
    let word = word.trim_matches('\'');
    let word = SAME
        .iter()
        .find(|(form, _)| *form == word)
        .map_or(word, |(_, full)| full);
    word.replace(['-', '/'], "")
        .split(' ')
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// The words of a text, compared as [`normal`] makes them, each with its
/// place among the text's own words (`tokens`), which run-together forms
/// share.
pub fn words_of(tokens: &[String]) -> Vec<(String, usize)> {
    tokens
        .iter()
        .enumerate()
        .flat_map(|(at, token)| normal(token).into_iter().map(move |w| (w, at)))
        .collect()
}

fn close(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    // Spellings a letter apart in a longer word: colour, color.
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len() < 5 || b.len() < 5 {
        return false;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            cur.push((prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + usize::from(ca != cb)));
        }
        prev = cur;
    }
    prev[b.len()] <= 1
}

/// For each target word, whether the hypothesis has it in its place (word
/// edit distance). Words heard before or after the line — a neighbour's,
/// caught in a clip's margin — cost nothing.
pub fn heard_which(target: &[String], heard: &[String]) -> Vec<bool> {
    let (n, m) = (target.len(), heard.len());
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    let cost = |i: usize, j: usize| usize::from(!close(&target[i - 1], &heard[j - 1]));
    for i in 1..=n {
        for j in 1..=m {
            d[i][j] = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + cost(i, j));
        }
    }
    let mut j = (0..=m).min_by_key(|&j| d[n][j]).unwrap_or(0);
    let (mut i, mut got) = (n, vec![false; n]);
    while i > 0 && j > 0 {
        if d[i][j] == d[i - 1][j - 1] + cost(i, j) {
            got[i - 1] = cost(i, j) == 0;
            i -= 1;
            j -= 1;
        } else if d[i][j] == d[i - 1][j] + 1 {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    got
}

/// The line's words with the three ears. `tokens` are the line's own words;
/// `names` says which of them are names.
pub fn hearing(tokens: &[String], is_name: impl Fn(usize) -> bool, heard: &Heard) -> Hearing {
    let target = words_of(tokens);
    let words: Vec<String> = target.iter().map(|(w, _)| w.clone()).collect();
    let kinds: Vec<WordKind> = target
        .iter()
        .enumerate()
        .map(|(i, (w, at))| {
            let next = target.get(i + 1).map(|(w, _)| w.as_str());
            let prev = i.checked_sub(1).map(|p| target[p].0.as_str());
            if FILLERS.contains(&w.as_str())
                || (w == "you" && next == Some("know"))
                || (w == "know" && prev == Some("you"))
            {
                WordKind::Filler
            } else if is_name(*at) {
                WordKind::Name
            } else {
                WordKind::Word
            }
        })
        .collect();
    let model_words = |heard: &[HeardWord]| -> Vec<(String, Option<f64>)> {
        heard
            .iter()
            .flat_map(|h| normal(&h.word).into_iter().map(move |w| (w, h.p)))
            .collect()
    };
    let tiny = model_words(&heard.tiny);
    let base = model_words(&heard.base);
    let small = model_words(&heard.small);
    let plain = |ws: &[(String, Option<f64>)]| ws.iter().map(|(w, _)| w.clone()).collect::<Vec<_>>();
    let (got_tiny, got_base, got_small) = (
        heard_which(&words, &plain(&tiny)),
        heard_which(&words, &plain(&base)),
        heard_which(&words, &plain(&small)),
    );

    // How sure small was of each word it heard, in order.
    let mut p_small = vec![None; words.len()];
    let mut at = 0;
    for (k, got) in got_small.iter().enumerate() {
        if *got {
            while at < small.len() && !close(&words[k], &small[at].0) {
                at += 1;
            }
            if at < small.len() {
                p_small[k] = small[at].1;
                at += 1;
            }
        }
    }

    let weigh = |kind: WordKind| match kind {
        WordKind::Word => 1.0,
        WordKind::Name => 0.5,
        WordKind::Filler => 0.0,
    };
    let counted = kinds.iter().filter(|k| **k != WordKind::Filler).count().max(1) as f64;
    let miss = |got: &[bool]| {
        got.iter()
            .zip(&kinds)
            .filter(|(g, _)| !**g)
            .map(|(_, k)| weigh(*k))
            .sum::<f64>()
            / counted
    };
    let unsure = (0..words.len())
        .filter(|&k| kinds[k] == WordKind::Word && got_small[k] && p_small[k].is_some_and(|p| p < 0.5))
        .count() as f64
        / counted;
    Hearing {
        miss_tiny: miss(&got_tiny),
        miss_base: miss(&got_base),
        miss_small: miss(&got_small),
        unsure,
        words: (0..words.len())
            .map(|k| WordHeard {
                word: words[k].clone(),
                kind: kinds[k],
                tiny: got_tiny[k],
                base: got_base[k],
                small: got_small[k],
                p: p_small[k],
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heard(text: &str) -> Vec<HeardWord> {
        text.split_whitespace()
            .map(|w| HeardWord {
                word: w.to_string(),
                p: Some(0.9),
            })
            .collect()
    }

    fn toks(text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'').to_string())
            .collect()
    }

    #[test]
    fn a_word_missed_only_by_the_weakest_ear_weighs_least() {
        let line = toks("I'm having second thoughts.");
        let h = hearing(
            &line,
            |_| false,
            &Heard {
                tiny: heard("I have in a second thoughts"),
                base: heard("I'm having second thoughts"),
                small: heard("I'm having second thoughts"),
            },
        );
        assert_eq!((h.miss_tiny, h.miss_base, h.miss_small), (0.5, 0.0, 0.0));
    }

    #[test]
    fn a_neighbours_words_in_the_margin_cost_nothing() {
        let got = heard_which(
            &["want".into(), "me".into(), "to".into(), "go".into()],
            &[
                "yeah".into(),
                "want".into(),
                "me".into(),
                "to".into(),
                "go".into(),
                "sure".into(),
            ],
        );
        assert_eq!(got, [true, true, true, true]);
    }

    #[test]
    fn written_together_matches_said_apart_and_fillers_cost_nothing() {
        let line = toks("Anyway, we, you know, kinda comforted each other.");
        let h = hearing(
            &line,
            |_| false,
            &Heard {
                tiny: heard("anyway we kind of comforted each other"),
                base: heard("anyway we kind of comforted each other"),
                small: heard("anyway we kind of comforted each other"),
            },
        );
        assert_eq!(h.miss_tiny, 0.0);
    }
}
