# Kiri Rust Rewrite — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rewrite kiri from Python to Rust — single compiled binary with wake word detection, whisper.cpp transcription, GTK4 popup overlay, and live paste-to-active-app.

**Architecture:** A single Rust binary (`kiri`) with subcommands: `popup` (GTK4 voice overlay that pastes into the active app), `listen` (CLI transcribe-to-stdout), `sync` (notes git sync), and `daemon` (always-on wake word listener). Audio capture via `cpal`, transcription via `whisper-rs` (whisper.cpp bindings), wake word detection via `ort` (ONNX Runtime for openWakeWord models), GTK4 popup via `gtk4-rs` + `gtk4-layer-shell`, clipboard paste via `wl-clipboard-rs` + `ydotool`.

**Tech Stack:**
- `whisper-rs 0.15` — whisper.cpp Rust bindings (Whisper Medium model, ~5% WER)
- `gtk4 0.11` + `gtk4-layer-shell 0.7` — popup overlay UI
- `cpal 0.17` — cross-platform audio capture
- `rubato 1.0` — audio resampling (48kHz mic → 16kHz whisper)
- `wl-clipboard-rs 0.9` — Wayland clipboard access
- `ort 2.0` — ONNX Runtime for wake word models
- `clap 4.5` — CLI argument parsing

---

## Crate Layout

```
kiri/
├── Cargo.toml
├── src/
│   ├── main.rs              # clap CLI, subcommand dispatch
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── capture.rs        # cpal mic capture, ring buffer
│   │   └── resample.rs       # rubato 48k→16k resampling
│   ├── transcribe/
│   │   ├── mod.rs
│   │   └── whisper.rs        # whisper-rs engine wrapper
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── popup.rs          # GTK4 VoicePopup window
│   │   ├── states.rs         # State enum
│   │   └── styles.rs         # CSS string
│   ├── output/
│   │   ├── mod.rs
│   │   ├── clipboard.rs      # wl-clipboard-rs
│   │   ├── typer.rs          # clipboard + ydotool paste
│   │   └── notes.rs          # save to ~/kiri/*.md
│   ├── config.rs             # constants, paths
│   └── sync.rs               # git operations for notes
├── install.sh                # updated installer
└── tests/
    └── ...
```

---

### Task 1: Cargo Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/config.rs`

**Step 1: Initialize the Cargo project**

Run: `cargo init --name kiri` in the repo root (on the `feat/rust-rewrite` branch).

This will create `Cargo.toml` and `src/main.rs`. The existing Python `src/kiri/` directory stays — we'll keep it until the Rust version is fully working, then remove it.

**Step 2: Write Cargo.toml with all dependencies**

```toml
[package]
name = "kiri"
version = "0.1.0"
edition = "2024"
description = "Kiri — voice-to-text assistant"

[[bin]]
name = "kiri"
path = "src/main.rs"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
gtk4 = "0.11"
gtk4-layer-shell = "0.7"
cpal = "0.17"
rubato = "1.0"
whisper-rs = "0.15"
wl-clipboard-rs = "0.9"
anyhow = "1"
dirs = "6"
chrono = "0.4"
```

Note: `ort` for wake word is deferred to a later task. Start with the core transcription flow.

**Step 3: Write src/config.rs**

```rust
use std::path::PathBuf;

pub const RECORD_RATE: u32 = 48000;
pub const WHISPER_RATE: u32 = 16000;
pub const CHANNELS: u16 = 1;

pub const SILENCE_THRESHOLD: f32 = 0.015;
pub const SILENCE_DURATION: f32 = 2.5;
pub const SPEECH_MIN_DURATION: f32 = 0.5;
pub const MAX_DURATION: f32 = 120.0;
pub const DONE_TIMEOUT: f32 = 5.0;

pub fn notes_dir() -> PathBuf {
    dirs::home_dir().unwrap().join("kiri")
}

pub fn models_dir() -> PathBuf {
    dirs::data_dir().unwrap().join("kiri").join("models")
}

pub fn default_model_path() -> PathBuf {
    models_dir().join("ggml-medium.bin")
}
```

