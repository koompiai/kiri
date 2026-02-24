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

    /// Transcribe 16kHz f32 audio. Returns text.
    pub fn transcribe(&self, audio: &[f32], language: &str) -> anyhow::Result<String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(language));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_single_segment(false);

        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| anyhow::anyhow!("Failed to create whisper state: {e}"))?;

        state
            .full(params, audio)
            .map_err(|e| anyhow::anyhow!("Transcription failed: {e}"))?;

        let n_segments = state.full_n_segments();

        let mut text = String::new();
        for i in 0..n_segments {
            if let Some(segment) = state.get_segment(i) {
                if let Ok(s) = segment.to_str() {
                    text.push_str(s.trim());
                    text.push(' ');
                }
            }
        }

        Ok(text.trim().to_string())
    }
}
