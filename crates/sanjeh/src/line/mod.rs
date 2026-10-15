//! How hard a film line is to **hear** (written down from the film: the
//! ear) and to **say** (shadowed with the actor: the mouth), and how much it
//! is worth practising.
//!
//! A line is measured in three steps:
//! - its **features**, computed from the line by tested code ([`features`]);
//! - whether it can be practised at all, for each skill (one voice, a whole
//!   sentence, a length to suit the skill…);
//! - its **level** (on the CEFR scale, A1 = 1 … C2 = 6) and **worth** (0–1)
//!   for each skill, by a [`LineModel`] — weights fitted to learners'
//!   ratings, in `models/line/`.
//!
//! Beside them come the **reasons**, what makes the line hard and what makes
//! it worth it, in a form an app can put in words.

mod features;
pub mod heard;
mod model;

use serde::{Deserialize, Serialize};

pub use features::{Facts, Features, facts};
pub use heard::{Heard, HeardWord};
pub use model::{LinearModel, LineModel, SkillModel};

use crate::Accent;
use crate::levels::Level;

/// A line to measure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LineInput {
    /// What is said, as the subtitle writes it, without its markup or the
    /// notes for the deaf (*(CHUCKLES)*, *STUART:*). A sentence that runs
    /// over two subtitles is one line.
    pub text: String,
    /// How long it is said for.
    pub duration_ms: u32,
    /// The accent it is said in, for its melts and sounds.
    #[serde(default = "american")]
    pub accent: Accent,
    /// How many people say it: two (`- What? - Nothing.`) can't be practised.
    #[serde(default = "one")]
    pub voices: u8,
    /// What speech recognisers of three strengths heard of it, if they were
    /// asked: the weak ear ([`heard`]).
    #[serde(default)]
    pub heard: Option<Heard>,
    /// The words and phrases a guide teaches in it, with their levels.
    #[serde(default)]
    pub entries: Vec<Entry>,
    /// Names known from elsewhere in its episode, so a name that only opens
    /// a sentence here is still a name.
    #[serde(default)]
    pub names: Vec<String>,
}

fn american() -> Accent {
    Accent::Us
}

fn one() -> u8 {
    1
}

/// A word, phrase or grammar point a guide teaches in the line.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Entry {
    pub term: String,
    /// `word`, `phrase`, `phrasal_verb`, `idiom`, `expression`, `slang`,
    /// `grammar`… — a `reference` or `pronunciation` is not learnt from the line.
    pub kind: String,
    #[serde(default)]
    pub level: Option<Level>,
}

impl Entry {
    /// Whether the entry is something learnt from the line's words.
    pub fn learnt(&self) -> bool {
        matches!(self.kind.as_str(), "word" | "phrase" | "phrasal_verb" | "idiom" | "expression" | "slang" | "grammar")
    }
}

/// The two skills a line is practised for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Skill {
    /// Written down from the film: dictation.
    Ear,
    /// Said with the actor: shadowing.
    Mouth,
}

/// Why a line can't be practised for a skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum WhyNot {
    /// It is cut off, or starts mid-sentence.
    NotASentence,
    /// More than one person says it.
    TwoVoices,
    /// Fewer words than the skill needs.
    TooShort { words: usize },
    /// More words than one breath, or one listen, holds.
    TooLong { words: usize },
    /// Said too fast or too slowly to take whole.
    Spoken { ms: u32 },
    /// Mostly names, which can't be learnt from it.
    Names { words: Vec<String> },
    /// Words said wrong on purpose (*weawy* for *really*): no answer to hold
    /// a learner to.
    SaidWrong { words: Vec<String> },
    /// Even the strongest recogniser missed much of it: noise, music, or a
    /// subtitle that says something else.
    NotClear,
}

/// Something that makes the line hard, or worth it, for an app to say.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum Reason {
    /// How many words, in how long.
    Pace { words: usize, seconds: f64, syllables: usize },
    /// Words that melt into each other, by the rules of connected speech.
    Melt { rule: String, text: String, sound: String },
    /// A pair written as it is said: *gonna* for *going to*.
    Written { word: String, full: String },
    /// A word a weaker recogniser missed or was unsure of.
    WeakEar { word: String },
    /// A word above B1.
    HardWord { word: String, level: Level },
    /// A sound Persian and many other learners find hard (`th`, `w`, an
    /// *s* before a consonant, three consonants at a word's end).
    HardSound { sound: String, word: String },
    /// Something a guide teaches in the line.
    Teaches { term: String, kind: String },
    /// How many of its words are A1–B1.
    Everyday { share: f64 },
    /// Asked, or said with feeling.
    Tune { question: bool },
}

/// A line's measure for one skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeasure {
    pub usable: bool,
    pub why_not: Vec<WhyNot>,
    /// On the CEFR scale: A1 is 1.0 up to 2.0, … C2 is 6.0 and above.
    pub level: f64,
    /// How much it is worth practising, 0 to 1.
    pub worth: f64,
    pub reasons: Vec<Reason>,
}

/// A line's measure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineMeasure {
    pub features: Features,
    pub ear: SkillMeasure,
    pub mouth: SkillMeasure,
}

/// The model lines are measured by when none is chosen: the newest in
/// `models/line/` that has beaten the one before it on the benchmarks.
pub fn default_model() -> &'static LineModel {
    static MODEL: std::sync::LazyLock<LineModel> = std::sync::LazyLock::new(|| {
        LineModel::from_json(include_str!("../../../../models/line/v1.json")).expect("models/line/v1.json is a line model")
    });
    &MODEL
}

/// Measures a line with a model.
pub fn measure(input: &LineInput, model: &LineModel) -> LineMeasure {
    let facts = facts(input);
    let skill = |skill: Skill| {
        let weights = match skill {
            Skill::Ear => &model.ear,
            Skill::Mouth => &model.mouth,
        };
        let why_not = facts.why_not(skill);
        SkillMeasure {
            usable: why_not.is_empty(),
            why_not,
            level: weights.level.value(&facts.features).clamp(1.0, 6.99),
            worth: sigmoid(weights.worth.value(&facts.features)),
            reasons: facts.reasons(skill),
        }
    };
    LineMeasure { ear: skill(Skill::Ear), mouth: skill(Skill::Mouth), features: facts.features.clone() }
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}
