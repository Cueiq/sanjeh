//! Sanjeh (سنجه, "a measure"): what a language learner's material asks of
//! them, measured — how an English word sounds, where a line's words melt
//! into each other, and (to come) how hard a line is to hear and to say, and
//! how much it is worth practising. Each measure has a benchmark beside it,
//! so it can be improved by anyone, at every level from A1 to C2.

pub mod connected;
mod pronounce;
pub use pronounce::{Accent, Pronounced, pronounce, warm_pronouncing};

/// The rules of connected speech and their word lists, as data: for a copy
/// kept beside an app's own code, held to this one by its tests.
pub const CONNECTED_SPEECH_JSON: &str = include_str!("../../../data/connected-speech.json");

/// The notices the pronouncing table's sources ask to be shipped with it.
pub const PRONOUNCING_NOTICE: &str = include_str!("../../../data/pronouncing/NOTICE.md");

use unicode_normalization::UnicodeNormalization;

/// What parts an English term's words: the ASCII spaces and controls, NEL,
/// the no-break and typographic spaces, the line and paragraph separators
/// and the byte-order mark.
const TERM_SPACES: [char; 26] = [
    '\u{0009}', '\u{000A}', '\u{000B}', '\u{000C}', '\u{000D}', '\u{0020}', '\u{0085}', '\u{00A0}',
    '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}',
    '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{2028}', '\u{2029}', '\u{202F}', '\u{205F}',
    '\u{3000}', '\u{FEFF}',
];

/// The marks written for an apostrophe: the typewriter one, and the curly,
/// reversed, modifier, prime, accent and full-width forms that stand in for
/// it.
const APOSTROPHES: [char; 10] = ['\'', '‘', '’', '‛', 'ʼ', 'ʻ', '′', '`', '´', '＇'];
/// The hyphens written for `-`.
const HYPHENS: [char; 2] = ['\u{2010}', '\u{2011}'];

/// An English word or phrase as a learner knows it, the same wherever it is
/// written: composed (NFC); its words one space apart, none around them;
/// lower case; every apostrophe the typewriter one, every hyphen `-`, and
/// three dots an ellipsis. Bembegoo's `term_key` for English, which its
/// tests hold to this.
pub fn term_key(term: &str) -> String {
    let composed: String = term.nfc().collect();
    composed
        .split(|char| TERM_SPACES.contains(&char))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .chars()
        .map(|char| {
            if APOSTROPHES.contains(&char) {
                '\''
            } else if HYPHENS.contains(&char) {
                '-'
            } else {
                char
            }
        })
        .collect::<String>()
        .replace("...", "…")
}
