//! The contract of `made_up_words` (`docs/placement.md`, "Vocabulary
//! yes/no"), seen from outside the crate, as the vocabulary test's caller
//! sees it: the same seed gives the same words, a different seed other
//! words, none of the words is a real word or one change from a listed
//! word, none has a real base form, none is vowel-less or offensive, and a
//! test that asks for `n` gets `n`.
//!
//! The length and syllable spread tests live inside `made_up` itself,
//! where the listed words the spread is measured against (`BANDS`,
//! `LISTED`) are in scope; the pronouncing table's iterator is
//! `pub(crate)`, so they cannot live here.

use sanjeh::levels::{base_forms, listed_level};
use sanjeh::placement::made_up_words;
use sanjeh::{Accent, Pronounced, pronounce};

/// The same seed gives the same words.
#[test]
fn the_same_seed_gives_the_same_words() {
    assert_eq!(made_up_words(50, 1), made_up_words(50, 1));
}

/// A different seed gives other words.
#[test]
fn a_different_seed_gives_other_words() {
    assert_ne!(made_up_words(50, 1), made_up_words(50, 2));
}

/// A made-up word must not be a word, not one change from a listed word,
/// not have a real base form, not be vowel-less, and not be offensive.
/// Several seeds and many words, so no one run's luck decides; the filters
/// inside `made_up` already check these against the same table and lists,
/// so this looks from outside the crate and gives the contract teeth.
#[test]
fn no_word_is_a_real_word_near_word_or_base_form() {
    for seed in 1..=5 {
        for word in made_up_words(200, seed) {
            assert!(
                matches!(pronounce(&word, Accent::Us), Pronounced::Unknown),
                "{word} is in the pronouncing table"
            );
            assert!(listed_level(&word).is_none(), "{word} is in the word lists");
            for form in base_forms(&word) {
                assert!(
                    matches!(pronounce(&form, Accent::Us), Pronounced::Unknown),
                    "{word}'s base form {form} is in the pronouncing table"
                );
                assert!(
                    listed_level(&form).is_none(),
                    "{word}'s base form {form} is in the word lists"
                );
            }
            assert!(
                word.bytes()
                    .any(|byte| matches!(byte, b'a' | b'e' | b'i' | b'o' | b'u' | b'y')),
                "{word} has no vowel"
            );
        }
    }
}

/// A made-up word must not show an offensive substring. With a blocklist
/// inside `made_up` and ~0.5% of unblocked words rude, a few thousand
/// words across several seeds would have shown some; this asserts none.
#[test]
fn no_word_is_offensive() {
    let bad = [
        "arse", "assb", "assh", "assw", "boob", "cock", "crap", "cunt", "dick", "fagg", "fuck", "jizz", "kike", "nigg",
        "rape", "shit", "slut", "spic", "tits", "twat", "wank",
    ];
    for seed in 1..=10 {
        for word in made_up_words(300, seed) {
            for &needle in &bad {
                assert!(!word.contains(needle), "{word} contains {needle}");
            }
        }
    }
}

/// A made-up word stays within the lengths real words come at, 2–12
/// letters; the spread test inside `made_up` checks the lengths follow the
/// lists' spread. This is the outside-the-crate floor.
#[test]
fn lengths_stay_within_two_to_twelve() {
    for word in made_up_words(200, 1) {
        let len = word.len();
        assert!((2..=12).contains(&len), "{word} is {len} letters");
    }
}

/// A test that asks for `n` gets `n`, no matter how many candidates the
/// filters refuse, since generation is lazy. Several sizes and seeds.
#[test]
fn as_many_as_asked() {
    assert!(made_up_words(0, 1).is_empty());
    for n in [1, 5, 50, 200, 500] {
        assert_eq!(made_up_words(n, 1).len(), n, "asked for {n}");
    }
    for seed in 1..=3 {
        assert_eq!(made_up_words(200, seed).len(), 200, "seed {seed}");
    }
}
