//! The placement test's engine: from a learner's answers, where their level
//! is, which item to give next, and when to stop. The design is in
//! `docs/placement.md`.
//!
//! Everything here is a pure function of its inputs: no data is needed, and
//! the only randomness is a seed the caller gives, so a test can be run again
//! and give the same questions.

use serde::{Deserialize, Serialize};

use crate::levels::Level;
use crate::line::Skill;

/// One part of the test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Task {
    /// Is this an English word? Yes or no.
    Vocabulary,
    /// Hear a line and write it down.
    Dictation,
    /// Say a line.
    Shadowing,
}

/// A question the learner can be given.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub skill: Skill,
    pub task: Task,
    /// On the scale the measures use: A1 is 1, C2 is 6.
    pub difficulty: f64,
    /// How sharply the item separates learners below its difficulty from
    /// those above it. With 1.0 for every item the model is the Rasch model.
    pub discrimination: f64,
    /// How many words the line has: how many chances to be right it holds.
    pub words: u32,
}

/// What the learner did with one item: `right` words of the `of` asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    pub item: String,
    pub right: u32,
    pub of: u32,
}

/// Where the learner's level is: `theta` on the same scale as the item
/// difficulties, and its standard error `se` (how far it may be off).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Estimate {
    pub theta: f64,
    pub se: f64,
}

/// The items a test can draw from.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bank {
    pub items: Vec<Item>,
}

impl Bank {
    /// The item with this id, if the bank has it.
    pub fn get(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|item| item.id == id)
    }
}

/// What is believed about the learner before any answer: a normal curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Prior {
    pub mean: f64,
    pub sd: f64,
}

impl Prior {
    /// Centred on `mean`, and wide (an SD of 1.2 levels, as the design
    /// says) so that a few answers can move the estimate a long way.
    pub fn centred_on(mean: f64) -> Prior {
        Prior { mean, sd: 1.2 }
    }
}

/// When a part of the test stops: once the standard error is below `se`, or
/// after `most` items, and never before `fewest`, so one lucky answer can't
/// end it. The numbers are set from simulation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StopRule {
    pub se: f64,
    pub fewest: usize,
    pub most: usize,
}

/// Where one level ends and the next begins: A1|A2, A2|B1, B1|B2, B2|C1,
/// C1|C2. Until learners with a known level have taken the test, they sit at
/// whole numbers, as `Level::of_rank` has them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Cuts {
    pub between: [f64; 5],
}

impl Default for Cuts {
    fn default() -> Cuts {
        Cuts { between: [2.0, 3.0, 4.0, 5.0, 6.0] }
    }
}


impl Item {
    /// Probability that a learner of ability `theta` answers a word correctly.
    /// Uses 2-parameter logistic model (2PL).
    #[inline]
    pub fn p(&self, theta: f64) -> f64 {
        let z = self.discrimination * (theta - self.difficulty);
        1.0 / (1.0 + (-z).exp())
    }

    /// Fisher Information of the item at ability `theta`: a^2 * p * (1 - p) * words.
    #[inline]
    pub fn information(&self, theta: f64) -> f64 {
        let p = self.p(theta);
        let q = 1.0 - p;
        let a = self.discrimination;
        (a * a) * p * q * (self.words as f64)
    }
}

/// EAP (Expected A Posteriori) estimation over a grid of theta.
/// Grid of 121 points from -1.0 to 8.0 on the CEFR-lined scale with normal prior.
pub fn estimate(answers: &[Answer], bank: &Bank, prior: Prior) -> Estimate {
    const GRID_SIZE: usize = 121;
    const MIN_THETA: f64 = -1.0;
    const MAX_THETA: f64 = 8.0;
    const STEP: f64 = (MAX_THETA - MIN_THETA) / ((GRID_SIZE - 1) as f64);

    let grid_thetas: [f64; GRID_SIZE] = std::array::from_fn(|i| MIN_THETA + (i as f64) * STEP);

    let mut log_posteriors = [0.0; GRID_SIZE];
    let mut max_log_post = f64::NEG_INFINITY;

    for (i, &theta) in grid_thetas.iter().enumerate() {
        let z = (theta - prior.mean) / prior.sd;
        let log_prior = -0.5 * z * z;

        let mut log_lik = 0.0;
        for ans in answers {
            if let Some(item) = bank.get(&ans.item) {
                let p = item.p(theta).clamp(1e-12, 1.0 - 1e-12);
                let q = (1.0 - p).clamp(1e-12, 1.0 - 1e-12);
                log_lik += (ans.right as f64) * p.ln() + ((ans.of.saturating_sub(ans.right)) as f64) * q.ln();
            }
        }

        let lp = log_prior + log_lik;
        log_posteriors[i] = lp;
        if lp > max_log_post {
            max_log_post = lp;
        }
    }

    let mut weights = [0.0; GRID_SIZE];
    let mut total_weight = 0.0;
    for (i, &lp) in log_posteriors.iter().enumerate() {
        let w = (lp - max_log_post).exp();
        weights[i] = w;
        total_weight += w;
    }

    if total_weight <= 0.0 {
        return Estimate {
            theta: prior.mean,
            se: prior.sd,
        };
    }

    let mut mean_theta = 0.0;
    for (i, &theta) in grid_thetas.iter().enumerate() {
        mean_theta += theta * (weights[i] / total_weight);
    }

    let mut var_theta = 0.0;
    for (i, &theta) in grid_thetas.iter().enumerate() {
        let diff = theta - mean_theta;
        var_theta += diff * diff * (weights[i] / total_weight);
    }

    Estimate {
        theta: mean_theta,
        se: var_theta.max(1e-6).sqrt(),
    }
}

