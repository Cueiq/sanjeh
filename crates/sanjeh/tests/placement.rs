//! The contract of `made_up_words` (docs/placement.md, "Vocabulary
//! yes/no"), seen from outside the crate, as the vocabulary test's caller
//! sees it: the same seed gives the same words, a different seed other
//! words, none of the words is a real word, each is as long as real words
//! come, and a test that asks for `n` gets `n`.

use sanjeh::levels::listed_level;
use sanjeh::placement::made_up_words;
use sanjeh::{Accent, Pronounced, pronounce};

#[test]
fn the_same_seed_gives_the_same_words() {
    assert_eq!(made_up_words(50, 1), made_up_words(50, 1));
}

#[test]
fn a_different_seed_gives_other_words() {
    assert_ne!(made_up_words(50, 1), made_up_words(50, 2));
}

/// A made-up word must not be a word: not in the pronouncing table, not in
/// the word lists. Several seeds, so no one run's luck decides.
#[test]
fn no_word_is_a_real_word() {
    for seed in 1..=3 {
        for word in made_up_words(200, seed) {
            assert!(
                matches!(pronounce(&word, Accent::Us), Pronounced::Unknown),
                "{word} is in the pronouncing table"
            );
            assert!(listed_level(&word).is_none(), "{word} is in the word lists");
        }
    }
}

/// Real words of two to twelve letters are everywhere in the table; a
/// made-up word stays within those lengths, so its length is no tell.
#[test]
fn lengths_sit_where_real_words_do() {
    for word in made_up_words(200, 1) {
        let len = word.len();
        assert!((2..=12).contains(&len), "{word} is {len} letters");
    }
}

#[test]
fn as_many_as_asked() {
    assert_eq!(made_up_words(10, 1).len(), 10);
    assert!(made_up_words(0, 1).is_empty());
    for seed in 1..=3 {
        assert_eq!(made_up_words(200, seed).len(), 200);
    }
}
