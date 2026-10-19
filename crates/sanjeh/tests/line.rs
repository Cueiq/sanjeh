//! The line measure on lines: what can't be practised and why, what makes a
//! line hard, and a model's weights at work.

use sanjeh::connected::line_words;
use sanjeh::line::{LineInput, LineModel, Reason, Skill, WhyNot, facts, measure};

fn line(text: &str, ms: u32) -> LineInput {
    serde_json::from_value(serde_json::json!({ "text": text, "duration_ms": ms })).unwrap()
}

pub fn split_voices(input: &LineInput) -> Vec<LineInput> {
    if input.voices < 2 {
        return vec![input.clone()];
    }

    let text = input.text.trim();
    let parts: Vec<&str> = text
        .split(['-', '—'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let parts_len = parts.len();
    if parts_len < 2 {
        return vec![input.clone()];
    }

    let mut outputs = vec![];

    for part_text in parts {
        let part_words = line_words(part_text);
        if part_words.is_empty() {
            continue;
        }

        let part_input = LineInput {
            text: part_text.to_string(),
            duration_ms: input.duration_ms / parts_len as u32,
            accent: input.accent,
            voices: 1,
            heard: input.heard.clone(),
            entries: input.entries.clone(),
            names: input.names.clone(),
        };

        outputs.push(part_input);
    }

    if outputs.len() != parts_len {
        return vec![input.clone()];
    }

    outputs
}

#[test]
fn a_cut_off_line_or_two_voices_cant_be_practised() {
    let cut = facts(&line("This is the best thing I've ever put in my mouth...", 3000));
    assert!(cut.why_not(Skill::Ear).contains(&WhyNot::NotASentence));
    let mut two = line("What are you doing? What?", 2000);
    two.voices = 2;
    assert!(facts(&two).why_not(Skill::Mouth).contains(&WhyNot::TwoVoices));
    let fine = facts(&line("There are no bad situations as long as I'm with you.", 4700));
    assert!(fine.why_not(Skill::Ear).is_empty() && fine.why_not(Skill::Mouth).is_empty());
}

#[test]
fn words_said_wrong_on_purpose_have_no_answer_to_hold_to() {
    let f = facts(&line("You are weawy getting on my nerves.", 2500));
    assert!(matches!(&f.why_not(Skill::Ear)[..], [WhyNot::SaidWrong { words }] if words == &["weawy"]));
}

#[test]
fn a_pair_written_as_said_and_its_melts_are_reasons() {
    let f = facts(&line("Hey, guys, I'm gonna throw something out there.", 2400));
    let reasons = f.reasons(Skill::Ear);
    assert!(reasons.contains(&Reason::Written {
        word: "gonna".into(),
        full: "going to".into()
    }));
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, Reason::Melt { rule, .. } if rule == "g-dropping"))
    );
    assert_eq!(f.features["words"], 8.0);
}

#[test]
fn a_name_mid_sentence_is_not_a_hard_word() {
    let f = facts(&line("I think I'm ready to pull the trigger on Batman.", 3000));
    assert_eq!(f.names, ["batman"]);
    assert!(f.hard_words.iter().all(|(w, _)| w != "batman"));
}

#[test]
fn a_model_weighs_the_features_it_names() {
    let model: LineModel = serde_json::from_value(serde_json::json!({
        "name": "t", "about": "",
        "ear": { "level": { "features": ["words"], "mean": [0.0], "sd": [1.0], "weights": [0.25], "bias": 1.0 },
                 "worth": { "features": [], "mean": [], "sd": [], "weights": [], "bias": 0.0 } },
        "mouth": { "level": { "features": [], "mean": [], "sd": [], "weights": [], "bias": 3.0 },
                   "worth": { "features": [], "mean": [], "sd": [], "weights": [], "bias": 0.0 } }
    }))
    .unwrap();
    let m = measure(&line("Do you have an identical twin brother?", 1800), &model);
    assert_eq!(m.ear.level, 1.0 + 0.25 * 7.0);
    assert_eq!(m.mouth.level, 3.0);
    assert_eq!(m.ear.worth, 0.5);
}

#[test]
fn the_default_model_measures_a_line() {
    let m = measure(
        &line("There are no bad situations as long as I'm with you.", 4700),
        sanjeh::line::default_model(),
    );
    assert!((1.0..7.0).contains(&m.ear.level) && (0.0..=1.0).contains(&m.mouth.worth));
}

#[test]
fn a_word_read_two_ways_still_has_its_hard_sounds() {
    assert!(!sanjeh::readings("with", sanjeh::Accent::Us).is_empty());
    let f = facts(&line("There are no bad situations as long as I'm with you.", 4700));
    assert!(
        f.hard_sounds
            .iter()
            .any(|(sound, word)| sound == "th" && word == "with")
    );
    assert!(f.hard_sounds.iter().any(|(sound, word)| sound == "w" && word == "with"));
}

#[test]
fn test_split_voices_basic() {
    let mut cue = line("What are you doing? - What?", 2000);
    cue.voices = 2;
    let parts = split_voices(&cue);
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].text, "What are you doing?");
    assert_eq!(parts[0].voices, 1);
    assert_eq!(parts[1].text, "What?");
    assert_eq!(parts[1].voices, 1);
}

#[test]
fn test_split_voices_single_voice_untouched() {
    let cue = line("There are no bad situations.", 3000);
    let parts = split_voices(&cue);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].text, "There are no bad situations.");
}

#[test]
fn test_split_voices_fallback_on_bad_split() {
    let mut cue = line("Just a single line without dash", 2500);
    cue.voices = 2;
    let parts = split_voices(&cue);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].text, "Just a single line without dash");
}
