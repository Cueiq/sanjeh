//! The connected-speech data (`connectedSpeech.json`) held to its shape: the
//! twelve rules in their learning order, each named and explained in English
//! and Persian, with two to four examples in the accents it belongs to, and
//! the weak forms and fusions the engine works from.

use std::collections::HashSet;

use sanjeh::Accent;
use sanjeh::connected::{RuleAccent, Sounds, connected_speech, melts};

const RULES: [&str; 12] = [
    "linking",
    "weak-form",
    "fusion",
    "elision",
    "yod",
    "h-dropping",
    "g-dropping",
    "them",
    "flap-t",
    "glottal-t",
    "assimilation",
    "linking-r",
];

fn has_both(ipa: &Sounds) -> bool {
    ipa.get(Accent::Us).is_some_and(|us| !us.is_empty()) && ipa.get(Accent::Gb).is_some_and(|gb| !gb.is_empty())
}

#[test]
fn the_twelve_rules_in_their_learning_order() {
    let data = connected_speech();
    assert_eq!(data.format, 1);
    let ids: Vec<&str> = data.rules.iter().map(|rule| rule.id.as_str()).collect();
    assert_eq!(ids, RULES);
    let orders: Vec<u8> = data.rules.iter().map(|rule| rule.order).collect();
    assert!(orders.is_sorted(), "{orders:?}");
    assert_eq!(orders.iter().collect::<HashSet<_>>().len(), 9, "nine steps: {orders:?}");
    let beginners: Vec<&str> = data
        .rules
        .iter()
        .filter(|rule| rule.for_beginners)
        .map(|rule| rule.id.as_str())
        .collect();
    assert_eq!(beginners, ["linking", "weak-form", "fusion"]);
}

#[test]
fn an_accent_meets_its_own_rules() {
    let data = connected_speech();
    let ids = |accent| -> Vec<&str> { data.rules_for(accent).map(|rule| rule.id.as_str()).collect() };
    let (us, gb) = (ids(Accent::Us), ids(Accent::Gb));
    assert!(us.contains(&"flap-t") && !gb.contains(&"flap-t"));
    assert!(gb.contains(&"glottal-t") && !us.contains(&"glottal-t"));
    assert!(gb.contains(&"linking-r") && !us.contains(&"linking-r"));
    assert_eq!((us.len(), gb.len()), (10, 11));
}

#[test]
fn every_rule_is_named_and_explained_in_english_and_persian() {
    for rule in &connected_speech().rules {
        for (what, texts) in [("name", &rule.name), ("explanation", &rule.explanation)] {
            let languages: Vec<&str> = texts.keys().map(String::as_str).collect();
            assert_eq!(languages, ["en", "fa"], "{} {what}", rule.id);
            assert!(texts.values().all(|text| !text.trim().is_empty()));
            let persian = &texts["fa"];
            // Persian letters, not Arabic ones, and the verb prefix joined
            // with the zero-width non-joiner.
            assert!(!persian.contains(['ي', 'ك']), "{} {what}: {persian}", rule.id);
            assert!(
                !persian.split_whitespace().any(|word| word == "می"),
                "{} {what}: {persian}",
                rule.id
            );
        }
    }
}

#[test]
fn every_example_shows_its_melt_in_the_accents_of_its_rule() {
    for rule in &connected_speech().rules {
        assert!(
            (2..=4).contains(&rule.examples.len()),
            "{}: {} examples",
            rule.id,
            rule.examples.len()
        );
        for example in &rule.examples {
            let at = format!("{} {:?}", rule.id, example.melt);
            assert!(
                example.text.to_lowercase().contains(&example.melt.to_lowercase()),
                "{at}: not in {:?}",
                example.text
            );
            assert!(!example.sound.trim().is_empty(), "{at}");
            let accents = (
                example.ipa.get(Accent::Us).is_some(),
                example.ipa.get(Accent::Gb).is_some(),
            );
            let wanted = match rule.accent {
                RuleAccent::Both => (true, true),
                RuleAccent::Us => (true, false),
                RuleAccent::Gb => (false, true),
            };
            assert_eq!(accents, wanted, "{at}: its IPA's accents");
            if let Some(from) = &example.from {
                let (lesson, line) = from.split_once(" #").unwrap_or_else(|| panic!("{at}: {from}"));
                assert!(
                    lesson
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                    "{at}: {from}"
                );
                assert!(line.parse::<u64>().is_ok(), "{at}: {from}");
            }
        }
    }
}

#[test]
fn the_word_lists_are_whole_and_the_examples_in_them() {
    let data = connected_speech();
    let weak: HashSet<&str> = data.weak_forms.iter().map(|form| form.word.as_str()).collect();
    assert_eq!(weak.len(), data.weak_forms.len(), "a weak form listed twice");
    let fused: HashSet<&str> = data.fusions.iter().map(|fusion| fusion.words.as_str()).collect();
    assert_eq!(fused.len(), data.fusions.len(), "a fusion listed twice");
    for form in &data.weak_forms {
        assert!(has_both(&form.ipa) && !form.sound.is_empty(), "{}", form.word);
    }
    for fusion in &data.fusions {
        assert!(has_both(&fusion.ipa) && !fusion.sound.is_empty(), "{}", fusion.words);
        assert!(
            !fusion.words.is_empty() && fusion.words.split(' ').all(|word| !word.is_empty()),
            "{}: fusion words must be non-empty and separated by single spaces",
            fusion.words
        );
    }
    // A fusion's examples are fusions on the list, and a weak form's have a
    // word on it.
    for example in &data.rule("fusion").expect("fusion").examples {
        assert!(fused.contains(example.melt.to_lowercase().as_str()), "{}", example.melt);
    }
    for example in &data.rule("weak-form").expect("weak-form").examples {
        let melt = example.melt.to_lowercase();
        assert!(melt.split(' ').any(|word| weak.contains(word)), "{}", example.melt);
    }
}

#[test]
fn written_single_word_fusions_are_found() {
    let cases = [
        ("kinda", "kinda"),
        ("sorta", "sorta"),
        ("dunno", "dunno"),
        ("lemme", "lemme"),
        ("gimme", "gimme"),
    ];

    for (input, expected_sound) in cases {
        for accent in [Accent::Us, Accent::Gb] {
            let results = melts(input, accent);
            let fusion = results
                .iter()
                .find(|melt| melt.rule == "fusion")
                .unwrap_or_else(|| panic!("{input}: fusion not found for {accent:?}"));

            assert_eq!(fusion.sound, expected_sound, "{input}, {accent:?}");
            assert!(!fusion.ipa.is_empty(), "{input}, {accent:?}: empty IPA");
        }
    }
}