**Step 4: Write src/main.rs with clap subcommands (stubs)**

```rust
mod config;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kiri", about = "Kiri — voice-to-text assistant")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// GTK4 voice popup — transcribe and paste into active app
    Popup {
        #[arg(short, long, default_value = "en")]
        lang: String,
        #[arg(short, long)]
        model: Option<String>,
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
        Some(Commands::Popup { lang, model }) => {
            eprintln!("popup: lang={lang}, model={model:?}");
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
```

**Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully (subcommands are stubs with `todo!()`)

**Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs src/config.rs
git commit -m "feat: scaffold Rust project with clap subcommands"
```

---

### Task 2: Audio Capture Module

**Files:**
- Create: `src/audio/mod.rs`
- Create: `src/audio/capture.rs`
- Create: `src/audio/resample.rs`
- Modify: `src/main.rs` (add `mod audio;`)

**Step 1: Write src/audio/resample.rs**

Resample 48kHz → 16kHz using `rubato`.

```rust
use rubato::{FftFixedIn, Resampler};

/// Resample f32 audio from 48kHz to 16kHz (ratio 1:3).
pub fn resample_48k_to_16k(input: &[f32]) -> Vec<f32> {
    let mut resampler = FftFixedIn::<f32>::new(48000, 16000, input.len(), 1, 1)
        .expect("failed to create resampler");
    let result = resampler.process(&[input], None).expect("resample failed");
    result.into_iter().next().unwrap()
}
```

**Step 2: Write src/audio/capture.rs**

Audio capture with silence detection, matching Python's `AudioRecorder`.

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::config::*;

/// Captured audio result.
pub struct AudioCapture {
    frames: Arc<Mutex<Vec<f32>>>,
    pub audio_level: Arc<Mutex<f32>>,
    stop: Arc<AtomicBool>,
}

impl AudioCapture {
    pub fn new() -> Self {
        Self {
            frames: Arc::new(Mutex::new(Vec::new())),
            audio_level: Arc::new(Mutex::new(0.0)),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal the recording to stop.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Record until silence is detected after speech. Returns 48kHz f32 audio.
    pub fn record_with_silence(&self) -> anyhow::Result<Vec<f32>> {
        self.stop.store(false, Ordering::Relaxed);
        self.frames.lock().unwrap().clear();

        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device found"))?;

        let config = cpal::StreamConfig {
            channels: CHANNELS,
            sample_rate: cpal::SampleRate(RECORD_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        let frames = self.frames.clone();
        let audio_level = self.audio_level.clone();
        let stop = self.stop.clone();
        let stop2 = self.stop.clone();

        let mut speech_detected = false;
        let mut silence_start: Option<Instant> = None;
        let mut speech_start: Option<Instant> = None;

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                frames.lock().unwrap().extend_from_slice(data);

                let rms = (data.iter().map(|&s| s * s).sum::<f32>() / data.len() as f32).sqrt();
                *audio_level.lock().unwrap() = rms;

                let now = Instant::now();

                if rms > SILENCE_THRESHOLD {
                    silence_start = None;
                    if !speech_detected {
                        speech_detected = true;
                        speech_start = Some(now);
                    }
                } else if speech_detected {
                    if let Some(start) = speech_start {
                        if now.duration_since(start).as_secs_f32() < SPEECH_MIN_DURATION {
                            return;
                        }
                    }
                    if silence_start.is_none() {
                        silence_start = Some(now);
                    } else if let Some(start) = silence_start {
                        if now.duration_since(start).as_secs_f32() >= SILENCE_DURATION {
                            stop.store(true, Ordering::Relaxed);
                        }
                    }
                }
            },
            move |err| {
                eprintln!("Audio stream error: {err}");
            },
            None,
        )?;

        stream.play()?;

        let start = Instant::now();
        while !stop2.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if start.elapsed().as_secs_f32() >= MAX_DURATION {
                break;
            }
        }

        drop(stream);
        let audio = self.frames.lock().unwrap().clone();
        Ok(audio)
    }

    /// Get current audio level (for UI meter).
    pub fn get_level(&self) -> f32 {
        *self.audio_level.lock().unwrap()
    }
}
```

