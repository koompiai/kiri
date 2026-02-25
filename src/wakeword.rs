use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustpotter::{
    AudioFmt, Endianness, Rustpotter, RustpotterConfig, SampleFormat, WakewordRefBuildFromFiles,
    WakewordRef, WakewordSave,
};

use crate::audio::capture::AudioCapture;
use crate::config;

/// Seconds to wait after activation before listening again.
const COOLDOWN: u64 = 5;

pub struct WakeWordDetector {
    rustpotter: Rustpotter,
}

impl WakeWordDetector {
    /// Load all .rpw wakeword files from the wakewords directory.
    pub fn new() -> anyhow::Result<Self> {
        let wakewords_dir = config::wakewords_dir();
        if !wakewords_dir.exists() {
            anyhow::bail!(
                "No wakewords directory at {}.\nTrain a wake word first:\n  kiri train hey-kiri",
                wakewords_dir.display()
            );
        }

        let mut config = RustpotterConfig::default();
        config.fmt = AudioFmt {
            sample_rate: config::RECORD_RATE as usize,
            sample_format: SampleFormat::F32,
            channels: config::CHANNELS,
            endianness: Endianness::Little,
        };
        // DEBUG: threshold set very low to observe scores. Tune later.
        config.detector.threshold = 0.15;
        config.detector.avg_threshold = 0.0;
        config.detector.min_scores = 1;
        config.detector.eager = true;

        let mut rustpotter =
            Rustpotter::new(&config).map_err(|e| anyhow::anyhow!("Rustpotter init: {e}"))?;

        let mut count = 0;
        for entry in std::fs::read_dir(&wakewords_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "rpw") {
                let name = path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                rustpotter
                    .add_wakeword_from_file(&name, path.to_str().unwrap())
                    .map_err(|e| anyhow::anyhow!("Failed to load {}: {e}", path.display()))?;
                eprintln!("  Loaded wake word: {name}");
                count += 1;
            }
        }

        if count == 0 {
            anyhow::bail!(
                "No .rpw files found in {}.\nTrain a wake word first:\n  kiri train hey-kiri",
                wakewords_dir.display()
            );
        }

        Ok(Self { rustpotter })
    }

    /// Run the detection loop. Calls `on_wake` with the matched wake word name.
    pub fn listen_loop(
        &mut self,
        stop: Arc<AtomicBool>,
        on_wake: impl Fn(&str),
    ) -> anyhow::Result<()> {
        let capture = AudioCapture::new();
        let _stream = capture.start_stream()?;

        let frame_size = self.rustpotter.get_samples_per_frame();
        let mut last_activation = Instant::now() - Duration::from_secs(COOLDOWN + 1);

        eprintln!("Listening... (frame size: {frame_size} samples)");
        eprintln!("Press Ctrl+C to stop.");

        let mut debug_counter = 0u32;

        loop {
            std::thread::sleep(Duration::from_millis(30));

            if stop.load(Ordering::Relaxed) {
                break;
            }

            // Cooldown after last activation
            if last_activation.elapsed() < Duration::from_secs(COOLDOWN) {
                capture.clear_buffer();
                continue;
            }

            let audio = capture.snapshot();
            if audio.len() < frame_size {
                continue;
            }
            capture.clear_buffer();

            // Feed audio frames to rustpotter
            let mut detected = false;
            for chunk in audio.chunks_exact(frame_size) {
                if let Some(detection) = self.rustpotter.process_samples(chunk.to_vec()) {
                    eprintln!(
                        "\n[kiri] Wake word detected: \"{}\" (score: {:.2}, avg: {:.2}, count: {})",
                        detection.name, detection.score, detection.avg_score, detection.counter
                    );
                    last_activation = Instant::now();
                    capture.clear_buffer(); // flush any audio that arrived during processing
                    on_wake(&detection.name);
                    detected = true;
                    break;
                }
            }
            if detected {
                continue; // skip to cooldown on next iteration
            }

            // Debug: always log partial detections when they exist
            let rms = self.rustpotter.get_rms_level();
            if let Some(partial) = self.rustpotter.get_partial_detection() {
                eprintln!(
                    "  partial: score={:.3} avg={:.3} count={} rms={:.4}",
                    partial.score, partial.avg_score, partial.counter, rms
                );
            } else {
                debug_counter += 1;
                if debug_counter % 33 == 0 {
                    if rms > 0.01 {
                        eprint!("*");
                    } else {
                        eprint!(".");
                    }
                }
            }
        }

        Ok(())
    }
}

