use std::path::Path;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct WhisperEngine {
    ctx: WhisperContext,
}

impl WhisperEngine {
    /// Load a GGML model file.
    pub fn load(model_path: &Path) -> anyhow::Result<Self> {
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().unwrap(),
            WhisperContextParameters::default(),
        )
        .map_err(|e| anyhow::anyhow!("Failed to load whisper model: {e}"))?;

        Ok(Self { ctx })
    }

    /// High-quality transcription using beam search. Use for final output.
    pub fn transcribe(&self, audio: &[f32], language: &str) -> anyhow::Result<String> {
        self.transcribe_inner(audio, language, None, true)
    }

    /// Fast transcription using greedy decoding. Use for streaming partials.
    pub fn transcribe_fast(&self, audio: &[f32], language: &str) -> anyhow::Result<String> {
        self.transcribe_inner(audio, language, None, false)
    }

    /// Transcribe with prompt bias (greedy). Use for wake word detection.
    pub fn transcribe_with_prompt(
        &self,
        audio: &[f32],
        language: &str,
        prompt: &str,
    ) -> anyhow::Result<String> {
        self.transcribe_inner(audio, language, Some(prompt), false)
    }

    fn transcribe_inner(
        &self,
        audio: &[f32],
        language: &str,
        prompt: Option<&str>,
        high_quality: bool,
    ) -> anyhow::Result<String> {
        // Preprocess: normalize volume and trim leading/trailing silence
        let mut processed = audio.to_vec();
        normalize_audio(&mut processed);
        let trimmed = trim_silence(&processed);

        if trimmed.is_empty() {
            return Ok(String::new());
        }

        let mut params = if high_quality {
            FullParams::new(SamplingStrategy::BeamSearch {
                beam_size: 5,
                patience: 1.0,
            })
        } else {
            FullParams::new(SamplingStrategy::Greedy { best_of: 1 })
        };

        params.set_language(Some(language));
        if let Some(p) = prompt {
            params.set_initial_prompt(p);
        }
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_single_segment(false);
        params.set_suppress_nst(true);
        params.set_suppress_blank(true);

        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| anyhow::anyhow!("Failed to create whisper state: {e}"))?;

        state
            .full(params, trimmed)
            .map_err(|e| anyhow::anyhow!("Transcription failed: {e}"))?;

        let n_segments = state.full_n_segments();

        let mut text = String::new();
        for i in 0..n_segments {
            if let Some(segment) = state.get_segment(i)
                && let Ok(s) = segment.to_str()
            {
                text.push_str(s.trim());
                text.push(' ');
            }
        }

        Ok(text.trim().to_string())
    }
}

/// Normalize audio to ~95% peak amplitude so whisper gets consistent input
/// regardless of mic gain.
fn normalize_audio(audio: &mut [f32]) {
    let max = audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if max > 0.001 && max < 0.95 {
        let scale = 0.95 / max;
        for s in audio.iter_mut() {
            *s *= scale;
        }
    }
}

/// Trim leading and trailing silence from 16kHz audio so whisper focuses
/// on speech content.
fn trim_silence(audio: &[f32]) -> &[f32] {
    let window = crate::config::WHISPER_RATE as usize / 50; // 20ms windows
    if audio.len() < window {
        return audio;
    }

    let threshold = 0.01;

    let start = audio
        .chunks(window)
        .position(|chunk| {
            let rms = (chunk.iter().map(|&s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
            rms > threshold
        })
        .unwrap_or(0)
        * window;

    let end = audio.len()
        - audio
            .chunks(window)
            .rev()
            .position(|chunk| {
                let rms =
                    (chunk.iter().map(|&s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
                rms > threshold
            })
            .unwrap_or(0)
            * window;

    if start >= end {
        return &audio[..0];
    }

    &audio[start..end]
}
