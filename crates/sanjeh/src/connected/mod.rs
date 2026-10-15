//! Connected speech: how English words melt into each other when said. The
//! twelve rules, with their names and explanations in English and Persian
//! and their examples, and the word lists the engine works from — weak forms
//! and fixed fusions — are data, `data/connected-speech.json`
//! (`CONNECTED_SPEECH_JSON`, for an app keeping a copy beside its own code);
//! the engine, `melts`, finds them in a line.

mod melted;
mod melts;

use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::Accent;

pub use melted::{Piece, melted};
pub use melts::{Melt, line_words, melts};

const DATA: &str = crate::CONNECTED_SPEECH_JSON;

static CONNECTED: LazyLock<ConnectedSpeech> = LazyLock::new(|| {
    serde_json::from_str(DATA).expect("connectedSpeech.json is the rules and their word lists")
});

/// The rules and the word lists.
pub fn connected_speech() -> &'static ConnectedSpeech {
    &CONNECTED
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectedSpeech {
    pub format: u64,
    /// In the order they are learnt.
    pub rules: Vec<Rule>,
    /// Small words said lightly when unstressed: *for* as *fer*.
    pub weak_forms: Vec<WeakForm>,
    /// Common pairs said as one word: *going to* as *gonna*. Matched before
    /// the other rules, and winning over them.
    pub fusions: Vec<Fusion>,
}

impl ConnectedSpeech {
    pub fn rule(&self, id: &str) -> Option<&Rule> {
        self.rules.iter().find(|rule| rule.id == id)
    }

    /// The rules a learner of this accent meets: those of both accents, and
    /// the accent's own (flap T in American, glottal T and linking R in
    /// British).
    pub fn rules_for(&self, accent: Accent) -> impl Iterator<Item = &Rule> {
        self.rules
            .iter()
            .filter(move |rule| rule.accent.includes(accent))
    }
}

/// Text in each language, by its code: `en`, `fa`.
pub type Texts = BTreeMap<String, String>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub accent: RuleAccent,
    /// Its place in the learning order, from easy and common to hard; rules
    /// learnt together share it.
    pub order: u8,
    /// One of the three an A1 or A2 learner starts with.
    pub for_beginners: bool,
    pub name: Texts,
    pub explanation: Texts,
    pub examples: Vec<Example>,
}

/// The accents a rule belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAccent {
    Both,
    Us,
    Gb,
}

impl RuleAccent {
    pub fn includes(self, accent: Accent) -> bool {
        match self {
            Self::Both => true,
            Self::Us => accent == Accent::Us,
            Self::Gb => accent == Accent::Gb,
        }
    }
}

/// A rule at work in a line.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Example {
    /// The line, as its subtitle writes it.
    pub text: String,
    /// The words of it that melt.
    pub melt: String,
    /// How they sound, in plain letters: `ka-mon`.
    pub sound: String,
    pub ipa: Sounds,
    /// The course line it is, `extra-s01e02 #16`, for proving it from the
    /// actor's own voice; none for a common phrase.
    #[serde(default)]
    pub from: Option<String>,
}

/// IPA in each accent the rule belongs to; `‿` marks a joint.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sounds {
    #[serde(default)]
    pub us: Option<String>,
    #[serde(default)]
    pub gb: Option<String>,
}

impl Sounds {
    pub fn get(&self, accent: Accent) -> Option<&str> {
        match accent {
            Accent::Us => self.us.as_deref(),
            Accent::Gb => self.gb.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeakForm {
    pub word: String,
    pub sound: String,
    pub ipa: Sounds,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fusion {
    pub words: String,
    pub sound: String,
    pub ipa: Sounds,
}
