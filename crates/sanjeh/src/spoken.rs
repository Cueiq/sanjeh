use serde::{Deserialize, Serialize};

/// Represents a recognized word with precise timing information (in milliseconds).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimedWord {
    pub word: String,
    pub p: Option<f64>,
    pub start_ms: f64,
    pub end_ms: f64,
}

/// Detailed evaluation score and reasons for a spoken line attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpokenEvaluation {
    pub overall_score: f64,
    pub rhythm: RhythmScore,
    pub stress: StressScore,
    pub intonation: IntonationScore,
    pub sounds: SoundScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RhythmScore {
    pub score: f64,
    pub speech_rate_wpm: f64,
    pub lag_ms: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressScore {
    pub score: f64,
    pub avg_energy: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntonationScore {
    pub score: f64,
    pub dtw_distance: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhonemeScore {
    pub expected: String,
    pub recognized: String,
    pub gop_score: f64,
    pub flagged_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundScore {
    pub score: f64,
    pub phoneme_scores: Vec<PhonemeScore>,
    pub reasons: Vec<String>,
}

impl SpokenEvaluation {
    /// Evaluates rhythm, stress, intonation, and pronunciation sounds (GOP).
    pub fn evaluate(
        reference_words: &[TimedWord],
        user_words: &[TimedWord],
        reference_pitches: &[f64],
        user_pitches: &[f64],
        user_energies: &[f64],
        expected_phonemes: &[String],
        recognized_phonemes: &[String],
    ) -> Self {
        let rhythm = evaluate_rhythm(reference_words, user_words);
        let stress = evaluate_stress(user_words, user_energies);
        let intonation = evaluate_intonation(reference_pitches, user_pitches);
        let sounds = evaluate_sounds(expected_phonemes, recognized_phonemes);

        // Balanced overall score combination across all four core pillars
        let overall_score =
            (rhythm.score * 0.25) + (stress.score * 0.25) + (intonation.score * 0.25) + (sounds.score * 0.25);

        Self {
            overall_score,
            rhythm,
            stress,
            intonation,
            sounds,
        }
    }
}

fn evaluate_rhythm(reference: &[TimedWord], user: &[TimedWord]) -> RhythmScore {
    let mut reasons = Vec::new();

    if user.is_empty() {
        reasons.push("No words detected in user recording.".to_string());
        return RhythmScore {
            score: 0.0,
            speech_rate_wpm: 0.0,
            lag_ms: 0.0,
            reasons,
        };
    }

    let user_start: f64 = user.first().map(|w| w.start_ms).unwrap_or(0.0);
    let user_end: f64 = user.last().map(|w| w.end_ms).unwrap_or(0.0);
    let duration_min: f64 = (user_end - user_start).max(100.0) / 60_000.0;
    let speech_rate_wpm: f64 = (user.len() as f64) / duration_min;

    let mut total_lag: f64 = 0.0;
    let lag_count = reference.len().min(user.len());

    if lag_count > 0 {
        for i in 0..lag_count {
            let diff: f64 = user[i].start_ms - reference[i].start_ms;
            total_lag += diff;
        }
    }
    let lag_ms: f64 = if lag_count > 0 {
        total_lag / (lag_count as f64)
    } else {
        0.0
    };

    let mut score: f64 = 90.0;
    if speech_rate_wpm < 80.0 {
        score -= 15.0;
        reasons.push(format!("Speech pace is a bit slow ({:.1} WPM).", speech_rate_wpm));
    } else if speech_rate_wpm > 180.0 {
        score -= 15.0;
        reasons.push(format!("Speech pace is too fast ({:.1} WPM).", speech_rate_wpm));
    } else {
        reasons.push(format!("Good speech pacing ({:.1} WPM).", speech_rate_wpm));
    }

    if lag_ms.abs() > 500.0 {
        score -= 10.0;
        reasons.push(format!("Notable timing lag of {:.0} ms compared to reference.", lag_ms));
    } else {
        reasons.push("Timing alignment with reference is well synchronized.".to_string());
    }

    let score = score.clamp(0.0f64, 100.0f64);

    RhythmScore {
        score,
        speech_rate_wpm,
        lag_ms,
        reasons,
    }
}

fn evaluate_stress(user_words: &[TimedWord], user_energies: &[f64]) -> StressScore {
    let mut reasons = Vec::new();
    if user_words.is_empty() || user_energies.is_empty() {
        reasons.push("No words or energy profiles available for stress analysis.".to_string());
        return StressScore {
            score: 0.0,
            avg_energy: 0.0,
            reasons,
        };
    }

    let sum_energy: f64 = user_energies.iter().sum();
    let avg_energy = sum_energy / (user_energies.len() as f64);

    let mut score: f64 = 85.0;
    if avg_energy < 0.1 {
        score -= 15.0;
        reasons.push("Voice projection is too soft; syllable stress lacks adequate energy.".to_string());
    } else if avg_energy > 0.9 {
        score -= 10.0;
        reasons.push("Voice energy is excessively high, leading to potential clipping.".to_string());
    } else {
        reasons.push(format!(
            "Syllable stress and energy distribution are well balanced (avg energy: {:.2}).",
            avg_energy
        ));
    }

    let avg_duration = user_words.iter().map(|w| w.end_ms - w.start_ms).sum::<f64>() / (user_words.len() as f64);

    if avg_duration < 100.0 {
        score -= 10.0;
        reasons.push("Words appear rushed, limiting proper duration for lexical stress.".to_string());
    } else {
        reasons.push("Word durations provide natural breathing room for lexical accents.".to_string());
    }

    let score = score.clamp(0.0f64, 100.0f64);

    StressScore {
        score,
        avg_energy,
        reasons,
    }
}

/// Converts frequency in Hertz to semitones from the speaker's median pitch.
pub fn hz_to_semitones(hz: f64, median_hz: f64) -> f64 {
    if hz <= 0.0 || median_hz <= 0.0 {
        return 0.0;
    }
    12.0 * (hz / median_hz).log2()
}

/// Computes Dynamic Time Warping (DTW) distance between two normalized pitch contours.
pub fn compute_dtw(seq1: &[f64], seq2: &[f64]) -> f64 {
    let n = seq1.len();
    let m = seq2.len();
    if n == 0 || m == 0 {
        return f64::MAX;
    }
    let mut dtw = vec![vec![f64::MAX; m + 1]; n + 1];
    dtw[0][0] = 0.0;

    for i in 1..=n {
        for j in 1..=m {
            let cost = (seq1[i - 1] - seq2[j - 1]).abs();
            let min_prev = dtw[i - 1][j].min(dtw[i][j - 1]).min(dtw[i - 1][j - 1]);
            dtw[i][j] = cost + min_prev;
        }
    }
    dtw[n][m] / ((n + m) as f64)
}

fn evaluate_intonation(reference_pitches: &[f64], user_pitches: &[f64]) -> IntonationScore {
    let mut reasons = Vec::new();
    if user_pitches.is_empty() || reference_pitches.is_empty() {
        reasons.push("Insufficient pitch contour data for intonation analysis.".to_string());
        return IntonationScore {
            score: 0.0,
            dtw_distance: f64::MAX,
            reasons,
        };
    }

    let mut ref_sorted = reference_pitches.to_vec();
    ref_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let ref_median = ref_sorted[ref_sorted.len() / 2].max(1.0);

    let mut user_sorted = user_pitches.to_vec();
    user_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let user_median = user_sorted[user_sorted.len() / 2].max(1.0);

    let ref_semitones: Vec<f64> = reference_pitches
        .iter()
        .map(|&h| hz_to_semitones(h, ref_median))
        .collect();
    let user_semitones: Vec<f64> = user_pitches.iter().map(|&h| hz_to_semitones(h, user_median)).collect();

    let dtw_distance = compute_dtw(&ref_semitones, &user_semitones);
    let score = (100.0 - (dtw_distance * 10.0)).clamp(0.0f64, 100.0f64);

    if dtw_distance < 2.0 {
        reasons.push("Intonation contour closely matches the reference actor.".to_string());
    } else if dtw_distance < 5.0 {
        reasons.push("Intonation contour moderately matches with minor pitch deviations.".to_string());
    } else {
        reasons.push("Notable pitch contour divergence from reference actor.".to_string());
    }

    IntonationScore {
        score,
        dtw_distance,
        reasons,
    }
}

fn evaluate_sounds(expected_phonemes: &[String], recognized_phonemes: &[String]) -> SoundScore {
    let mut reasons = Vec::new();
    if expected_phonemes.is_empty() || recognized_phonemes.is_empty() {
        reasons.push("No phoneme data available for pronunciation analysis.".to_string());
        return SoundScore {
            score: 0.0,
            phoneme_scores: vec![],
            reasons,
        };
    }

    let mut phoneme_scores = Vec::new();
    let mut total_gop: f64 = 0.0;
    let count = expected_phonemes.len().min(recognized_phonemes.len());

    for i in 0..count {
        let exp = &expected_phonemes[i];
        let rec = &recognized_phonemes[i];

        let mut gop: f64 = 90.0;
        let mut flagged_error = None;

        if exp != rec {
            gop -= 40.0;
            // Check common non-native learner errors
            if (exp == "w" && rec == "v") || (exp == "W" && rec == "V") {
                flagged_error = Some("Substituted 'v' for 'w'".to_string());
                reasons.push(format!(
                    "Phoneme error: Used 'v' instead of expected 'w' at position {}.",
                    i
                ));
            } else if (exp == "TH" || exp == "DH") && (rec == "t" || rec == "s" || rec == "d" || rec == "z") {
                flagged_error = Some("Substituted stop/sibilant for dental fricative (th)".to_string());
                reasons.push(format!(
                    "Phoneme error: Used '{}' instead of expected '{}' (th sound).",
                    rec, exp
                ));
            } else {
                flagged_error = Some(format!("Phoneme mismatch: expected '{}', got '{}'", exp, rec));
                reasons.push(format!(
                    "Phoneme mismatch at position {}: expected '{}', got '{}'.",
                    i, exp, rec
                ));
            }
        } else {
            reasons.push(format!("Phoneme '{}' correctly pronounced.", exp));
        }

        gop = gop.clamp(0.0f64, 100.0f64);
        total_gop += gop;

        phoneme_scores.push(PhonemeScore {
            expected: exp.clone(),
            recognized: rec.clone(),
            gop_score: gop,
            flagged_error,
        });
    }

    let score = if count > 0 { total_gop / (count as f64) } else { 0.0 };

    SoundScore {
        score,
        phoneme_scores,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comprehensive_spoken_evaluation_with_sounds() {
        let ref_words = vec![
            TimedWord {
                word: "hello".into(),
                p: Some(0.9),
                start_ms: 0.0,
                end_ms: 400.0,
            },
            TimedWord {
                word: "world".into(),
                p: Some(0.95),
                start_ms: 500.0,
                end_ms: 900.0,
            },
        ];
        let user_words = vec![
            TimedWord {
                word: "hello".into(),
                p: Some(0.85),
                start_ms: 100.0,
                end_ms: 500.0,
            },
            TimedWord {
                word: "world".into(),
                p: Some(0.9),
                start_ms: 600.0,
                end_ms: 1050.0,
            },
        ];

        let ref_pitches = vec![120.0, 125.0, 130.0, 128.0];
        let user_pitches = vec![210.0, 215.0, 220.0, 218.0];
        let user_energies = vec![0.45, 0.55, 0.50, 0.48];
        let expected_phonemes = vec!["h".into(), "e".into(), "l".into(), "o".into()];
        let recognized_phonemes = vec!["h".into(), "e".into(), "l".into(), "o".into()];

        let evaluation = SpokenEvaluation::evaluate(
            &ref_words,
            &user_words,
            &ref_pitches,
            &user_pitches,
            &user_energies,
            &expected_phonemes,
            &recognized_phonemes,
        );

        assert!(evaluation.overall_score > 0.0);
        assert!(evaluation.sounds.score > 0.0);
        assert_eq!(evaluation.sounds.phoneme_scores.len(), 4);
    }
}
