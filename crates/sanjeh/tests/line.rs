//! The line measure on lines: what can't be practised and why, what makes a
//! line hard, and a model's weights at work.

use sanjeh::line::{Heard, HeardWord, LineInput, LineModel, Reason, Skill, WhyNot, facts, measure};

fn line(text: &str, ms: u32) -> LineInput {
    serde_json::from_value(serde_json::json!({ "text": text, "duration_ms": ms })).unwrap()
}

fn heard_word(word: &str, start_ms: Option<u32>, end_ms: Option<u32>) -> HeardWord {
    HeardWord {
        word: word.to_string(),
        p: Some(0.9),
        start_ms,
        end_ms,
    }
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
fn a_real_slang_word_in_subtlex_is_not_said_wrong() {
    let f = facts(&line("That was a wanky idea.", 3000));

    assert!(
        !f.said_wrong.iter().any(|word| word == "wanky"),
        "a real word should not be marked as said wrong: {:?}",
        f.said_wrong
    );

    assert!(
        !f.names.iter().any(|word| word == "wanky"),
        "a real word should not be treated as a name: {:?}",
        f.names
    );
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
fn leading_apostrophe_fusions_are_written_reasons_in_line_analysis() {
    let cases = [
        ("I did it 'cause I could.", "'cause", "because"),
        ("Tell 'em what happened.", "'em", "them"),
        ("'Cause I said so.", "'cause", "because"),
    ];

    for (text, short, full) in cases {
        let f = facts(&line(text, 3000));
        let reasons = f.reasons(Skill::Ear);

        assert!(
            reasons.contains(&Reason::Written {
                word: short.into(),
                full: full.into(),
            }),
            "{text}: expected Written reason for {short}"
        );
    }
}

#[test]
fn spoken_fusions_use_full_forms_for_word_levels() {
    let cases = [
        ("I did it 'cause I could.", "I did it because I could."),
        ("Tell 'em what happened.", "Tell them what happened."),
        ("I kinda like it.", "I kind of like it."),
        ("I dunno what happened.", "I don't know what happened."),
        ("They shouted c'mon, let's go.", "They shouted come on, let's go."),
        ("I cannot do that.", "I can not do that."),
        ("I heard someone say lemme see.", "I heard someone say let me see."),
        ("I heard someone say gimme that.", "I heard someone say give me that."),
        ("I sorta like it.", "I sort of like it."),
    ];

    for (short_form, full_form) in cases {
        let short = facts(&line(short_form, 3000));
        let full = facts(&line(full_form, 3000));

        assert_eq!(
            short.hard_words, full.hard_words,
            "{short_form}: hard words differ from {full_form}"
        );

        assert_eq!(
            short.common, full.common,
            "{short_form}: common-word ratio differs from {full_form}"
        );

        assert_eq!(
            short.features["vocab"], full.features["vocab"],
            "{short_form}: vocabulary level differs from {full_form}"
        );
    }
}

#[test]
fn a_hyphenated_number_is_not_a_hard_word() {
    let f = facts(&line("I paid ninety-nine dollars in nineteen ninety-nine.", 3500));

    assert!(
        f.hard_words.iter().all(|(word, _)| word != "ninety-nine"),
        "number words should not be treated as rare: {:?}",
        f.hard_words
    );
}

#[test]
fn numeric_literals_are_measured_as_spoken_words() {
    let examples = [
        ("I paid 10,000 dollars.", "I paid ten thousand dollars."),
        ("I was born in 1999.", "I was born in nineteen ninety-nine."),
        ("She finished 1st.", "She finished first."),
        ("Meet me at 9:30.", "Meet me at nine thirty."),
    ];

    for (numeric, written_out) in examples {
        let numeric_facts = facts(&line(numeric, 3000));
        let written_facts = facts(&line(written_out, 3000));

        assert_eq!(
            numeric_facts.tokens, written_facts.tokens,
            "tokens differ for {numeric:?}"
        );
        assert_eq!(
            numeric_facts.syllables, written_facts.syllables,
            "syllables differ for {numeric:?}"
        );
        assert_eq!(
            numeric_facts.features["words"], written_facts.features["words"],
            "word counts differ for {numeric:?}"
        );
        assert_eq!(
            numeric_facts.features["hard_words"], written_facts.features["hard_words"],
            "hard word counts differ for {numeric:?}"
        );
        assert_eq!(
            numeric_facts.features["common"], written_facts.features["common"],
            "common word shares differ for {numeric:?}"
        );
        assert_eq!(
            numeric_facts.whole_sentence, written_facts.whole_sentence,
            "sentence validity differs for {numeric:?}"
        );
    }
}

#[test]
fn speech_duration_uses_word_timings_when_available() {
    let mut input = line("I can do it.", 5000);

    input.heard = Some(Heard {
        small: vec![
            heard_word("I", Some(1000), Some(1200)),
            heard_word("can", Some(1200), Some(1600)),
            heard_word("do", Some(1600), Some(2000)),
            heard_word("it", Some(2200), Some(2500)),
        ],
        ..Heard::default()
    });

    let f = facts(&input);

    assert_eq!(f.duration_ms, 1500);
    assert_eq!(f.features["sps"], f.syllables as f64 / 1.5);
}

#[test]
fn speech_duration_falls_back_to_subtitle_duration() {
    let mut input = line("I can do it.", 5000);

    input.heard = Some(Heard {
        small: vec![
            heard_word("I", Some(2500), Some(2600)),
            heard_word("can", Some(2600), Some(2700)),
            heard_word("do", Some(2700), Some(2800)),
            heard_word("it", Some(2800), Some(1000)),
        ],
        ..Heard::default()
    });

    let f = facts(&input);

    assert_eq!(f.duration_ms, 5000);
    assert_eq!(f.features["sps"], f.syllables as f64 / 5.0);
}

#[test]
fn speech_duration_falls_back_when_small_hears_too_few_words() {
    let mut input = line("I can do it.", 5000);

    input.heard = Some(Heard {
        small: vec![heard_word("I", Some(100), Some(200))],
        base: vec![
            heard_word("I", Some(2000), Some(2100)),
            heard_word("can", Some(2100), Some(2400)),
            heard_word("do", Some(2400), Some(3200)),
        ],
        ..Heard::default()
    });

    let f = facts(&input);

    assert_eq!(f.duration_ms, 5000);
}

#[test]
fn speech_duration_falls_back_when_no_model_hears_most_words() {
    let mut input = line("I can do it.", 5000);

    input.heard = Some(Heard {
        small: vec![
            heard_word("I", Some(1000), Some(1200)),
            heard_word("can", Some(1200), Some(1500)),
        ],
        base: vec![heard_word("I", Some(1500), Some(1700))],
        tiny: vec![
            heard_word("I", Some(2000), Some(2200)),
            heard_word("can", Some(2200), Some(2500)),
        ],
    });

    let f = facts(&input);

    assert_eq!(f.duration_ms, 5000);
}

#[test]
fn speech_duration_is_clamped_to_subtitle_duration() {
    let mut input = line("I can do it.", 4000);

    input.heard = Some(Heard {
        small: vec![
            heard_word("I", Some(1000), Some(2000)),
            heard_word("can", Some(2000), Some(3000)),
            heard_word("do", Some(3000), Some(4000)),
            heard_word("it", Some(4000), Some(7000)),
        ],
        ..Heard::default()
    });

    let f = facts(&input);

    assert_eq!(f.duration_ms, 4000);
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
fn known_sentence_opening_words_are_not_names() {
    let examples = ["It's fine.", "That's it.", "You're late.", "Don't go."];

    for text in examples {
        let f = facts(&line(text, 2500));

        assert!(
            f.names.is_empty(),
            "{text:?} should not be treated as a name: {:?}",
            f.names
        );
    }
}

#[test]
fn a_known_word_at_sentence_start_is_still_graded() {
    let capitalized = facts(&line("Congratulations on your success.", 3000));
    let lowercase = facts(&line("congratulations on your success.", 3000));

    assert!(
        !capitalized.names.contains(&"congratulations".to_string()),
        "congratulations should not be treated as a name"
    );

    assert_eq!(
        capitalized.features["vocab"], lowercase.features["vocab"],
        "capitalization should not change vocabulary measurement"
    );

    assert_eq!(
        capitalized.hard_words, lowercase.hard_words,
        "capitalization should not change hard-word detection"
    );
}

#[test]
fn a_sentence_opening_contraction_keeps_its_hard_th_sound() {
    let f = facts(&line("That's it.", 2500));

    assert!(
        f.hard_sounds
            .iter()
            .any(|(sound, word)| sound == "th" && word == "that's"),
        "the th sound in that's should be preserved"
    );
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
fn persian_hard_sounds_are_used_when_language_is_explicit() {
    let mut input = line("There are no bad situations as long as I'm with you.", 4700);
    input.language = Some("fa".to_string());

    let f = facts(&input);

    assert!(
        f.hard_sounds
            .iter()
            .any(|(sound, word)| sound == "th" && word == "with")
    );
    assert!(f.hard_sounds.iter().any(|(sound, word)| sound == "w" && word == "with"));
}

#[test]
fn an_unknown_language_does_not_use_the_persian_profile() {
    let mut input = line("There are no bad situations as long as I'm with you.", 4700);
    input.language = Some("unknown".to_string());

    let f = facts(&input);

    assert!(f.hard_sounds.is_empty());
}

#[test]
fn a_known_episode_name_at_sentence_start_is_not_a_hard_word() {
    let mut input = line("Elizabeth arrived at the station.", 2500);
    input.names = vec!["Elizabeth".to_string()];

    let f = facts(&input);

    assert_eq!(f.names, ["elizabeth"]);
    assert!(f.hard_words.iter().all(|(w, _)| w != "elizabeth"));
}

#[test]
fn a_common_word_at_sentence_start_is_not_a_name() {
    let f = facts(&line("Maybe we should leave now.", 2500));

    assert!(f.names.is_empty());
}

#[test]
fn language_defaults_to_none_for_older_inputs() {
    let input = line("There are no bad situations.", 2500);

    assert_eq!(input.language, None);
}
