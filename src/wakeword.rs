use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::audio::capture::AudioCapture;
use crate::audio::resample::resample_48k_to_16k;
use crate::config;
use crate::transcribe::whisper::WhisperEngine;

/// Default wake phrases.
pub const DEFAULT_PHRASES: &[&str] = &["hey kiri", "kiri", "koompi", "nimmit", "nadi", "private"];

/// Seconds between analysis cycles.
const WAKE_STRIDE: f32 = 1.5;

/// Minimum audio duration worth transcribing (seconds).
const MIN_AUDIO: f32 = 0.8;

/// Minimum RMS to consider a window worth transcribing.
const WAKE_VAD_THRESHOLD: f32 = 0.02;

/// Maximum normalized edit distance for fuzzy matching (0.0–1.0).
const MATCH_THRESHOLD: f32 = 0.35;

/// Seconds to wait after activation before listening again.
const COOLDOWN: u64 = 5;

pub struct WakeWordDetector {
    engine: WhisperEngine,
    phrases: Vec<String>,
    initial_prompt: String,
}

impl WakeWordDetector {
    pub fn new(model_path: &Path, phrases: &[String]) -> anyhow::Result<Self> {
        let engine = WhisperEngine::load(model_path)?;
        let phrase_list: Vec<String> = phrases.iter().map(|p| p.to_lowercase()).collect();
        let initial_prompt = phrases.join(". ") + ".";
        Ok(Self {
            engine,
            phrases: phrase_list,
            initial_prompt,
        })
    }

    /// Run the detection loop. Calls `on_wake` with the matched phrase.
    /// Blocks until stop flag is set or process is killed.
    pub fn listen_loop(
        &self,
        stop: Arc<AtomicBool>,
        on_wake: impl Fn(&str),
    ) -> anyhow::Result<()> {
        let capture = AudioCapture::new();
        let _stream = capture.start_stream()?;

        let stride = Duration::from_secs_f32(WAKE_STRIDE);
        let min_samples = (MIN_AUDIO * config::RECORD_RATE as f32) as usize;
        let mut last_activation = Instant::now() - Duration::from_secs(COOLDOWN + 1);

        loop {
            std::thread::sleep(stride);

            if stop.load(Ordering::Relaxed) {
                break;
            }

            // Cooldown after last activation
            if last_activation.elapsed() < Duration::from_secs(COOLDOWN) {
                capture.clear_buffer();
                continue;
            }

            let audio = capture.snapshot();
            capture.clear_buffer();

            if audio.len() < min_samples {
                continue;
            }

            // Simple VAD: skip quiet windows
            let rms =
                (audio.iter().map(|&s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
            if rms < WAKE_VAD_THRESHOLD {
                continue;
            }

            // Resample and transcribe with prompt bias
            let audio_16k = resample_48k_to_16k(&audio);
            let text = match self
                .engine
                .transcribe_with_prompt(&audio_16k, "en", &self.initial_prompt)
            {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Wake word error: {e}");
                    continue;
                }
            };

            let text_lower = text.trim().to_lowercase();
            if text_lower.is_empty() {
                continue;
            }

            if let Some(phrase) = self.find_match(&text_lower) {
                eprintln!(
                    "[kiri] Wake word detected: \"{}\" (heard: \"{}\")",
                    phrase, text_lower
                );
                last_activation = Instant::now();
                on_wake(&phrase);
            }
        }

        Ok(())
    }

    fn find_match(&self, text: &str) -> Option<String> {
        // Strip punctuation for matching
        let clean: String = text
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect();
        let clean = clean.trim();

        for phrase in &self.phrases {
            // Exact substring match
            if clean.contains(phrase.as_str()) {
                return Some(phrase.clone());
            }

            // Fuzzy: sliding window of phrase-length words
            let phrase_words: Vec<&str> = phrase.split_whitespace().collect();
            let text_words: Vec<&str> = clean.split_whitespace().collect();

            for window in text_words.windows(phrase_words.len().max(1)) {
                let window_str = window.join(" ");
                let dist = levenshtein(&window_str, phrase);
                let max_len = window_str.len().max(phrase.len());
                if max_len > 0 && (dist as f32 / max_len as f32) <= MATCH_THRESHOLD {
                    return Some(phrase.clone());
                }
            }
        }

        None
    }
}

/// Levenshtein edit distance between two strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());

    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}
