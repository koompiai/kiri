mod audio;
mod config;
mod output;
mod sync;
mod transcribe;
mod ui;
mod wakeword;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kiri", about = "Kiri — voice-to-text assistant")]
struct Cli {
    #[arg(short, long, global = true)]
    model: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// GTK4 voice popup — transcribe and paste into active app
    Popup {
        #[arg(short, long, default_value = "en")]
        lang: String,
        /// Save transcription as a private note to ~/kiri/ instead of pasting
        #[arg(long)]
        note: bool,
    },
    /// CLI transcription to stdout
    Listen {
        #[arg(short, long, default_value = "en")]
        lang: String,
        #[arg(short, long, default_value_t = 60)]
        duration: u32,
    },
    /// Notes git sync
    Sync,
    /// Listen for wake word and launch popup
    Wake,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Popup { lang, note }) => {
            let model_path = cli
                .model
                .map(std::path::PathBuf::from)
                .unwrap_or_else(config::default_model_path);
            ui::popup::run_popup(lang, model_path, note)
        }
        Some(Commands::Listen { lang, duration: _ }) => {
            let model_path = cli
                .model
                .map(std::path::PathBuf::from)
                .unwrap_or_else(config::default_model_path);

            eprintln!("Loading model from {}...", model_path.display());
            let engine = transcribe::whisper::WhisperEngine::load(&model_path)?;
            eprintln!("Model ready.");

            eprintln!("Recording... (Ctrl+C to stop)");
            let capture = audio::capture::AudioCapture::new();
            let raw_audio = capture.record_with_silence()?;

            if raw_audio.len() < config::RECORD_RATE as usize {
                anyhow::bail!("Recording too short, discarding.");
            }

            eprintln!("Resampling...");
            let audio_16k = audio::resample::resample_48k_to_16k(&raw_audio);

            eprintln!("Transcribing...");
            let text = engine.transcribe(&audio_16k, &lang)?;

            if text.is_empty() {
                anyhow::bail!("No speech detected.");
            }

            println!("{text}");
            Ok(())
        }
        Some(Commands::Sync) => {
            println!("{}", sync::status());
            Ok(())
        }
        Some(Commands::Wake) => {
            let model_path = cli
                .model
                .map(std::path::PathBuf::from)
                .unwrap_or_else(config::wake_model_path);

            if !model_path.exists() {
                anyhow::bail!(
                    "Wake word model not found at {}.\nDownload it:\n  curl -L -o {} \
                     https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
                    model_path.display(),
                    model_path.display()
                );
            }

            let phrases: Vec<String> = wakeword::DEFAULT_PHRASES
                .iter()
                .map(|s| s.to_string())
                .collect();

            eprintln!("Loading wake word model from {}...", model_path.display());
            let detector = wakeword::WakeWordDetector::new(&model_path, &phrases)?;
            eprintln!(
                "Listening for: {}",
                wakeword::DEFAULT_PHRASES.join(", ")
            );
            eprintln!("Press Ctrl+C to stop.");

            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            detector.listen_loop(stop, |phrase| {
                if phrase == "private" {
                    eprintln!("[kiri] Private note mode (triggered by: {phrase})");
                    let _ = std::process::Command::new("kiri")
                        .args(["popup", "--note"])
                        .spawn();
                } else {
                    eprintln!("[kiri] Launching popup (triggered by: {phrase})");
                    let _ = std::process::Command::new("kiri").arg("popup").spawn();
                }
            })?;

            Ok(())
        }
        None => {
            let model_path = cli
                .model
                .map(std::path::PathBuf::from)
                .unwrap_or_else(config::default_model_path);
            ui::popup::run_popup("en".to_string(), model_path, false)
        }
    }
}