/// Record audio samples and build a .rpw wake word reference file.
pub fn train_wakeword(name: &str, num_samples: usize) -> anyhow::Result<()> {
    let wakewords_dir = config::wakewords_dir();
    std::fs::create_dir_all(&wakewords_dir)?;

    let samples_dir = wakewords_dir.join("samples");
    std::fs::create_dir_all(&samples_dir)?;

    eprintln!("Training wake word: \"{name}\"");
    eprintln!("You will record {num_samples} samples.");
    eprintln!("Say the wake word after each prompt.\n");

    let capture = AudioCapture::new();
    let mut wav_paths = Vec::new();

    for i in 1..=num_samples {
        eprintln!("  [{i}/{num_samples}] Press Enter, then say \"{name}\"...");

        // Wait for Enter
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        // Record with silence detection
        eprintln!("  Recording...");
        capture.reset();
        let audio = capture.record_with_silence_opts(1.0)?;

        if audio.len() < (config::RECORD_RATE as usize / 4) {
            eprintln!("  Too short, skipping. Try again.");
            continue;
        }

        // Trim silence from start and end for tight MFCC templates
        let audio = trim_training_audio(&audio, config::RECORD_RATE);
        let duration = audio.len() as f32 / config::RECORD_RATE as f32;
        eprintln!("  Trimmed to {duration:.1}s");

        if audio.len() < (config::RECORD_RATE as usize / 4) {
            eprintln!("  Too short after trimming, skipping. Try again.");
            continue;
        }

        // Save as WAV
        let wav_path = samples_dir.join(format!("{name}_{i}.wav"));
        save_wav(&wav_path, &audio, config::RECORD_RATE)?;
        eprintln!("  Saved: {}", wav_path.display());
        wav_paths.push(wav_path);
    }

    if wav_paths.len() < 3 {
        anyhow::bail!("Need at least 3 samples, got {}. Try again.", wav_paths.len());
    }

    // Build wakeword reference from WAV files
    eprintln!("\nBuilding wake word reference from {} samples...", wav_paths.len());
    let sample_files: Vec<String> = wav_paths.iter().map(|p| p.to_str().unwrap().to_string()).collect();

    let wakeword = WakewordRef::new_from_sample_files(
        name.to_string(),
        None, // use default threshold
        None, // use default avg_threshold
        sample_files,
        16,   // mfcc_size
    )
    .map_err(|e| anyhow::anyhow!("Failed to build wake word: {e}"))?;

    let rpw_path = wakewords_dir.join(format!("{name}.rpw"));
    wakeword
        .save_to_file(rpw_path.to_str().unwrap())
        .map_err(|e| anyhow::anyhow!("Failed to save: {e}"))?;

    eprintln!("Wake word saved: {}", rpw_path.display());
    eprintln!("\nNow run: kiri wake");

    Ok(())
}

/// Trim leading/trailing silence from training audio for tight MFCC templates.
/// Keeps a small padding (50ms) around the speech.
fn trim_training_audio(audio: &[f32], sample_rate: u32) -> Vec<f32> {
    let window = sample_rate as usize / 50; // 20ms windows
    let padding = sample_rate as usize / 20; // 50ms padding
    let threshold = 0.02;

    if audio.len() < window {
        return audio.to_vec();
    }

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
                let rms = (chunk.iter().map(|&s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
                rms > threshold
            })
            .unwrap_or(0)
            * window;

    if start >= end {
        return audio.to_vec();
    }

    let start = start.saturating_sub(padding);
    let end = (end + padding).min(audio.len());
    audio[start..end].to_vec()
}

fn save_wav(path: &Path, audio: &[f32], sample_rate: u32) -> anyhow::Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &sample in audio {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}