**Step 3: Write src/audio/mod.rs**

```rust
pub mod capture;
pub mod resample;
```

**Step 4: Add `mod audio;` to src/main.rs**

Add `mod audio;` after `mod config;`.

**Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 6: Commit**

```bash
git add src/audio/
git commit -m "feat: add audio capture with cpal and rubato resampler"
```

---

### Task 3: Whisper Transcription Engine

**Files:**
- Create: `src/transcribe/mod.rs`
- Create: `src/transcribe/whisper.rs`
- Modify: `src/main.rs` (add `mod transcribe;`)

**Context:** `whisper-rs` wraps whisper.cpp. It uses GGML model files (e.g., `ggml-medium.bin`). The user will download the model separately. The engine needs: load model, transcribe 16kHz f32 audio → text.

**Step 1: Write src/transcribe/whisper.rs**

```rust
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

        let mut state = self.ctx.create_state()
            .map_err(|e| anyhow::anyhow!("Failed to create whisper state: {e}"))?;

        state.full(params, audio)
            .map_err(|e| anyhow::anyhow!("Transcription failed: {e}"))?;

        let n_segments = state.full_n_segments()
            .map_err(|e| anyhow::anyhow!("Failed to get segments: {e}"))?;

        let mut text = String::new();
        for i in 0..n_segments {
            if let Ok(segment) = state.full_get_segment_text(i) {
                text.push_str(segment.trim());
                text.push(' ');
            }
        }

        Ok(text.trim().to_string())
    }
}
```

**Step 2: Write src/transcribe/mod.rs**

```rust
pub mod whisper;
```

**Step 3: Add `mod transcribe;` to src/main.rs**

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles (requires whisper.cpp C library — whisper-rs builds it from source via its build script).

Note: First build will be slow as whisper.cpp compiles. If it fails on missing system deps, you may need `cmake` and a C++ compiler installed.

**Step 5: Commit**

```bash
git add src/transcribe/
git commit -m "feat: add whisper-rs transcription engine"
```

---

### Task 4: Output Modules (Clipboard, Typer, Notes)

**Files:**
- Create: `src/output/mod.rs`
- Create: `src/output/clipboard.rs`
- Create: `src/output/typer.rs`
- Create: `src/output/notes.rs`
- Modify: `src/main.rs` (add `mod output;`)

**Step 1: Write src/output/clipboard.rs**

```rust
use wl_clipboard_rs::copy::{MimeType, Options, Source};

/// Copy text to the Wayland clipboard.
pub fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    let opts = Options::new();
    opts.copy(Source::Bytes(text.as_bytes().into()), MimeType::Text)?;
    Ok(())
}
```

**Step 2: Write src/output/typer.rs**

Paste text into the active app: set clipboard, then simulate Ctrl+Shift+V via ydotool.

```rust
use std::process::Command;

use crate::output::clipboard::copy_to_clipboard;

/// Paste text into the focused application.
/// Sets clipboard, then simulates Ctrl+Shift+V with ydotool.
pub fn paste_text(text: &str) -> anyhow::Result<()> {
    copy_to_clipboard(text)?;
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Ctrl+Shift+V: keycode 29=LCtrl, 42=LShift, 47=V
    Command::new("ydotool")
        .args(["key", "29:1", "42:1", "47:1", "47:0", "42:0", "29:0"])
        .status()
        .map_err(|e| anyhow::anyhow!("ydotool failed: {e}. Is ydotoold running?"))?;

    Ok(())
}
```

**Step 3: Write src/output/notes.rs**

