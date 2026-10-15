//! The rule engine (`sanjeh::connected::melts`) on lines: each rule where
//! it applies, and where it mustn't.

use sanjeh::Accent::{self, Gb, Us};
use sanjeh::connected::melts;

/// The line's melts as `rule sound`.
fn found(line: &str, accent: Accent) -> Vec<String> {
    melts(line, accent)
        .into_iter()
        .map(|melt| format!("{} {}", melt.rule, melt.sound))
        .collect()
}

fn has(line: &str, accent: Accent, melt: &str) {
    let found = found(line, accent);
    assert!(found.iter().any(|one| one == melt), "{line:?} ({accent:?}): {found:?}");
}

fn lacks(line: &str, accent: Accent, rule: &str) {
    let found = found(line, accent);
    assert!(
        !found.iter().any(|one| one.split(' ').next() == Some(rule)),
        "{line:?} ({accent:?}) has {rule}: {found:?}"
    );
}

#[test]
fn a_consonant_moves_onto_the_vowel_after_it() {
    has("Come on, Charley.", Gb, "linking co-mon");
    has("Would you like a drink?", Us, "linking li-ka");
    has("It's a bed.", Gb, "linking it-sa");
    // Links in a row are one melt.
    has("Pick it up.", Gb, "linking pi-ki-tup");
}

#[test]
fn between_two_vowels_a_w_or_y_slips_in() {
    has("Go away!", Us, "linking go-waway");
    has("Go away!", Gb, "linking go-waway");
}

#[test]
fn nothing_melts_across_a_pause() {
    lacks("Come, on.", Gb, "linking");
    lacks("Stop. Ask him.", Us, "linking");
}

#[test]
fn a_fixed_pair_fuses_and_keeps_its_words() {
    has("What are we going to do?", Us, "fusion gonna");
    has("I don't know.", Gb, "fusion dunno");
    has("What are you looking for?", Us, "fusion whatcha");
    // Going somewhere isn't gonna.
    lacks("I'm going to the shop.", Us, "fusion");
    lacks("We're going to London.", Gb, "fusion");
}

#[test]
fn a_small_word_is_said_lightly_unless_a_pause_follows() {
    has("Do you have a bed for me?", Gb, "weak-form fer");
    has("Fish and chips.", Us, "weak-form 'n'");
    lacks("What is it for?", Us, "weak-form");
    // *have* the verb is said whole.
    assert!(!found("Do you have a bed?", Us).contains(&"weak-form uv".to_owned()));
}

#[test]
fn a_t_or_d_between_consonants_drops() {
    has("I live next door.", Us, "elision nex door");
    has("OK. I'll just buy the shirt.", Gb, "elision jus buy");
    lacks("I want it.", Gb, "elision");
}

#[test]
fn t_d_s_and_z_melt_into_you() {
    has("Would you like a drink?", Gb, "yod wouldja");
    has("Did you hear something?", Us, "yod didja");
    has("Annie, it's not what you write,", Us, "yod whatcha");
    has("I miss you.", Gb, "yod mishoo");
}

#[test]
fn h_and_them_drop_inside_a_sentence() {
    has("Tell her.", Gb, "h-dropping te-ler");
    has("But how did he guess?", Us, "h-dropping di-dee");
    has("Kill them!", Us, "them ki-lem");
    lacks("He is here.", Us, "h-dropping");
}

#[test]
fn ing_is_said_in_in_words_of_two_syllables() {
    has("What on earth are you doing?", Us, "g-dropping doin'");
    has("I've got something for you.", Gb, "g-dropping somethin'");
    has("Oh, well, keep going, Hector.", Us, "g-dropping goin'");
    lacks("A good thing.", Us, "g-dropping");
}

#[test]
fn an_american_t_between_vowels_is_soft() {
    has("A little water, please.", Us, "flap-t liddle");
    has("Tell her about it.", Us, "flap-t abou-dit");
    lacks("A little water, please.", Gb, "flap-t");
    // A t that starts a stressed syllable stays.
    lacks("Attack!", Us, "flap-t");
}

#[test]
fn a_british_t_is_cut_off_before_a_consonant() {
    has("That's not for you.", Gb, "glottal-t no' for");
    lacks("That's not for you.", Us, "glottal-t");
}

#[test]
fn a_sound_changes_to_suit_the_next() {
    has("Hector ordered ten melons.", Us, "assimilation tem melons");
    has("Get in bed.", Gb, "assimilation im bed");
    has("In case.", Us, "assimilation ing case");
    has("Oh, good boy!", Gb, "assimilation goob boy");
}

#[test]
fn a_british_r_comes_back_before_a_vowel() {
    has("Where is it?", Gb, "linking-r whe-ris");
    has("Law and order.", Gb, "linking-r law-rand");
    // In American English the r is always said: it simply links.
    lacks("Where is it?", Us, "linking-r");
    has("Where is it?", Us, "linking whe-ri-zit");
}

#[test]
fn a_melt_knows_its_words_and_their_place_in_the_line() {
    let found = melts("Oh, good boy!", Gb);
    let good_boy = found.iter().find(|melt| melt.rule == "assimilation").expect("good boy");
    assert_eq!(good_boy.words, 1..3);
    assert_eq!(good_boy.span, 4..12);
    assert_eq!(good_boy.ipa, "gʊb bɔɪ");
    // UTF-16, as the app counts: an em dash is one unit, as é is.
    let found = melts("Café — come on.", Gb);
    assert_eq!(found[0].span, 7..14);
}

#[test]
fn every_rule_the_engine_names_is_in_the_data() {
    let data = sanjeh::connected::connected_speech();
    for line in [
        "Pick it up. Go away! What are we going to do? I live next door.",
        "Would you like a drink? Tell her. Kill them! What are you doing?",
        "A little water, please. That's not for you. Ten melons. Where is it?",
    ] {
        for accent in [Us, Gb] {
            for melt in melts(line, accent) {
                let rule = data.rule(melt.rule).expect(melt.rule);
                assert!(rule.accent.includes(accent), "{} in {accent:?}", melt.rule);
            }
        }
    }
}

#[test]
fn the_sound_is_written_as_it_is_heard() {
    // A silent h isn't written once the sound runs into it.
    has("To be honest, it's tempting.", Us, "linking be-yonest");
    // A t spelt -ed, the v of *of*, an x's s.
    has("but no one's ever witnessed it.", Us, "linking witness-tit");
    has("So you've gotta fix it.", Us, "linking fik-sit");
    has("A cup of it.", Gb, "linking cu-po-vit");
    let honest = melts("To be honest.", Us);
    assert_eq!(honest[1].ipa, "biː‿ˈjɑːnəst");
}

#[test]
fn some_words_are_left_alone() {
    // Letters said one by one.
    assert!(found("I got the GPS on.", Us).iter().all(|melt| !melt.contains("gp")));
    // A t before /ən/ is held, not softened.
    lacks("It's important.", Us, "flap-t");
    // An -ing word is said -in', and doesn't link as -ing.
    lacks("What's going on?", Us, "linking");
    has("What's going on?", Us, "g-dropping goin'");
}
