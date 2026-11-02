//! Decoding audio clips into words and their certainty using `whisper-rs`.
//!
//! This module provides the `WhisperModel` for transcribing 16 kHz mono WAV
//! audio files. It extracts the words and their token probabilities to measure
//! how clearly each word was spoken.

use crate::line::HeardWord;
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// A loaded Whisper model ready to transcribe audio.
pub struct WhisperModel {
    ctx: WhisperContext,
}

impl WhisperModel {
    /// Loads a GGML Whisper model from the given file path.
    pub fn load(model_path: &str) -> Result<Self, String> {
        if !Path::new(model_path).exists() {
            return Err(format!(
                "Model file '{}' not found. Please check the path, use --model <file>, or download ggml-tiny.en.bin from whisper.cpp's models into models/.",
                model_path
            ));
        }

        let ctx_params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(model_path, ctx_params)
            .map_err(|e| format!("Failed to create a new whisper context: {}", e))?;
        Ok(Self { ctx })
    }

    /// Transcribes a slice of 16 kHz mono audio samples.
    ///
    /// Returns a list of `HeardWord`s containing the text and certainty of each word.
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

        let num_segments = state.full_n_segments();
        let mut all_tokens_owned = Vec::new();

        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                let num_tokens = segment.n_tokens();
                for j in 0..num_tokens {
                    if let Some(token) = segment.get_token(j) {
                        if let Ok(text_cow) = token.to_str_lossy() {
                            let text = text_cow.to_string();
                            let prob = token.token_probability();
                            all_tokens_owned.push((text, prob));
                        }
                    }
                }
            }
        }

        let token_refs: Vec<(&str, f32)> = all_tokens_owned.iter().map(|(t, p)| (t.as_str(), *p)).collect();
        Ok(tokens_to_words(&token_refs))
    }
}

/// Groups individual Whisper tokens into whole words and calculates their probability.
///
/// Punctuation marks are attached to the preceding word. The final word probability
/// is the minimum probability of its constituent alphanumeric tokens (ignoring the
/// probabilities of standalone punctuation tokens or Whisper special tokens).
pub fn tokens_to_words(tokens: &[(&str, f32)]) -> Vec<HeardWord> {
    let mut words = Vec::new();
    let mut current_word = String::new();
    let mut current_p = f32::MAX;
    let mut has_alphanumeric_tokens = false;

    for (text, p) in tokens {
        let text = *text;
        let p = *p;

        // Skip Whisper's special tokens like [_BEG_] or [_TT_205]
        if text.trim_start().starts_with('[') {
            continue;
        }

        if text.starts_with(' ') {
            if !current_word.is_empty() {
                words.push(HeardWord {
                    word: current_word.clone(),
                    p: if current_p == f32::MAX {
                        None
                    } else {
                        Some(current_p as f64)
                    },
                });
            }
            current_word = text.trim_start().to_string();
            current_p = f32::MAX;
            has_alphanumeric_tokens = false;
        } else {
            current_word.push_str(text);
        }

        if text.chars().any(|c| c.is_alphanumeric()) {
            current_p = current_p.min(p);
            has_alphanumeric_tokens = true;
        }
    }

    if !current_word.is_empty() {
        if !has_alphanumeric_tokens && current_p == f32::MAX {
            if let Some(last) = tokens.last() {
                current_p = last.1;
            }
        }
        words.push(HeardWord {
            word: current_word,
            p: if current_p == f32::MAX {
                None
            } else {
                Some(current_p as f64)
            },
        });
    }

    words
}

/// Reads a WAV file from the given path and converts it to a slice of f32 samples.
///
/// Ensures the audio is exactly 1 channels (mono) and 16,000 Hz, which is required
/// by the Whisper model. It supports both 16-bit integer and 32-bit float formats.
pub fn read_audio_for_whisper(path: &str) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| format!("Failed to open WAV: {}", e))?;
    let spec = reader.spec();

    if spec.channels != 1 {
        return Err(format!("Expected mono audio, got {} channels", spec.channels));
    }
    if spec.sample_rate != 16000 {
        return Err(format!("Expected 16000 Hz sample rate, got {} Hz", spec.sample_rate));
    }

    let samples = match spec.sample_format {
        hound::SampleFormat::Int => {
            if spec.bits_per_sample == 16 {
                let result: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
                let raw = result.map_err(|e| format!("Failed to read samples: {}", e))?;
                raw.into_iter().map(|s| s as f32 / 32768.0).collect()
            } else {
                return Err(format!("Unsupported integer bit depth: {}", spec.bits_per_sample));
            }
        }
        hound::SampleFormat::Float => {
            if spec.bits_per_sample == 32 {
                let result: Result<Vec<f32>, _> = reader.samples::<f32>().collect();
                result.map_err(|e| format!("Failed to read samples: {}", e))?
            } else {
                return Err(format!("Unsupported float bit depth: {}", spec.bits_per_sample));
            }
        }
    };

    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokens_to_words() {
        let tokens: Vec<(&str, f32)> = vec![
            ("[_BEG_]", 0.84), // Should be skipped completely
            (" hello", 0.9),
            (" world", 0.8),
            ("!", 0.2), // Ignored probability
            (" doing", 0.4),
            ("?", 0.99),         // Ignored probability
            ("[_TT_240]", 0.22), // Should be skipped completely
            (" Museum", 0.95),
            (",", 0.61), // Ignored probability
        ];
        let words = tokens_to_words(&tokens);

        assert_eq!(words.len(), 4);

        assert_eq!(words[0].word, "hello");
        assert_eq!(words[0].p, Some(0.9_f32 as f64));

        assert_eq!(words[1].word, "world!");
        assert_eq!(words[1].p, Some(0.8_f32 as f64));

        assert_eq!(words[2].word, "doing?");
        assert_eq!(words[2].p, Some(0.4_f32 as f64));

        assert_eq!(words[3].word, "Museum,");
        assert_eq!(words[3].p, Some(0.95_f32 as f64));
    }
}