/// Simple deterministic hash to pick an index with caller's seed (no hidden randomness).
fn pseudo_random_index(seed: u64, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let mut z = seed.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((z ^ (z >> 31)) as usize) % count
}

/// The most informative items at the current theta, one drawn from the best few
/// with the caller's seed.
pub fn next_item<'a>(
    bank: &'a Bank,
    now: Estimate,
    taken: &[String],
    part: Task,
    seed: u64,
) -> Option<&'a Item> {
    let mut candidates: Vec<(&Item, f64)> = bank
        .items
        .iter()
        .filter(|item| item.task == part && !taken.iter().any(|id| id == &item.id))
        .map(|item| (item, item.information(now.theta)))
        .collect();

    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let pool_size = candidates.len().min(3);
    let chosen_idx = pseudo_random_index(seed, pool_size);
    Some(candidates[chosen_idx].0)
}

/// SE below a threshold, or the most items; never before the fewest.
pub fn finished(now: Estimate, asked: usize, rule: StopRule) -> bool {
    if asked < rule.fewest {
        return false;
    }
    if asked >= rule.most {
        return true;
    }
    now.se <= rule.se
}

/// The CEFR level a theta falls in.
pub fn level(theta: f64, cuts: Cuts) -> Level {
    if theta < cuts.between[0] {
        Level::A1
    } else if theta < cuts.between[1] {
        Level::A2
    } else if theta < cuts.between[2] {
        Level::B1
    } else if theta < cuts.between[3] {
        Level::B2
    } else if theta < cuts.between[4] {
        Level::C1
    } else {
        Level::C2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bank() -> Bank {
        let mut items = Vec::new();
        for i in 1..=20 {
            let diff = 1.0 + (i as f64) * 0.25;
            items.push(Item {
                id: format!("item_{i}"),
                skill: Skill::Ear,
                task: Task::Dictation,
                difficulty: diff,
                discrimination: 1.0,
                words: 5,
            });
        }
        Bank { items }
    }

    #[test]
    fn information_peaks_where_p_is_half() {
        let item = Item {
            id: "i1".to_string(),
            skill: Skill::Ear,
            task: Task::Dictation,
            difficulty: 3.0,
            discrimination: 1.2,
            words: 1,
        };
        let max_info = item.information(3.0);
        let info_lower = item.information(2.0);
        let info_higher = item.information(4.0);

        assert!(max_info > info_lower);
        assert!(max_info > info_higher);
        assert!((item.p(3.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn learner_who_gets_everything_right_climbs() {
        let bank = sample_bank();
        let prior = Prior::centred_on(3.0);

        let initial = estimate(&[], &bank, prior);
        let answers = vec![
            Answer { item: "item_5".to_string(), right: 5, of: 5 },
            Answer { item: "item_8".to_string(), right: 5, of: 5 },
            Answer { item: "item_12".to_string(), right: 5, of: 5 },
        ];
        let after = estimate(&answers, &bank, prior);

        assert!(after.theta > initial.theta);
    }

    #[test]
    fn learner_who_gets_everything_wrong_falls() {
        let bank = sample_bank();
        let prior = Prior::centred_on(3.0);

        let initial = estimate(&[], &bank, prior);
        let answers = vec![
            Answer { item: "item_5".to_string(), right: 0, of: 5 },
            Answer { item: "item_8".to_string(), right: 0, of: 5 },
            Answer { item: "item_12".to_string(), right: 0, of: 5 },
        ];
        let after = estimate(&answers, &bank, prior);

        assert!(after.theta < initial.theta);
    }

    #[test]
    fn se_shrinks_with_each_answer() {
        let bank = sample_bank();
        let prior = Prior::centred_on(3.0);

        let e0 = estimate(&[], &bank, prior);
        let a1 = vec![Answer { item: "item_8".to_string(), right: 3, of: 5 }];
        let e1 = estimate(&a1, &bank, prior);

        let a2 = vec![
            Answer { item: "item_8".to_string(), right: 3, of: 5 },
            Answer { item: "item_9".to_string(), right: 4, of: 5 },
        ];
        let e2 = estimate(&a2, &bank, prior);

        assert!(e1.se < e0.se);
        assert!(e2.se < e1.se);
    }

    #[test]
    fn same_seed_gives_same_item() {
        let bank = sample_bank();
        let est = Estimate { theta: 3.5, se: 0.8 };
        let taken = vec![];

        let item1 = next_item(&bank, est, &taken, Task::Dictation, 42);
        let item2 = next_item(&bank, est, &taken, Task::Dictation, 42);

        assert_eq!(item1, item2);
    }
}
