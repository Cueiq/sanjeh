//! The weights a line is measured by: for each skill, a linear model of its
//! level on the CEFR scale and a logistic model of its worth, over the
//! line's features standardised by the means and spreads they were fitted on.
//! Plain JSON in `models/line/`, so the same file is used in Rust, in an app
//! and in a notebook.

use serde::{Deserialize, Serialize};

use super::Features;

/// `bias + Σ weight × (feature − mean) / sd`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinearModel {
    pub features: Vec<String>,
    pub mean: Vec<f64>,
    pub sd: Vec<f64>,
    pub weights: Vec<f64>,
    pub bias: f64,
}

impl LinearModel {
    /// The model's value for a line. A feature the line lacks counts as the
    /// mean: it moves nothing.
    pub fn value(&self, features: &Features) -> f64 {
        self.features.iter().enumerate().fold(self.bias, |sum, (at, name)| {
            let x = features.get(name).copied().unwrap_or(self.mean[at]);
            let sd = if self.sd[at] > 0.0 { self.sd[at] } else { 1.0 };
            sum + self.weights[at] * (x - self.mean[at]) / sd
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillModel {
    /// The line's level, on the CEFR scale (A1 = 1 … C2 = 6).
    pub level: LinearModel,
    /// The log-odds that it is worth practising.
    pub worth: LinearModel,
}

/// A version of the weights, with where it came from.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LineModel {
    pub name: String,
    /// What it was fitted on, and how.
    pub about: String,
    pub ear: SkillModel,
    pub mouth: SkillModel,
}

impl LineModel {
    pub fn from_json(text: &str) -> serde_json::Result<LineModel> {
        serde_json::from_str(text)
    }
}
