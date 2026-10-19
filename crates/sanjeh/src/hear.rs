#![cfg(feature = "hear")]
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::line::HeardWord;

pub struct WhisperModel {
    ctx: WhisperContext,
}

impl WhisperModel {
    /// Load the Whisper model once so it can be reused across multiple clips.
    pub fn load(model_path: &str) -> Result<Self, String> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| format!("Failed to load model: {}", e))?;
        Ok(Self { ctx })
    }

    /// Transcribe audio samples and return recognized words with probabilities.
    pub fn transcribe(&self, audio_samples: &[f32]) -> Result<Vec<HeardWord>, String> {
        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| format!("Failed to create state: {}", e))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_progress(false);
        params.set_print_special(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_initial_prompt("");

        state
            .full(params, audio_samples)
            .map_err(|e| format!("Failed to run model: {}", e))?;

        let num_segments = state
            .full_n_segments()
            .map_err(|e| format!("Failed to get segments: {}", e))?;

        let mut all_tokens = Vec::new();

        for i in 0..num_segments {
            let num_tokens = state.full_n_tokens(i).unwrap_or(0);
            for j in 0..num_tokens {
                let text = state.full_get_token_text(i, j).unwrap_or_default();
                let prob = state.full_get_token_prob(i, j).unwrap_or(0.0);
                all_tokens.push((text, prob));
            }
        }

        // Borrow slice references for token-to-word joining
        let token_refs: Vec<(&str, f32)> = all_tokens.iter().map(|(t, p)| (t.as_str(), *p)).collect();

        Ok(tokens_to_words(&token_refs))
    }
}

/// Joins sub-word tokens into full words and calculates average probability.
pub fn tokens_to_words(tokens: &[(&str, f32)]) -> Vec<HeardWord> {
    let mut heard_words = Vec::new();
    let mut current_word = String::new();
    let mut current_probs = Vec::new();

    for &(text, prob) in tokens {
        if text.is_empty() || text.starts_with('[') {
            continue;
        }

        // In Whisper, tokens starting with whitespace mark the start of a new word.
        let starts_with_space = text.chars().next().is_some_and(|c| c.is_whitespace());

        if starts_with_space && !current_word.is_empty() {
            let word_str = current_word.trim().to_string();
            if !word_str.is_empty() && word_str.chars().any(|c| c.is_alphanumeric()) {
                let avg_p = if current_probs.is_empty() {
                    None
                } else {
                    Some(current_probs.iter().sum::<f64>() / current_probs.len() as f64)
                };
                heard_words.push(HeardWord {
                    word: word_str,
                    p: avg_p,
                });
            }
            current_word = text.to_string();
            current_probs = vec![prob as f64];
        } else {
            current_word.push_str(text);
            current_probs.push(prob as f64);
        }
    }

    let word_str = current_word.trim().to_string();
    if !word_str.is_empty() && word_str.chars().any(|c| c.is_alphanumeric()) {
        let avg_p = if current_probs.is_empty() {
            None
        } else {
            Some(current_probs.iter().sum::<f64>() / current_probs.len() as f64)
        };
        heard_words.push(HeardWord {
            word: word_str,
            p: avg_p,
        });
    }

    heard_words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokens_to_words() {
        // Test merging subword tokens like "civil" + "ization"
        let tokens = vec![(" civil", 0.9), ("ization", 0.85), (" test", 0.95)];
        let words = tokens_to_words(&tokens);
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].word, "civilization");
        assert_eq!(words[1].word, "test");
    }
}
