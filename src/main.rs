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
    /// Listen for wake words and launch popup
    Wake,
    /// Record samples and train a wake word
    Train {
        /// Wake word name (e.g. "hey-kiri", "koompi")
        name: String,
        /// Number of samples to record (default: 5)
        #[arg(short = 'n', long, default_value_t = 5)]
        samples: usize,
    },
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
            eprintln!("Loading wake words...");
            let mut detector = wakeword::WakeWordDetector::new()?;

            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            detector.listen_loop(stop, |name| {
                if name == "private" {
                    eprintln!("[kiri] Private note mode (triggered by: {name})");
                    let _ = std::process::Command::new("kiri")
                        .args(["popup", "--note"])
                        .spawn();
                } else {
                    eprintln!("[kiri] Launching popup (triggered by: {name})");
                    let _ = std::process::Command::new("kiri").arg("popup").spawn();
                }
            })?;

            Ok(())
        }
        Some(Commands::Train { name, samples }) => {
            wakeword::train_wakeword(&name, samples)
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
