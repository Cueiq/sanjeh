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

        whisper_rs::install_logging_hooks();
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
        params.set_suppress_nst(true);

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
                        if let Ok(token_bytes) = token.to_bytes() {
                            let bytes = token_bytes.to_vec();
                            let prob = token.token_probability();
                            all_tokens_owned.push((bytes, prob));
                        }
                    }
                }
            }
        }

        let token_refs: Vec<(&[u8], f32)> = all_tokens_owned.iter().map(|(b, p)| (b.as_slice(), *p)).collect();
        Ok(tokens_to_words(&token_refs))
    }
}

/// Groups individual Whisper tokens into whole words and calculates their probability.
///
/// Punctuation marks are attached to the preceding word. The final word probability
/// is the minimum probability of its constituent alphanumeric tokens (ignoring the
/// probabilities of standalone punctuation tokens or Whisper special tokens).
pub fn tokens_to_words(tokens: &[(&[u8], f32)]) -> Vec<HeardWord> {
    let mut words = Vec::new();
    let mut current_bytes = Vec::new();
    let mut current_p = f32::MAX;
    let mut has_alphanumeric_tokens = false;

    for (bytes, p) in tokens {
        let bytes = *bytes;
        let p = *p;

        // Skip Whisper's special tokens
        if bytes.starts_with(b"[_") || bytes.starts_with(b"<|") {
            continue;
        }

        if bytes.starts_with(b" ") {
            if !current_bytes.is_empty() {
                let word_str = String::from_utf8_lossy(&current_bytes).into_owned();
                words.push(HeardWord {
                    word: word_str,
                    p: if !has_alphanumeric_tokens {
                        None
                    } else {
                        Some(current_p as f64)
                    },
                });
            }
            current_bytes.clear();
            let mut i = 0;
            while i < bytes.len() && bytes[i] == b' ' {
                i += 1;
            }
            current_bytes.extend_from_slice(&bytes[i..]);

            current_p = f32::MAX;
            has_alphanumeric_tokens = false;
        } else {
            current_bytes.extend_from_slice(bytes);
        }

        if String::from_utf8_lossy(bytes).chars().any(|c| c.is_alphanumeric()) {
            current_p = current_p.min(p);
            has_alphanumeric_tokens = true;
        }
    }

    if !current_bytes.is_empty() {
        let word_str = String::from_utf8_lossy(&current_bytes).into_owned();
        words.push(HeardWord {
            word: word_str,
            p: if !has_alphanumeric_tokens {
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
    fn tokens_are_grouped_into_words_with_minimum_certainty() {
        let tokens: Vec<(&[u8], f32)> = vec![
            (b"[_BEG_]", 0.84),
            (b" hello", 0.9),
            (b" world", 0.8),
            (b"!", 0.2),
            (b" doing", 0.4),
            (b"?", 0.99),
            (b"[_TT_240]", 0.22),
            (b" Museum", 0.95),
            (b",", 0.61),
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
