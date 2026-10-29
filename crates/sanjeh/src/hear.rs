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

        let token_refs: Vec<(&str, f32)> = all_tokens.iter().map(|(t, p)| (t.as_str(), *p)).collect();
        Ok(tokens_to_words(&token_refs))
    }
}

/// Joins sub-word tokens into full words and calculates the minimum probability.
pub fn tokens_to_words(tokens: &[(&str, f32)]) -> Vec<HeardWord> {
    let mut heard_words = Vec::new();
    let mut current_word = String::new();
    let mut current_min_p = 1.0f32;
    let mut has_tokens = false;

    for &(text, prob) in tokens {
        if text.is_empty() || text.starts_with('[') {
            continue;
        }

        let starts_with_space = text.chars().next().is_some_and(|c| c.is_whitespace());

        if starts_with_space && !current_word.is_empty() {
            let word_str = current_word.trim().to_string();
            if !word_str.is_empty() && word_str.chars().any(|c| c.is_alphanumeric()) {
                let p_val = if has_tokens { Some(current_min_p as f64) } else { None };
                heard_words.push(HeardWord {
                    word: word_str,
                    p: p_val,
                });
            }
            current_word = text.to_string();
            current_min_p = prob;
            has_tokens = true;
        } else {
            current_word.push_str(text);
            if has_tokens {
                current_min_p = current_min_p.min(prob);
            } else {
                current_min_p = prob;
                has_tokens = true;
            }
        }
    }

    let word_str = current_word.trim().to_string();
    if !word_str.is_empty() && word_str.chars().any(|c| c.is_alphanumeric()) {
        let p_val = if has_tokens { Some(current_min_p as f64) } else { None };
        heard_words.push(HeardWord {
            word: word_str,
            p: p_val,
        });
    }

    heard_words
}

/// Reads a WAV file and strictly enforces 16kHz, mono, and 16-bit/32-bit format.
pub fn read_audio_for_whisper(path: &str) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| format!("Failed to open WAV file: {}", e))?;
    let spec = reader.spec();

    if spec.channels != 1 {
        return Err(format!(
            "Unsupported audio: expected 1 channel (mono), got {}. Please convert the file.",
            spec.channels
        ));
    }

    if spec.sample_rate != 16000 {
        return Err(format!(
            "Unsupported audio: expected 16000 Hz sample rate, got {} Hz. Please convert the file.",
            spec.sample_rate
        ));
    }

    let mut audio_samples = Vec::new();

    match spec.sample_format {
        hound::SampleFormat::Int => {
            if spec.bits_per_sample != 16 {
                return Err(format!(
                    "Unsupported audio: expected 16-bit integer samples, got {}-bit.",
                    spec.bits_per_sample
                ));
            }
            for sample in reader.samples::<i16>() {
                let s = sample.map_err(|e| format!("Failed to read audio sample: {}", e))?;
                audio_samples.push(s as f32 / 32768.0);
            }
        }
        hound::SampleFormat::Float => {
            if spec.bits_per_sample != 32 {
                return Err(format!(
                    "Unsupported audio: expected 32-bit float samples, got {}-bit.",
                    spec.bits_per_sample
                ));
            }
            for sample in reader.samples::<f32>() {
                let s = sample.map_err(|e| format!("Failed to read audio sample: {}", e))?;
                audio_samples.push(s);
            }
        }
    }

    Ok(audio_samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokens_to_words() {
        let tokens = vec![(" civil", 0.9), ("ization", 0.4), (" test", 0.95)];
        let words = tokens_to_words(&tokens);
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].word, "civilization");
        assert_eq!(words[0].p, Some(0.4)); // Minimum probability check
        assert_eq!(words[1].word, "test");
        assert_eq!(words[1].p, Some(0.95));
    }
}
