mod audio;
mod config;

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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Popup { lang }) => {
            eprintln!("popup: lang={lang}, model={:?}", cli.model);
            todo!("popup not implemented yet")
        }
        Some(Commands::Listen { lang, duration }) => {
            eprintln!("listen: lang={lang}, duration={duration}");
            todo!("listen not implemented yet")
        }
        Some(Commands::Sync) => {
            eprintln!("sync");
            todo!("sync not implemented yet")
        }
        None => {
            // Default: popup
            eprintln!("default -> popup");
            todo!("popup not implemented yet")
        }
    }
}
