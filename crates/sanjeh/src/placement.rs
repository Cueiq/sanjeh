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

/// Why an estimate can't be made from what it was given.
#[derive(Debug, Clone, PartialEq)]
pub enum PlacementError {
    /// The prior's mean isn't finite, or its SD isn't positive and finite:
    /// every answer would weigh NaN and the level would be meaningless.
    BadPrior,
    /// An answer is for an item the bank doesn't have: a typo, or an answer
    /// from another version of the bank. Dropping it silently would lose it.
    UnknownItem(String),
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
        Cuts {
            between: [2.0, 3.0, 4.0, 5.0, 6.0],
        }
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

    /// How much an answer to this item tells about a learner of ability
    /// `theta`: a² · p · (1 − p), the Fisher information of one chance to be
    /// right. It is highest where the odds are even (p = ½). The words of a
    /// line are not counted here: `estimate` already counts them, as that
    /// many trials, and choosing by them would favour a long line that is too
    /// hard over a shorter one at the learner's level.
    #[inline]
    pub fn information(&self, theta: f64) -> f64 {
        let p = self.p(theta);
        let a = self.discrimination;
        a * a * p * (1.0 - p)
    }
}

/// EAP (Expected A Posteriori) estimation over a grid of theta.
/// Grid of 121 points from -1.0 to 8.0 on the CEFR-lined scale with normal prior.
///
/// A prior whose mean isn't finite, or whose SD isn't positive and finite,
/// is refused with `PlacementError::BadPrior`. An answer for an item the
/// bank doesn't have is refused with `PlacementError::UnknownItem`. An
/// answer with more words right than asked counts as all right.
pub fn estimate(answers: &[Answer], bank: &Bank, prior: Prior) -> Result<Estimate, PlacementError> {
    const GRID_SIZE: usize = 121;
    const MIN_THETA: f64 = -1.0;
    const MAX_THETA: f64 = 8.0;
    const STEP: f64 = (MAX_THETA - MIN_THETA) / ((GRID_SIZE - 1) as f64);

    let grid_thetas: [f64; GRID_SIZE] = std::array::from_fn(|i| MIN_THETA + (i as f64) * STEP);

    if !prior.mean.is_finite() || !(prior.sd.is_finite() && prior.sd > 0.0) {
        return Err(PlacementError::BadPrior);
    }

    // Each answer's item is looked up once, here, and not again for every
    // point of the grid: with a big bank that was most of the time spent.
    let mut counted: Vec<(&Item, f64, f64)> = Vec::with_capacity(answers.len());
    for answer in answers {
        let item = bank
            .get(&answer.item)
            .ok_or_else(|| PlacementError::UnknownItem(answer.item.clone()))?;
        let right = answer.right.min(answer.of);
        counted.push((item, right as f64, (answer.of - right) as f64));
    }

    let mut log_posteriors = [0.0; GRID_SIZE];
    let mut max_log_post = f64::NEG_INFINITY;

    for (i, &theta) in grid_thetas.iter().enumerate() {
        let z = (theta - prior.mean) / prior.sd;
        let log_prior = -0.5 * z * z;

        let mut log_lik = 0.0;
        for &(item, right, wrong) in &counted {
            let p = item.p(theta).clamp(1e-12, 1.0 - 1e-12);
            log_lik += right * p.ln() + wrong * (1.0 - p).ln();
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

    let mut mean_theta = 0.0;
    for (i, &theta) in grid_thetas.iter().enumerate() {
        mean_theta += theta * (weights[i] / total_weight);
    }

    let mut var_theta = 0.0;
    for (i, &theta) in grid_thetas.iter().enumerate() {
        let diff = theta - mean_theta;
        var_theta += diff * diff * (weights[i] / total_weight);
    }

    Ok(Estimate {
        theta: mean_theta,
        se: var_theta.max(1e-6).sqrt(),
    })
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

/// How many of the most informative items `next_item` draws from. The design
/// says "the best few" and doesn't fix a number: three is a starting value,
/// small enough that the test stays close to the most informative item and
/// big enough that learners aren't all given the same one. Simulation (#13)
/// is meant to set it, as it sets the stopping rule.
const BEST_FEW: usize = 3;

/// The most informative items at the current theta, one drawn from the best few
/// with the caller's seed.
///
/// How to use the seed: give the same `seed` for the whole test. The number of
/// items already taken is mixed into it here, so each item gets a fresh draw
/// and the same seed still gives the same test, item by item. Another seed
/// gives another test.
pub fn next_item<'a>(bank: &'a Bank, now: Estimate, taken: &[String], part: Task, seed: u64) -> Option<&'a Item> {
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

    let pool_size = candidates.len().min(BEST_FEW);
    let draw = seed.wrapping_add((taken.len() as u64).wrapping_mul(0x9e3779b97f4a7c15));
    let chosen_idx = pseudo_random_index(draw, pool_size);
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

/// Keep an ability estimate up to date as the learner practises: nudge theta
/// towards how they did on this item, and narrow how far off it may be.
pub fn update(now: &Estimate, item: &Item, answer: &Answer) -> Estimate {
    if answer.of == 0 {
        return *now;
    }

    let p = item.p(now.theta);
    let actual = (answer.right as f64) / (answer.of as f64);
    let error = actual - p;

    // How far theta moves: big steps early on when se is wide, smaller steps
    // once we know more.
    let k = now.se.min(1.0) * item.discrimination * 0.4;
    let new_theta = now.theta + k * error;

    // Each answered line gives a little more certainty.
    let info = item.information(now.theta);
    let se_reduction = 0.01 * (1.0 + info).ln().min(0.05);
    let new_se = (now.se - se_reduction).max(0.15);

    Estimate {
        theta: new_theta,
        se: new_se,
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

        let initial = estimate(&[], &bank, prior).unwrap();
        let answers = vec![
            Answer {
                item: "item_5".to_string(),
                right: 5,
                of: 5,
            },
            Answer {
                item: "item_8".to_string(),
                right: 5,
                of: 5,
            },
            Answer {
                item: "item_12".to_string(),
                right: 5,
                of: 5,
            },
        ];
        let after = estimate(&answers, &bank, prior).unwrap();

        assert!(after.theta > initial.theta);
    }

    #[test]
    fn learner_who_gets_everything_wrong_falls() {
        let bank = sample_bank();
        let prior = Prior::centred_on(3.0);

        let initial = estimate(&[], &bank, prior).unwrap();
        let answers = vec![
            Answer {
                item: "item_5".to_string(),
                right: 0,
                of: 5,
            },
            Answer {
                item: "item_8".to_string(),
                right: 0,
                of: 5,
            },
            Answer {
                item: "item_12".to_string(),
                right: 0,
                of: 5,
            },
        ];
        let after = estimate(&answers, &bank, prior).unwrap();

        assert!(after.theta < initial.theta);
    }

    #[test]
    fn se_shrinks_with_each_answer() {
        let bank = sample_bank();
        let prior = Prior::centred_on(3.0);

        let e0 = estimate(&[], &bank, prior).unwrap();
        let a1 = vec![Answer {
            item: "item_8".to_string(),
            right: 3,
            of: 5,
        }];
        let e1 = estimate(&a1, &bank, prior).unwrap();

        let a2 = vec![
            Answer {
                item: "item_8".to_string(),
                right: 3,
                of: 5,
            },
            Answer {
                item: "item_9".to_string(),
                right: 4,
                of: 5,
            },
        ];
        let e2 = estimate(&a2, &bank, prior).unwrap();

        assert!(e1.se < e0.se);
        assert!(e2.se < e1.se);
    }

    /// Runs a whole test on `bank`: after each answer the estimate is made
    /// again and the next item is asked for. The learner answers every word of
    /// an item right or every word wrong, by `learner`. Returns the ids of the
    /// items given, in order, and the last estimate.
    fn run_test(bank: &Bank, seed: u64, learner: impl Fn(&Item) -> bool, count: usize) -> (Vec<String>, Estimate) {
        let prior = Prior::centred_on(3.0);
        let mut answers: Vec<Answer> = Vec::new();
        let mut given: Vec<String> = Vec::new();
        for _ in 0..count {
            let now = estimate(&answers, bank, prior).unwrap();
            let Some(item) = next_item(bank, now, &given, Task::Dictation, seed) else {
                break;
            };
            answers.push(Answer {
                item: item.id.clone(),
                right: if learner(item) { item.words } else { 0 },
                of: item.words,
            });
            given.push(item.id.clone());
        }
        let last = estimate(&answers, bank, prior).unwrap();
        (given, last)
    }

    #[test]
    fn the_same_seed_gives_the_same_whole_test() {
        let bank = sample_bank();
        let learner = |item: &Item| item.difficulty < 4.0;
        let first = run_test(&bank, 42, learner, 10).0;
        let second = run_test(&bank, 42, learner, 10).0;
        assert_eq!(first, second);
    }

    #[test]
    fn other_seeds_give_other_tests() {
        let bank = sample_bank();
        let learner = |item: &Item| item.difficulty < 4.0;
        let mut tests = std::collections::HashSet::new();
        for seed in 0..100 {
            tests.insert(run_test(&bank, seed, learner, 8).0);
        }
        // With one draw for the whole test there were only three possible tests.
        assert!(tests.len() > 3, "only {} different tests from 100 seeds", tests.len());
    }

    #[test]
    fn every_answer_right_ends_at_the_top_and_every_answer_wrong_at_the_bottom() {
        let bank = sample_bank();
        let (_, top) = run_test(&bank, 1, |_| true, 15);
        let (_, bottom) = run_test(&bank, 1, |_| false, 15);
        assert_eq!(level(top.theta, Cuts::default()), Level::C2);
        assert_eq!(level(bottom.theta, Cuts::default()), Level::A1);
    }

    #[test]
    fn next_item_skips_taken_items_and_other_parts() {
        let mut bank = sample_bank();
        bank.items.push(Item {
            id: "say_1".to_string(),
            skill: Skill::Mouth,
            task: Task::Shadowing,
            difficulty: 3.0,
            discrimination: 1.0,
            words: 5,
        });
        let now = Estimate { theta: 3.0, se: 0.8 };

        // Only the part asked for is given.
        for seed in 0..50 {
            let item = next_item(&bank, now, &[], Task::Dictation, seed).unwrap();
            assert_eq!(item.task, Task::Dictation);
        }

        // An item already taken is not given again.
        let first = next_item(&bank, now, &[], Task::Dictation, 7).unwrap().id.clone();
        let taken = vec![first.clone()];
        for seed in 0..50 {
            let item = next_item(&bank, now, &taken, Task::Dictation, seed).unwrap();
            assert_ne!(item.id, first);
        }

        // Nothing is left when every item of the part is taken, or the part has none.
        let all: Vec<String> = bank
            .items
            .iter()
            .filter(|item| item.task == Task::Dictation)
            .map(|item| item.id.clone())
            .collect();
        assert!(next_item(&bank, now, &all, Task::Dictation, 1).is_none());
        assert!(next_item(&bank, now, &[], Task::Vocabulary, 1).is_none());
    }

    #[test]
    fn the_item_given_is_one_of_the_best_three() {
        let bank = sample_bank();
        let now = Estimate { theta: 3.0, se: 0.8 };
        let mut by_information: Vec<&Item> = bank.items.iter().collect();
        by_information.sort_by(|a, b| b.information(3.0).partial_cmp(&a.information(3.0)).unwrap());
        let best: Vec<&str> = by_information.iter().take(3).map(|item| item.id.as_str()).collect();
        for seed in 0..50 {
            let item = next_item(&bank, now, &[], Task::Dictation, seed).unwrap();
            assert!(best.contains(&item.id.as_str()));
        }
    }

    #[test]
    fn finished_waits_for_the_fewest_and_stops_at_the_most() {
        let rule = StopRule {
            se: 0.35,
            fewest: 4,
            most: 10,
        };
        let sure = Estimate { theta: 3.0, se: 0.1 };
        let unsure = Estimate { theta: 3.0, se: 0.9 };
        assert!(
            !finished(sure, 3, rule),
            "a sure estimate must still wait for the fewest"
        );
        assert!(finished(sure, 4, rule));
        assert!(!finished(unsure, 9, rule));
        assert!(
            finished(unsure, 10, rule),
            "the most must stop an estimate still unsure"
        );
    }

    #[test]
    fn a_level_begins_on_its_cut() {
        let cuts = Cuts::default();
        assert_eq!(level(2.0, cuts), Level::A2);
        assert_eq!(level(1.999, cuts), Level::A1);
        assert_eq!(level(6.0, cuts), Level::C2);
        assert_eq!(level(5.999, cuts), Level::C1);
        assert_eq!(level(0.5, cuts), Level::A1);
        assert_eq!(level(-3.0, cuts), Level::A1);
        assert_eq!(level(9.0, cuts), Level::C2);
    }

    #[test]
    fn a_bad_prior_is_refused() {
        let bank = sample_bank();
        let bad = [
            Prior { mean: 3.0, sd: 0.0 },
            Prior { mean: 3.0, sd: -1.0 },
            Prior {
                mean: 3.0,
                sd: f64::NAN,
            },
            Prior {
                mean: 3.0,
                sd: f64::INFINITY,
            },
            Prior {
                mean: f64::NAN,
                sd: 1.2,
            },
            Prior {
                mean: f64::INFINITY,
                sd: 1.2,
            },
        ];
        for prior in bad {
            assert_eq!(estimate(&[], &bank, prior), Err(PlacementError::BadPrior));
        }
    }

    #[test]
    fn an_answer_for_an_unknown_item_is_refused() {
        let bank = sample_bank();
        let answers = vec![Answer {
            item: "item_99".to_string(),
            right: 1,
            of: 1,
        }];
        assert_eq!(
            estimate(&answers, &bank, Prior::centred_on(3.0)),
            Err(PlacementError::UnknownItem("item_99".to_string()))
        );
    }

    #[test]
    fn more_words_right_than_asked_count_as_all_right() {
        let bank = sample_bank();
        let prior = Prior::centred_on(3.0);
        let too_many = vec![Answer {
            item: "item_8".to_string(),
            right: 50,
            of: 5,
        }];
        let all_right = vec![Answer {
            item: "item_8".to_string(),
            right: 5,
            of: 5,
        }];
        assert_eq!(
            estimate(&too_many, &bank, prior).unwrap(),
            estimate(&all_right, &bank, prior).unwrap()
        );
    }

    #[test]
    fn update_climbs_on_better_than_expected() {
        let item = Item {
            id: "i1".to_string(),
            skill: Skill::Ear,
            task: Task::Dictation,
            difficulty: 3.0,
            discrimination: 1.0,
            words: 5,
        };
        // A learner at 3.0 getting all five words right should climb.
        let now = Estimate { theta: 3.0, se: 0.5 };
        let ans = Answer {
            item: "i1".to_string(),
            right: 5,
            of: 5,
        };
        let updated = update(&now, &item, &ans);

        assert!(updated.theta > now.theta);
        assert!(updated.se < now.se);
    }

    #[test]
    fn update_falls_on_worse_than_expected() {
        let item = Item {
            id: "i1".to_string(),
            skill: Skill::Ear,
            task: Task::Dictation,
            difficulty: 3.0,
            discrimination: 1.0,
            words: 5,
        };
        // Getting zero words right should bring the estimate down.
        let now = Estimate { theta: 3.0, se: 0.5 };
        let ans = Answer {
            item: "i1".to_string(),
            right: 0,
            of: 5,
        };
        let updated = update(&now, &item, &ans);

        assert!(updated.theta < now.theta);
        assert!(updated.se < now.se);
    }

    #[test]
    fn update_handles_zero_total_words() {
        let item = Item {
            id: "i1".to_string(),
            skill: Skill::Ear,
            task: Task::Dictation,
            difficulty: 3.0,
            discrimination: 1.0,
            words: 0,
        };
        let now = Estimate { theta: 3.0, se: 0.5 };
        let ans = Answer {
            item: "i1".to_string(),
            right: 0,
            of: 0,
        };
        let updated = update(&now, &item, &ans);

        assert_eq!(updated, now);
    }
}
