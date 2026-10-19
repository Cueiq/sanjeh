#[cfg(feature = "hear")]
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[cfg(feature = "hear")]
use crate::line::HeardWord;

#[cfg(feature = "hear")]
pub fn run_whisper_on_clip(
    audio_samples: &[f32],
    model_path: &str,
) -> Result<Vec<HeardWord>, String> {
    let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .map_err(|e| format!("Failed to load model: {}", e))?;

    let mut state = ctx
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

    let mut heard_words = Vec::new();

    for i in 0..num_segments {
        let num_tokens = state.full_n_tokens(i).unwrap_or(0);
        for j in 0..num_tokens {
            let text = state.full_get_token_text(i, j).unwrap_or_default();
            let prob = state.full_get_token_prob(i, j).unwrap_or(0.0);

            let word = text.trim().to_string();
            if !word.is_empty() && !word.starts_with('[') && word.chars().any(|c| c.is_alphanumeric()) {
                heard_words.push(HeardWord { word, p: Some(prob as f64) });
            }
        }
    }

    Ok(heard_words)
}