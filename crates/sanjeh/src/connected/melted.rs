//! A line as it is said: its melts drawn together into the stretches the
//! learner reads — *thi-sis thuh story-yuv two girls who sha-ruh fla-tin
//! London*. Melts over the same word are one stretch: a weak *of* inside the
//! link of *story of*, the dropped d of *and shine* after the link of *rise
//! and*.

use std::ops::Range;

use super::Melt;
use super::melts::line_words;

/// One stretch of a line where words melt, as it sounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    /// Where they are in the line, in UTF-16 units, as JavaScript counts.
    pub span: Range<usize>,
    /// How they sound, in plain letters: `sha-ruh`.
    pub said: String,
    /// The melts made in it, by their place in the line's melts.
    pub melts: Vec<usize>,
    /// Nothing but a small word said lightly: drawn small and grey.
    pub weak: bool,
}

/// The line's melts (`melts`, of the same line) as stretches, in the
/// order of the line.
pub fn melted(line: &str, melts: &[Melt]) -> Vec<Piece> {
    let words: Vec<String> = line_words(line)
        .into_iter()
        .map(|bytes| line[bytes].to_lowercase().replace('’', "'"))
        .collect();
    let mut pieces = Vec::new();
    let mut at = 0;
    while at < melts.len() {
        let mut end = melts[at].words.end;
        let mut next = at + 1;
        while next < melts.len() && melts[next].words.start < end {
            end = end.max(melts[next].words.end);
            next += 1;
        }
        pieces.push(piece(line, &words, melts, (at..next).collect()));
        at = next;
    }
    pieces
}

/// Melts over the same words, as one stretch: the joints chained in their
/// order, and a word's own melt (a weak form, -in') at the stretch's end
/// written into it. One the letters can't show stays among its melts,
/// without being written.
fn piece(line: &str, words: &[String], melts: &[Melt], group: Vec<usize>) -> Piece {
    let from = melts[group[0]].span.start;
    let to = group
        .iter()
        .map(|&at| melts[at].span.end)
        .max()
        .unwrap_or(from);
    let (joints, own): (Vec<usize>, Vec<usize>) =
        group.iter().partition(|&&at| melts[at].words.len() >= 2);

    let said = match joints.split_first() {
        None => melts[group[0]].sound.clone(),
        Some((&first, rest)) => {
            let mut said = melts[first].sound.clone();
            let mut last = melts[first].words.end - 1;
            for &at in rest {
                let melt = &melts[at];
                said = chained(&said, &words[melt.words.start], &melt.sound).unwrap_or_else(|| {
                    // Written as it is, past the word they share.
                    let after = &words[melt.words.start + 1..melt.words.end];
                    format!("{said} {}", after.join(" "))
                });
                last = last.max(melt.words.end - 1);
            }
            for &at in &own {
                let melt = &melts[at];
                if melt.words.start == last
                    && let Some(written) = chained(&said, &words[last], &melt.sound)
                {
                    said = written;
                }
            }
            said
        }
    };
    Piece {
        span: from..to,
        said: as_in_line(&said, line, from),
        weak: group.iter().all(|&at| melts[at].rule == "weak-form"),
        melts: group,
    }
}

/// `said`, which ends in `word` as some melt wrote it, followed by `next`,
/// a melt that starts at that word: the end of the word in `said` gives way
/// to `next`, which writes the word again its own way. None where `said`
/// doesn't end in the word's letters, or `next` doesn't start with the
/// letters `said` changed.
fn chained(said: &str, word: &str, next: &str) -> Option<String> {
    let said_chars: Vec<char> = said.chars().collect();
    let word_chars: Vec<char> = word.chars().collect();
    let shared = said_chars
        .iter()
        .rev()
        .zip(word_chars.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    if shared == 0 {
        return None;
    }
    let changed: String = word_chars[..word_chars.len() - shared].iter().collect();
    let rest = next.strip_prefix(changed.as_str())?;
    let kept: String = said_chars[..said_chars.len() - shared].iter().collect();
    Some(format!("{kept}{rest}"))
}

/// The melts write lower case: a stretch that starts the line, or a name,
/// starts with its capital again, and *I* is written *I*.
fn as_in_line(said: &str, line: &str, from: usize) -> String {
    let first_upper = char::decode_utf16(line.encode_utf16().skip(from))
        .next()
        .and_then(Result::ok)
        .is_some_and(char::is_uppercase);
    let mut written = String::with_capacity(said.len());
    let mut token_start = true;
    let chars: Vec<char> = said.chars().collect();
    for (at, &c) in chars.iter().enumerate() {
        let alone = token_start
            && chars
                .get(at + 1)
                .is_none_or(|next| matches!(next, ' ' | '-' | '\''));
        if (at == 0 && first_upper) || (c == 'i' && alone) {
            written.extend(c.to_uppercase());
        } else {
            written.push(c);
        }
        token_start = matches!(c, ' ' | '-');
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Accent;
    use crate::connected::melts;

    fn said(line: &str, accent: Accent) -> Vec<String> {
        melted(line, &melts(line, accent))
            .into_iter()
            .map(|piece| piece.said)
            .collect()
    }

    #[test]
    fn a_weak_word_inside_a_link_is_written_weak() {
        assert_eq!(
            said(
                "This is the story of two girls who share a flat in London.",
                Accent::Gb
            ),
            ["Thi-sis", "thuh", "story-yuv", "sha-ruh", "fla-tin"]
        );
        assert_eq!(
            said("Oh, you have a cold, Bridget.", Accent::Gb),
            ["ya", "ha-vuh"]
        );
    }

    #[test]
    fn two_joints_over_one_word_run_on() {
        assert_eq!(
            said("Come on. Rise and shine.", Accent::Gb),
            ["Co-mon", "Ri-zan shine"]
        );
        assert_eq!(
            said("I have this car at home.", Accent::Gb),
            ["ca-ra' home"]
        );
    }

    #[test]
    fn a_word_s_own_melt_at_the_end_is_written_in() {
        assert_eq!(said("Good morning.", Accent::Gb), ["Goob mornin'"]);
    }

    #[test]
    fn i_is_a_capital_and_so_is_the_line_s_start() {
        assert_eq!(as_in_line("i-yam", "I am Nick.", 0), "I-yam");
        assert_eq!(as_in_line("so-wi", "and so I", 4), "so-wi");
        assert_eq!(as_in_line("an i", "and I", 0), "an I");
        assert_eq!(as_in_line("an i'm", "and I'm", 0), "an I'm");
        assert_eq!(as_in_line("i'd go", "I'd go", 0), "I'd go");
        assert_eq!(
            said("I am Nick. I live next door.", Accent::Gb),
            ["nex door"]
        );
    }

    #[test]
    fn a_stretch_names_its_melts_and_a_lone_weak_form_is_weak() {
        let line = "Oh, you have a cold, Bridget.";
        let found = melts(line, Accent::Gb);
        let pieces = melted(line, &found);
        assert!(pieces[0].weak);
        assert!(!pieces[1].weak);
        let rules: Vec<&str> = pieces[1].melts.iter().map(|&at| found[at].rule).collect();
        assert_eq!(rules, ["linking", "weak-form"]);
        let span = &pieces[1].span;
        let text: String =
            char::decode_utf16(line.encode_utf16().skip(span.start).take(span.len()))
                .map(Result::unwrap)
                .collect();
        assert_eq!(text, "have a");
    }
}
