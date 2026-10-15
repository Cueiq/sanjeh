//! Benchmarks: labelled items, and how well a model agrees with them.
//!
//! A line benchmark is a JSON Lines file, an item a line:
//!
//! ```json
//! {"id": "e06-73", "group": "e06", "split": "test",
//!  "input": { …a LineInput… },
//!  "labels": [{"rater": "r1", "rater_level": "B2", "skill": "ear",
//!              "level": "right", "worth": "yes"}]}
//! ```
//!
//! A rater says, for themselves, whether a line was `easy`, `right` or
//! `hard`, and whether it was worth practising: `yes`, `meh` or `no`. Their
//! own level turns that into a statement about the line's level: a line a B2
//! rater found right is B2, one they found easy is below B2, one they found
//! hard is C1 or above.

use serde::{Deserialize, Serialize};

use crate::levels::Level;
use crate::line::{LineInput, LineMeasure, Skill};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Item {
    pub id: String,
    /// Items measured together: an episode.
    #[serde(default)]
    pub group: String,
    /// `fit` (for fitting models) or `test` (never fitted on).
    pub split: String,
    pub input: LineInput,
    pub labels: Vec<Label>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Label {
    pub rater: String,
    pub rater_level: Level,
    pub skill: Skill,
    #[serde(default)]
    pub level: Option<Vote>,
    #[serde(default)]
    pub worth: Option<Worth>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Vote {
    Easy,
    Right,
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Worth {
    Yes,
    Meh,
    No,
}

/// The vote a level makes for a rater: below their level is easy, within
/// it right, above it hard.
pub fn vote_for(level: f64, rater: Level) -> Vote {
    let r = rater.rank();
    if level < r {
        Vote::Easy
    } else if level < r + 1.0 {
        Vote::Right
    } else {
        Vote::Hard
    }
}

/// The chance a randomly chosen positive scores above a randomly chosen
/// negative (½ for chance, 1 for perfect); `None` without both.
pub fn auc(scored: &[(f64, bool)]) -> Option<f64> {
    let pos: Vec<f64> = scored.iter().filter(|(_, y)| *y).map(|(s, _)| *s).collect();
    let neg: Vec<f64> = scored.iter().filter(|(_, y)| !*y).map(|(s, _)| *s).collect();
    if pos.is_empty() || neg.is_empty() {
        return None;
    }
    let mut won = 0.0;
    for p in &pos {
        for n in &neg {
            won += if p > n {
                1.0
            } else if p == n {
                0.5
            } else {
                0.0
            };
        }
    }
    Some(won / (pos.len() * neg.len()) as f64)
}

/// How a model did on a skill.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SkillReport {
    pub rated: usize,
    /// The share of votes the model's level made exactly.
    pub votes_matched: Option<f64>,
    /// Whether its levels rank the lines a rater found easy below the rest.
    pub level_auc: Option<f64>,
    /// Whether its worth ranks the lines rated worth it (yes) above the rest.
    pub worth_auc: Option<f64>,
    /// Of the lines it puts at the rater's level, the share they found right.
    pub at_level_right: Option<f64>,
    /// Of the lines it calls worth it (over ½), the share rated yes.
    pub worth_yes: Option<f64>,
    /// Votes matched, by the level the model gave the line.
    pub by_level: Vec<(Level, usize, f64)>,
}

/// Scores a skill's measures against its labels.
pub fn report(items: &[(Item, LineMeasure)], skill: Skill) -> SkillReport {
    let mut rows = vec![];
    for (item, measure) in items {
        let m = match skill {
            Skill::Ear => &measure.ear,
            Skill::Mouth => &measure.mouth,
        };
        for label in item.labels.iter().filter(|l| l.skill == skill) {
            rows.push((m.level, m.worth, label.clone()));
        }
    }
    let share = |hits: usize, of: usize| (of > 0).then(|| hits as f64 / of as f64);
    let voted: Vec<_> = rows
        .iter()
        .filter_map(|(level, _, l)| l.level.map(|v| (*level, v, l.rater_level)))
        .collect();
    let matched = voted.iter().filter(|(level, v, r)| vote_for(*level, *r) == *v).count();
    let level_auc = auc(&voted
        .iter()
        .map(|(level, v, r)| (level - r.rank(), *v != Vote::Easy))
        .collect::<Vec<_>>());
    let worths: Vec<_> = rows
        .iter()
        .filter_map(|(_, w, l)| l.worth.map(|x| (*w, x == Worth::Yes)))
        .collect();
    let at_level: Vec<_> = voted
        .iter()
        .filter(|(level, _, r)| vote_for(*level, *r) == Vote::Right)
        .collect();
    let called: Vec<_> = worths.iter().filter(|(w, _)| *w > 0.5).collect();
    let by_level = Level::ALL
        .into_iter()
        .filter_map(|band| {
            let here: Vec<_> = voted
                .iter()
                .filter(|(level, _, _)| Level::of_rank(*level) == band)
                .collect();
            (!here.is_empty()).then(|| {
                let hit = here.iter().filter(|(level, v, r)| vote_for(*level, *r) == *v).count();
                (band, here.len(), hit as f64 / here.len() as f64)
            })
        })
        .collect();
    SkillReport {
        rated: rows.len(),
        votes_matched: share(matched, voted.len()),
        level_auc,
        worth_auc: auc(&worths),
        at_level_right: share(
            at_level.iter().filter(|(_, v, _)| *v == Vote::Right).count(),
            at_level.len(),
        ),
        worth_yes: share(called.iter().filter(|(_, y)| *y).count(), called.len()),
        by_level,
    }
}
