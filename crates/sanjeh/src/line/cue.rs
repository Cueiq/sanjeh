#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedCue {
    pub text: String,
    pub voices: u8,
}

/// Converts a raw subtitle cue into the text that should be measured.
pub fn prepare_cue(text: &str) -> PreparedCue {
    let text = strip_markup(text);
    let text = strip_sound_annotations(&text);

    let (text, named_voices) = strip_speaker_labels(&text);
    let (text, dialogue_markers) = strip_dialogue_dashes(&text);

    let text = normalize_whitespace(&text);

    PreparedCue {
        text,
        voices: if named_voices >= 2 || dialogue_markers >= 2 {
            2
        } else {
            1
        },
    }
}

fn strip_markup(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut inside_tag = false;

    for ch in text.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' if inside_tag => inside_tag = false,
            _ if !inside_tag => result.push(ch),
            _ => {}
        }
    }

    result
}

fn strip_sound_annotations(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    while i < text.len() {
        let ch = text[i..].chars().next().expect("valid character boundary");

        let closing = match ch {
            '[' => Some(']'),
            '(' => Some(')'),
            _ => None,
        };

        if let Some(closing) = closing {
            if let Some(end) = text[i + ch.len_utf8()..].find(closing) {
                let content_start = i + ch.len_utf8();
                let content_end = content_start + end;
                let annotation = &text[content_start..content_end];

                if is_sound_annotation(annotation) {
                    i = content_end + closing.len_utf8();
                    continue;
                }
            }
        }

        if ch != '♪' {
            result.push(ch);
        }

        i += ch.len_utf8();
    }

    result
}

fn is_sound_annotation(annotation: &str) -> bool {
    let normalized = annotation
        .trim()
        .trim_matches(|ch: char| ch == '.' || ch == '!' || ch == '?')
        .to_ascii_lowercase();

    matches!(
        normalized.as_str(),
        "music"
            | "♪"
            | "laugh"
            | "laughs"
            | "laughing"
            | "laughter"
            | "sigh"
            | "sighs"
            | "gasps"
            | "gasping"
            | "crying"
            | "cries"
            | "sobbing"
            | "applause"
            | "silence"
            | "inaudible"
            | "indistinct"
            | "door slams"
            | "phone ringing"
    )
}

fn strip_speaker_labels(text: &str) -> (String, usize) {
    let mut ranges = Vec::new();
    let mut segment_start = 0;
    let mut voices = std::collections::HashSet::new();

    for (index, ch) in text.char_indices() {
        if ch == ':' {
            let segment = &text[segment_start..index];

            if is_speaker_label(segment) {
                let leading_whitespace = segment.len() - segment.trim_start().len();
                ranges.push((segment_start + leading_whitespace, index + 1));
                voices.insert(segment.trim().to_ascii_lowercase());
            }

            segment_start = index + ch.len_utf8();
        } else if matches!(ch, '.' | '?' | '!' | '\n' | '\r' | '—' | '–') {
            segment_start = index + ch.len_utf8();
        }
    }

    let mut result = text.to_owned();

    for (start, end) in ranges.into_iter().rev() {
        result.replace_range(start..end, "");
    }

    (result, voices.len())
}

fn is_speaker_label(label: &str) -> bool {
    let label = label.trim();

    !label.is_empty()
        && label.chars().any(|ch| ch.is_ascii_alphabetic())
        && label.chars().all(|ch| {
            ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch.is_ascii_whitespace() || ch == '\'' || ch == '-'
        })
}

fn strip_dialogue_dashes(text: &str) -> (String, usize) {
    let mut ranges = Vec::new();
    let mut markers = 0;

    for (index, ch) in text.char_indices() {
        if !matches!(ch, '-' | '–' | '—') {
            continue;
        }

        let end = index + ch.len_utf8();

        let previous = text[..index].chars().next_back();
        let next = text[end..].chars().next();

        let at_boundary = previous.is_none_or(char::is_whitespace);
        let followed_by_space = next.is_none_or(char::is_whitespace);

        if at_boundary && followed_by_space {
            ranges.push((index, end));
            markers += 1;
        }
    }

    let mut result = text.to_owned();

    for (start, end) in ranges.into_iter().rev() {
        result.replace_range(start..end, "");
    }

    (result, markers)
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{prepare_cue, strip_sound_annotations};

    #[test]
    fn strips_markup_and_sound_annotations() {
        let cue = "<i>Hello there.</i> [music] (laughs) ♪";

        let prepared = prepare_cue(cue);

        assert_eq!(prepared.text, "Hello there.");
        assert_eq!(prepared.voices, 1);
    }

    #[test]
    fn repeated_speaker_label_does_not_mean_two_voices() {
        let prepared = prepare_cue("STUART: Hello. STUART: What now?");

        assert_eq!(prepared.text, "Hello. What now?");
        assert_eq!(prepared.voices, 1);
    }

    #[test]
    fn preserves_words_inside_ordinary_parentheses() {
        let prepared = prepare_cue("I (really) mean it.");

        assert_eq!(prepared.text, "I (really) mean it.");
    }

    #[test]
    fn strips_named_speakers_and_counts_voices() {
        let prepared = prepare_cue("STUART: Hello. CHUCK: Hey.");

        assert_eq!(prepared.text, "Hello. Hey.");
        assert_eq!(prepared.voices, 2);
    }

    #[test]
    fn counts_dialogue_dashes() {
        let prepared = prepare_cue("- What are you doing?\n- What?");

        assert_eq!(prepared.text, "What are you doing? What?");
        assert_eq!(prepared.voices, 2);
    }

    #[test]
    fn leaves_single_speaker_at_one_voice() {
        let prepared = prepare_cue("STUART: Hello there.");

        assert_eq!(prepared.text, "Hello there.");
        assert_eq!(prepared.voices, 1);
    }

    #[test]
    fn preserves_non_annotation_parentheses() {
        let text = strip_sound_annotations("I (really) mean it.");

        assert_eq!(text, "I (really) mean it.");
    }

    #[test]
    fn one_dialogue_dash_does_not_mean_two_voices() {
        let prepared = prepare_cue("- What?");

        assert_eq!(prepared.text, "What?");
        assert_eq!(prepared.voices, 1);
    }
}