```rust
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use chrono::Local;

use crate::config::notes_dir;

/// Save transcribed text to a markdown file in ~/kiri/.
/// Returns the file path.
pub fn save_to_notes(text: &str) -> anyhow::Result<PathBuf> {
    let dir = notes_dir();
    fs::create_dir_all(&dir)?;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let filepath = dir.join(format!("{today}.md"));
    let timestamp = Local::now().format("%H:%M").to_string();

    let is_new = !filepath.exists();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&filepath)?;

    if is_new {
        writeln!(file, "# {today}\n")?;
    }
    writeln!(file, "<!-- {timestamp} -->\n{text}\n")?;

    Ok(filepath)
}
```

**Step 4: Write src/output/mod.rs**

```rust
pub mod clipboard;
pub mod notes;
pub mod typer;
```

**Step 5: Add `mod output;` to src/main.rs**

**Step 6: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 7: Commit**

```bash
git add src/output/
git commit -m "feat: add output modules (clipboard, typer, notes)"
```

---

### Task 5: CLI `listen` Subcommand (End-to-End Transcription)

**Files:**
- Modify: `src/main.rs` — implement the `Listen` subcommand

**Context:** This wires up audio capture → resample → whisper → print. It's the simplest end-to-end path and validates all the core modules work together.

**Step 1: Implement the Listen subcommand**

In `src/main.rs`, replace the `Listen` arm:

```rust
Some(Commands::Listen { lang, duration }) => {
    let model_path = model.map(std::path::PathBuf::from)
        .unwrap_or_else(|| config::default_model_path());

    eprintln!("Loading model from {}...", model_path.display());
    let engine = transcribe::whisper::WhisperEngine::load(&model_path)?;
    eprintln!("Model ready.");

    eprintln!("Recording... (max {duration}s, Ctrl+C to stop)");
    let capture = audio::capture::AudioCapture::new();
    let raw_audio = capture.record_with_silence()?;

    if raw_audio.len() < RECORD_RATE as usize {
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
```

Note: Also add a global `--model` flag to `Cli` struct so all subcommands can share it:

```rust
#[derive(Parser)]
#[command(name = "kiri", about = "Kiri — voice-to-text assistant")]
struct Cli {
    #[arg(short, long, global = true)]
    model: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}
```

**Step 2: Build and test manually**

Run: `cargo build --release`

Then download a whisper GGML model if not already present:
```bash
# Download whisper medium model (1.5GB)
mkdir -p ~/.local/share/kiri/models
curl -L -o ~/.local/share/kiri/models/ggml-medium.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin
```

Test: `./target/release/kiri listen --lang en`
Expected: Records audio, transcribes, prints text to stdout.

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement CLI listen subcommand (end-to-end transcription)"
```

---

### Task 6: GTK4 Popup — UI Shell

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/states.rs`
- Create: `src/ui/styles.rs`
- Create: `src/ui/popup.rs`
- Modify: `src/main.rs` (add `mod ui;`, implement `Popup` subcommand)

**Context:** This is the GTK4 popup window — the core UX. Must match the Python version: gradient background, dot indicator, status label, level bar, result text, hint label, stop button. Layer shell: top-right, no keyboard focus steal.

**Step 1: Write src/ui/states.rs**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    Loading,
    Listening,
    Transcribing,
    Result,
    Error,
}
```

**Step 2: Write src/ui/styles.rs**

Copy the CSS verbatim from the Python version (including `.stop-btn`).

```rust
pub const CSS: &str = r#"
window {
    background: linear-gradient(135deg,
        rgba(118, 56, 250, 0.92),
        rgba(200, 60, 180, 0.92),
        rgba(56, 200, 160, 0.92));
    border-radius: 16px;
}
.container { padding: 24px 32px; }
.status-label {
    color: #ffffff;
    font-size: 15px;
    font-weight: 600;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}
.result-text {
    color: rgba(255, 255, 255, 0.92);
    font-size: 14px;
    font-style: italic;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}
.error-label {
    color: #ffcdd2;
    font-size: 13px;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}
.dot-recording { color: #ffffff; font-size: 24px; text-shadow: 0 0 8px rgba(255, 100, 100, 0.8); }
.dot-loading { color: rgba(255, 255, 255, 0.8); font-size: 24px; }
.dot-done { color: #a5ffd6; font-size: 24px; text-shadow: 0 0 8px rgba(100, 255, 180, 0.6); }
.level-bar-bg { background-color: rgba(255, 255, 255, 0.15); border-radius: 4px; min-height: 8px; }
.level-bar-fg { background: linear-gradient(90deg, #c83cb4, #ffffff, #38c8a0); border-radius: 4px; min-height: 8px; }
.hint-label { color: rgba(255, 255, 255, 0.5); font-size: 10px; }
.stop-btn {
    background: rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    color: rgba(255, 255, 255, 0.7);
    font-size: 14px;
    font-weight: 700;
    min-width: 28px;
    min-height: 28px;
    padding: 0;
    border: none;
    box-shadow: none;
}
.stop-btn:hover { background: rgba(255, 80, 80, 0.5); color: #ffffff; }
"#;
```

**Step 3: Write src/ui/popup.rs — GTK4 window setup**

This is the big one. Structure it as a `VoicePopup` that:
1. Creates the GTK Application
2. On `activate`: builds the window with layer shell, CSS, all widgets
3. Spawns a background thread for model load → audio capture → transcription
4. Uses `glib::idle_add_local_once` to safely update UI from background thread
5. On line completion, calls `paste_text()` to paste into active app
6. 5s silence timeout with countdown

```rust
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use gtk4::prelude::*;
use gtk4::{self as gtk, gdk, glib, Application, ApplicationWindow, CssProvider};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::audio::capture::AudioCapture;
use crate::audio::resample::resample_48k_to_16k;
use crate::config::*;
use crate::output::typer::paste_text;
use crate::transcribe::whisper::WhisperEngine;
use crate::ui::states::State;
use crate::ui::styles::CSS;

pub fn run_popup(lang: String, model_path: PathBuf) -> anyhow::Result<()> {
    let app = Application::builder()
        .application_id("com.kiri.popup")
        .build();

    let lang = lang.clone();
    let model_path = model_path.clone();

    app.connect_activate(move |app| {
        build_ui(app, lang.clone(), model_path.clone());
    });

    app.run_with_args::<String>(&[]);
    Ok(())
}

fn build_ui(app: &Application, lang: String, model_path: PathBuf) {
    // CSS
    let provider = CssProvider::new();
    provider.load_from_string(CSS);
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Window
    let win = ApplicationWindow::builder()
        .application(app)
        .title("Kiri")
        .default_width(340)
        .default_height(140)
        .resizable(false)
        .decorated(false)
        .build();

    // Layer shell
    win.init_layer_shell();
    win.set_layer(Layer::Top);
    win.set_anchor(Edge::Top, true);
    win.set_anchor(Edge::Right, true);
    win.set_margin(Edge::Top, 12);
    win.set_margin(Edge::Right, 12);
    win.set_keyboard_mode(KeyboardMode::None);

    // Build widget tree (same layout as Python version)
    // ... (dot, status_label, level_bar, info_label, hint, stop_btn)
    // Omitted for brevity — follow the Python popup.py layout exactly

    win.present();

    // Spawn background work
    // ... (model load, audio capture, transcription loop)
}
```

Due to the complexity of the GTK4 popup, this task focuses on getting the **window visible with the correct layout and styling**. The background transcription loop is wired in the next task.

**Step 4: Wire popup subcommand in main.rs**

```rust
Some(Commands::Popup { lang, model }) => {
    let model_path = model.map(std::path::PathBuf::from)
        .unwrap_or_else(|| config::default_model_path());
    ui::popup::run_popup(lang, model_path)
}
```

**Step 5: Build and test**

Run: `cargo build && ./target/debug/kiri popup`
Expected: A floating GTK4 popup appears in the top-right corner with gradient background, "Loading model..." text.

**Step 6: Commit**

```bash
git add src/ui/
git commit -m "feat: add GTK4 popup window with layer shell"
```

---

### Task 7: Wire Popup to Transcription (Live Streaming Paste)

**Files:**
- Modify: `src/ui/popup.rs` — add background thread with audio capture + whisper + live paste

**Context:** This is the live streaming flow from the Python version: capture audio in chunks → feed to whisper → paste each transcribed segment immediately. Since whisper-rs doesn't have a native streaming callback API like Moonshine, we implement chunk-based streaming: record until silence (short, ~1s), transcribe that chunk, paste it, loop until 5s total silence.

**Step 1: Implement the streaming transcription loop**

In `build_ui`, after `win.present()`, spawn a background thread:

```rust
let app_clone = app.clone();
// Shared state between UI thread and background thread
let state = Arc::new(Mutex::new(State::Loading));
let accumulated_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
let has_speech = Arc::new(AtomicBool::new(false));
let stop_flag = Arc::new(AtomicBool::new(false));

std::thread::spawn(move || {
    // 1. Load model
    let engine = match WhisperEngine::load(&model_path) {
        Ok(e) => e,
        Err(e) => {
            // update UI to show error
            glib::idle_add_local_once(move || { /* set error state */ });
            return;
        }
    };

    // 2. Streaming loop: record short chunks, transcribe, paste
    // update UI to LISTENING state
    let capture = AudioCapture::new();

    loop {
        if stop_flag.load(Ordering::Relaxed) { break; }

        // Record until short silence (1s) or done timeout (5s)
        let raw = match capture.record_with_silence() {
            Ok(audio) => audio,
            Err(_) => break,
        };

        if raw.len() < (RECORD_RATE as usize / 2) { continue; }

        let audio_16k = resample_48k_to_16k(&raw);
        let text = match engine.transcribe(&audio_16k, &lang) {
            Ok(t) => t,
            Err(_) => continue,
        };

        if text.is_empty() { continue; }

        has_speech.store(true, Ordering::Relaxed);
        accumulated_lines.lock().unwrap().push(text.clone());

        // Paste this line immediately
        glib::idle_add_local_once(move || {
            let _ = paste_text(&format!("{text} "));
        });
    }

    // Session done — quit
    glib::idle_add_local_once(move || {
        app_clone.quit();
    });
});
```

**Step 2: Add pulse timer for UI updates**

```rust
glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
    // Update level bar width from audio_level
    // Update countdown hint
    glib::ControlFlow::Continue
});
```

**Step 3: Build and test**

Run: `cargo build --release && ./target/release/kiri popup`
Expected: Popup appears, loads model, starts listening, transcribes speech, pastes into active app, shows countdown, quits after 5s silence.

**Step 4: Commit**

```bash
git add src/ui/popup.rs
git commit -m "feat: wire popup to live streaming transcription with paste"
```

---

### Task 8: Sync Module (Git Operations for Notes)

**Files:**
- Create: `src/sync.rs`
- Modify: `src/main.rs` (implement `Sync` subcommand)

**Step 1: Write src/sync.rs**

Port the Python `sync.py` — git operations on ~/kiri/ directory.

```rust
use std::path::Path;
use std::process::Command;

use crate::config::notes_dir;

fn git(args: &[&str]) -> anyhow::Result<std::process::Output> {
    let output = Command::new("git")
        .args(args)
        .current_dir(notes_dir())
        .output()?;
    Ok(output)
}

pub fn is_notes_repo() -> bool {
    notes_dir().join(".git").exists()
}

pub fn commit_notes() -> anyhow::Result<bool> {
    if !is_notes_repo() { return Ok(false); }
    git(&["add", "-A"])?;
    let status = git(&["diff", "--cached", "--quiet"])?;
    if status.status.success() { return Ok(false); }
    let msg = format!("kiri: notes update {}", chrono::Local::now().format("%Y-%m-%d %H:%M"));
    git(&["commit", "-m", &msg])?;
    Ok(true)
}

pub fn push_notes() -> anyhow::Result<()> {
    if !is_notes_repo() {
        anyhow::bail!("Not a git repo. Run: kiri sync --init <url>");
    }
    git(&["push", "-u", "origin", "main"])?;
    Ok(())
}

pub fn status() -> String {
    if !is_notes_repo() {
        return format!("Not a git repo: {}", notes_dir().display());
    }
    let log = git(&["log", "--oneline", "-5"]).ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    format!("Notes dir: {}\nRecent commits:\n{}", notes_dir().display(), log)
}
```

**Step 2: Wire Sync subcommand in main.rs**

```rust
Some(Commands::Sync) => {
    println!("{}", sync::status());
    Ok(())
}
```

**Step 3: Build and verify**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/sync.rs
git commit -m "feat: add notes git sync module"
```

---

### Task 9: Update Installer for Rust Binary

**Files:**
- Modify: `install.sh`

**Context:** The installer currently clones the repo, runs `uv sync`, and creates wrapper scripts. For Rust, it should either: (A) download a prebuilt binary from GitHub releases, or (B) compile from source with `cargo build --release`. Start with option B (compile from source) since we don't have CI/release infrastructure yet.

**Step 1: Update install.sh**

Key changes:
- Install Rust toolchain (if not present) instead of uv
- System deps: add `cmake` and C++ compiler for whisper.cpp build
- Build with `cargo build --release`
- Copy binary to `~/.local/bin/kiri`
- Remove uv/Python-related sections
- Add model download step

The installer should still support `--uninstall`, `--no-deps`, and `--local=`.

**Step 2: Test install**

Run: `bash install.sh --local=. --no-deps`
Expected: Builds from local source, installs binary to ~/.local/bin/kiri.

**Step 3: Commit**

```bash
git add install.sh
git commit -m "feat: update installer for Rust binary"
```

---

### Task 10: Remove Python Source, Clean Up

**Files:**
- Delete: `src/kiri/` (entire Python source tree)
- Delete: `pyproject.toml`
- Delete: `uv.lock`
- Delete: `.python-version`
- Delete: `.venv/` (if present)
- Delete: `tests/` (Python tests)
- Update: `README.md`
- Update: `.gitignore` (add `/target/`)

**Step 1: Remove Python files**

```bash
rm -rf src/kiri/ tests/ .venv/ .pytest_cache/
rm -f pyproject.toml uv.lock .python-version
```

**Step 2: Update .gitignore**

Add:
```
/target/
```

**Step 3: Update README.md**

Brief update to reflect Rust build:
- Build: `cargo build --release`
- Install: `bash install.sh`
- Binary: `kiri popup`, `kiri listen`, `kiri sync`

**Step 4: Commit**

```bash
git add -A
git commit -m "chore: remove Python source, complete Rust migration"
```

---

### Task 11 (Future): Wake Word Daemon

**Deferred.** This task adds always-on wake word detection using openWakeWord ONNX models via the `ort` crate. It requires:
- Custom "Hey Kiri" and "Kiri" wake word models (trained separately)
- ONNX Runtime inference pipeline: mel spectrogram → embedding → classifier
- Daemon subcommand: `kiri daemon` (systemd service)
- Audio chime on detection, then launch popup

This is tracked separately because it depends on:
1. Training the wake word models (Python, one-time)
2. The core popup working end-to-end in Rust

---

## Build & Test Checklist

After each task, verify:
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo clippy` passes
- [ ] Manual test of the new functionality
- [ ] Commit with descriptive message

## Model Download

The Rust version uses whisper.cpp's GGML format models, not OpenVINO. Download once:

```bash
mkdir -p ~/.local/share/kiri/models
curl -L -o ~/.local/share/kiri/models/ggml-medium.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin
```

This is ~1.5GB. For testing, use `ggml-base.bin` (~142MB) first:

```bash
curl -L -o ~/.local/share/kiri/models/ggml-base.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
```
