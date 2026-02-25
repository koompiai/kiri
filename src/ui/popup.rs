use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use gtk4::prelude::*;
use gtk4::{self as gtk, gdk, glib, Application, ApplicationWindow, CssProvider, Label};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use super::states::State;
use super::styles::CSS;

/// Seconds between partial transcription attempts during speech.
/// Low because the tiny model is fast (~100ms).
const STREAM_INTERVAL: f32 = 1.5;
/// Seconds of silence after speech to finalize a segment and paste.
const SEGMENT_SILENCE: f32 = 1.0;
/// Minimum audio duration worth transcribing (seconds).
const MIN_SEGMENT: f32 = 0.5;

/// Messages sent from the background thread to the GTK main thread.
enum StateMsg {
    SetState(State),
    SetResult(String),
    SetError(String),
    AudioLevel(f32),
    PartialText(String),
    PasteText(String),
    NoteSaved(String),
    Quit,
}

/// Shared mutable state for the popup, accessible from closures.
struct PopupWidgets {
    dot: Label,
    status_label: Label,
    info_label: Label,
    hint: Label,
    level_fill: gtk::Box,
    level_frame: gtk::Box,
    stop_btn: gtk::Button,
    state: Cell<State>,
    pulse_phase: Cell<f64>,
    audio_level: Cell<f32>,
    has_speech: Cell<bool>,
    last_speech_time: Cell<f64>,
    note_mode: bool,
}

pub fn run_popup(lang: String, model_path: PathBuf, note_mode: bool) -> anyhow::Result<()> {
    let app = Application::builder()
        .application_id("com.kiri.popup")
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(move |app| {
        // Guard against duplicate activations (GTK single-instance re-activate)
        if app.active_window().is_some() {
            return;
        }
        build_ui(app, lang.clone(), model_path.clone(), note_mode);
    });

    // GTK application expects &[&str] args; pass empty since we use clap.
    let empty: Vec<String> = vec![];
    app.run_with_args(&empty);
    Ok(())
}

fn build_ui(app: &Application, lang: String, model_path: PathBuf, note_mode: bool) {
    // --- CSS ---
    let provider = CssProvider::new();
    provider.load_from_data(CSS);
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not get default display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // --- Window ---
    let win = ApplicationWindow::builder()
        .application(app)
        .title("Kiri")
        .default_width(340)
        .default_height(140)
        .resizable(false)
        .decorated(false)
        .build();

    // --- Layer shell ---
    win.init_layer_shell();
    win.set_layer(Layer::Top);
    win.set_anchor(Edge::Top, true);
    win.set_anchor(Edge::Right, true);
    win.set_margin(Edge::Top, 12);
    win.set_margin(Edge::Right, 12);
    win.set_keyboard_mode(KeyboardMode::None);

    // --- Widget tree ---

    // Container
    let container = gtk::Box::new(gtk::Orientation::Vertical, 10);
    container.add_css_class("container");

    // Top row: left group (dot + status) + stop button
    let top = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    top.set_halign(gtk::Align::Fill);

    let left = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    left.set_halign(gtk::Align::Center);
    left.set_hexpand(true);

    let dot = Label::new(Some("\u{25cf}"));
    dot.add_css_class("dot-loading");
    left.append(&dot);

    let loading_text = if note_mode {
        "Loading (private note)..."
    } else {
        "Loading model..."
    };
    let status_label = Label::new(Some(loading_text));
    status_label.add_css_class("status-label");
    left.append(&status_label);

    top.append(&left);

    let stop_btn = gtk::Button::with_label("\u{2715}");
    stop_btn.add_css_class("stop-btn");
    stop_btn.set_visible(false);
    top.append(&stop_btn);

    container.append(&top);

    // Level bar
    let level_frame = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    level_frame.add_css_class("level-bar-bg");
    level_frame.set_size_request(280, 8);
    level_frame.set_visible(false); // hidden during Loading

    let level_fill = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    level_fill.add_css_class("level-bar-fg");
    level_fill.set_size_request(0, 8);
    level_fill.set_hexpand(false);
    level_frame.append(&level_fill);

    container.append(&level_frame);

    // Info / result label
    let info_label = Label::new(None);
    info_label.add_css_class("result-text");
    info_label.set_wrap(true);
    info_label.set_max_width_chars(40);
    container.append(&info_label);

    // Hint label
    let hint = Label::new(None);
    hint.add_css_class("hint-label");
    hint.set_halign(gtk::Align::Center);
    hint.set_visible(false);
    container.append(&hint);

    win.set_child(Some(&container));

    // --- Shared state ---
    let widgets = Rc::new(PopupWidgets {
        dot,
        status_label,
        info_label,
        hint,
        level_fill,
        level_frame,
        stop_btn: stop_btn.clone(),
        state: Cell::new(State::Loading),
        pulse_phase: Cell::new(0.0),
        audio_level: Cell::new(0.0),
        has_speech: Cell::new(false),
        last_speech_time: Cell::new(0.0),
        note_mode,
    });

    // --- Stop flag shared between UI and background thread ---
    let stop_flag = Arc::new(AtomicBool::new(false));

    // --- Escape key handler ---
    let key_ctrl = gtk::EventControllerKey::new();
    {
        let stop = stop_flag.clone();
        key_ctrl.connect_key_pressed(move |_ctrl, keyval, _keycode, _modifier| {
            if keyval == gdk::Key::Escape {
                stop.store(true, Ordering::Relaxed);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    win.add_controller(key_ctrl);

    // --- Stop button handler ---
    {
        let stop = stop_flag.clone();
        stop_btn.connect_clicked(move |_| {
            stop.store(true, Ordering::Relaxed);
        });
    }

    // --- Background thread communication ---
    let (tx, rx) = mpsc::channel::<StateMsg>();

    // --- Pulse timer (80ms) -- also drains the message channel ---
    {
        let w = Rc::clone(&widgets);
        let app = app.clone();
        let epoch = Instant::now();
        glib::timeout_add_local(Duration::from_millis(80), move || {
            // Drain pending messages from background thread
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    StateMsg::SetState(state) => apply_state(&w, state, None),
                    StateMsg::SetResult(text) => apply_state(&w, State::Result, Some(&text)),
                    StateMsg::SetError(text) => apply_state(&w, State::Error, Some(&text)),
                    StateMsg::AudioLevel(level) => {
                        w.audio_level.set(level);
                        if level > crate::config::SILENCE_THRESHOLD {
                            w.has_speech.set(true);
                            w.last_speech_time.set(epoch.elapsed().as_secs_f64());
                        }
                    }
                    StateMsg::PasteText(text) => {
                        let _ = crate::output::typer::paste_text(&format!("{text} "));
                    }
                    StateMsg::NoteSaved(path) => {
                        apply_state(&w, State::Result, Some(&format!("Saved to {path}")));
                    }
                    StateMsg::PartialText(text) => {
                        let display = if text.len() > 100 {
                            format!("\u{2026}{}", &text[text.len() - 99..])
                        } else {
                            text
                        };
                        w.info_label.set_text(&display);
                    }
                    StateMsg::Quit => {
                        app.quit();
                        return glib::ControlFlow::Break;
                    }
                }
            }

            // Pulse animation
            let phase = w.pulse_phase.get() + 0.15;
            w.pulse_phase.set(phase);

            match w.state.get() {
                State::Listening => {
                    // Real audio level drives the bar
                    let level = w.audio_level.get();
                    let bar_width = (level.min(0.15) / 0.15 * 280.0) as i32;
                    w.level_fill.set_size_request(bar_width.max(2), 8);

                    // Show "Done in Xs" countdown after speech detected
                    if w.has_speech.get() {
                        let now = epoch.elapsed().as_secs_f64();
                        let elapsed_since_speech = now - w.last_speech_time.get();
                        let remaining =
                            (crate::config::DONE_TIMEOUT as f64 - elapsed_since_speech).max(0.0);
                        if remaining > 0.0 {
                            w.hint
                                .set_text(&format!("Done in {:.0}s...", remaining.ceil()));
                        } else {
                            w.hint.set_text("Finishing...");
                        }
                    }
                }
                State::Transcribing => {
                    let dots = ".".repeat((phase as usize) % 4);
                    w.status_label.set_text(&format!("Transcribing{dots}"));
                }
                _ => {}
            }

            glib::ControlFlow::Continue
        });
    }

    win.present();

    // --- Background thread: real streaming transcription ---
    {
        let stop = stop_flag.clone();
        std::thread::spawn(move || {
            streaming_transcription(tx, stop, lang, model_path, note_mode);
        });
    }
}

/// Background thread: two-model streaming transcription.
///
/// Loads the tiny model first (~0.5s) so the popup starts listening almost
/// instantly, then loads the medium model in the background. Tiny handles
/// live partial previews; medium (beam search) handles accurate finals.
fn streaming_transcription(
    tx: mpsc::Sender<StateMsg>,
    stop: Arc<AtomicBool>,
    lang: String,
    model_path: PathBuf,
    note_mode: bool,
) {
    macro_rules! send {
        ($msg:expr) => {
            if tx.send($msg).is_err() {
                return;
            }
        };
    }

    // --- Load tiny model first for instant responsiveness ---
    let tiny_path = crate::config::wake_model_path();
    let fast_engine = if tiny_path.exists() {
        match crate::transcribe::whisper::WhisperEngine::load(&tiny_path) {
            Ok(e) => Some(e),
            Err(e) => {
                eprintln!("Tiny model load failed, using main model only: {e}");
                None
            }
        }
    } else {
        None
    };

    // If we have the tiny model, start listening immediately while medium loads
    if fast_engine.is_some() {
        send!(StateMsg::SetState(State::Listening));
    }

    // --- Load medium model (in background if tiny is available) ---
    let main_engine_rx = {
        let (etx, erx) = mpsc::channel();
        let mp = model_path.clone();
        std::thread::spawn(move || {
            let _ = etx.send(crate::transcribe::whisper::WhisperEngine::load(&mp));
        });
        erx
    };

    // If we had no tiny model, we must wait for medium before we can proceed
    let (fast_engine, mut main_engine) = if let Some(fe) = fast_engine {
        (fe, None)
    } else {
        match main_engine_rx
            .recv()
            .unwrap_or_else(|_| Err(anyhow::anyhow!("Engine loader thread died")))
        {
            Ok(e) => {
                send!(StateMsg::SetState(State::Listening));
                // Use medium as both fast and main
                (e, None::<crate::transcribe::whisper::WhisperEngine>)
            }
            Err(e) => {
                send!(StateMsg::SetError(format!("Model load failed: {e}")));
                std::thread::sleep(Duration::from_secs(3));
                send!(StateMsg::Quit);
                return;
            }
        }
    };

    let capture = crate::audio::capture::AudioCapture::new();
    let audio_level = capture.audio_level.clone();

    // Start continuous audio stream
    let _stream = match capture.start_stream() {
        Ok(s) => s,
        Err(e) => {
            send!(StateMsg::SetError(format!("Audio error: {e}")));
            std::thread::sleep(Duration::from_secs(3));
            send!(StateMsg::Quit);
            return;
        }
    };

    // Level reporter: pumps audio levels to UI independently of transcription
    let level_done = Arc::new(AtomicBool::new(false));
    {
        let audio_level = audio_level.clone();
        let tx = tx.clone();
        let done = level_done.clone();
        std::thread::spawn(move || {
            while !done.load(Ordering::Relaxed) {
                let level = *audio_level.lock().unwrap();
                let _ = tx.send(StateMsg::AudioLevel(level));
                std::thread::sleep(Duration::from_millis(60));
            }
        });
    }

    let mut accumulated_text = String::new();
    let mut speech_onset: Option<Instant> = None;
    let mut had_speech = false;
    let mut last_speech_time: Option<Instant> = None;
    let mut last_transcribe = Instant::now();
    let mut new_speech = false;

    loop {
        std::thread::sleep(Duration::from_millis(100));

        if stop.load(Ordering::Relaxed) {
            break;
        }

        // Check if medium model finished loading in the background
        if main_engine.is_none() {
            if let Ok(result) = main_engine_rx.try_recv() {
                match result {
                    Ok(e) => main_engine = Some(e),
                    Err(e) => eprintln!("Medium model load failed: {e}"),
                }
            }
        }

        // Track speech/silence from audio level
        let level = capture.get_level();
        let now = Instant::now();

        if level > crate::config::SILENCE_THRESHOLD {
            // Track onset — require sustained speech before considering it real
            if speech_onset.is_none() {
                speech_onset = Some(now);
            }
            if let Some(onset) = speech_onset {
                if now.duration_since(onset).as_secs_f32()
                    >= crate::config::SPEECH_MIN_DURATION
                {
                    last_speech_time = Some(now);
                    had_speech = true;
                    new_speech = true;
                }
            }
        } else {
            // Reset onset on silence — brief noise bursts don't count
            speech_onset = None;
        }

        let silence_secs = last_speech_time
            .map(|t| now.duration_since(t).as_secs_f32())
            .unwrap_or(f32::MAX);

        // End session on long silence after speech
        if had_speech && silence_secs >= crate::config::DONE_TIMEOUT {
            break;
        }

        let audio = capture.snapshot();
        let duration = audio.len() as f32 / crate::config::RECORD_RATE as f32;

        // Max duration safeguard
        if duration >= crate::config::MAX_DURATION {
            if duration >= MIN_SEGMENT {
                send!(StateMsg::SetState(State::Transcribing));
                let audio_16k = crate::audio::resample::resample_48k_to_16k(&audio);
                // Use medium beam search if available, else fast greedy
                let result = if let Some(ref me) = main_engine {
                    me.transcribe(&audio_16k, &lang)
                } else {
                    fast_engine.transcribe_fast(&audio_16k, &lang)
                };
                if let Ok(text) = result {
                    let text = text.trim().to_string();
                    if !text.is_empty() && !is_hallucination(&text) {
                        if !note_mode {
                            send!(StateMsg::PasteText(text.clone()));
                        }
                        if !accumulated_text.is_empty() {
                            accumulated_text.push(' ');
                        }
                        accumulated_text.push_str(&text);
                    }
                }
            }
            break;
        }

        // Finalize segment: silence detected after speech, enough audio
        // Use medium model (beam search) for accuracy, fall back to tiny
        if had_speech && silence_secs >= SEGMENT_SILENCE && duration >= MIN_SEGMENT {
            send!(StateMsg::SetState(State::Transcribing));

            // Wait briefly for medium model if it's still loading
            if main_engine.is_none() {
                eprintln!("[kiri] Medium model not ready, waiting up to 3s...");
                if let Ok(result) = main_engine_rx.recv_timeout(Duration::from_secs(3)) {
                    match result {
                        Ok(e) => {
                            eprintln!("[kiri] Medium model loaded just in time");
                            main_engine = Some(e);
                        }
                        Err(e) => eprintln!("[kiri] Medium model failed: {e}"),
                    }
                }
            }

            let audio_16k = crate::audio::resample::resample_48k_to_16k(&audio);
            let result = if let Some(ref me) = main_engine {
                eprintln!("[kiri] Final transcription: medium model (beam search)");
                me.transcribe(&audio_16k, &lang)
            } else {
                eprintln!("[kiri] Final transcription: tiny model (greedy fallback)");
                fast_engine.transcribe_fast(&audio_16k, &lang)
            };
            match result {
                Ok(text) => {
                    let text = text.trim().to_string();
                    if !text.is_empty() && !is_hallucination(&text) {
                        if !note_mode {
                            send!(StateMsg::PasteText(text.clone()));
                        }
                        if !accumulated_text.is_empty() {
                            accumulated_text.push(' ');
                        }
                        accumulated_text.push_str(&text);
                        send!(StateMsg::PartialText(accumulated_text.clone()));
                    }
                }
                Err(e) => eprintln!("Transcription error: {e}"),
            }
            capture.clear_buffer();
            new_speech = false;
            speech_onset = None;
            last_transcribe = Instant::now();
            send!(StateMsg::SetState(State::Listening));
            continue;
        }

        // Periodic partial transcription while speaking (tiny model, very fast)
        if new_speech
            && duration >= 1.0
            && last_transcribe.elapsed().as_secs_f32() >= STREAM_INTERVAL
        {
            let audio_16k = crate::audio::resample::resample_48k_to_16k(&audio);
            match fast_engine.transcribe_fast(&audio_16k, &lang) {
                Ok(text) => {
                    let text = text.trim().to_string();
                    if !text.is_empty() && !is_hallucination(&text) {
                        let mut display = accumulated_text.clone();
                        if !display.is_empty() {
                            display.push(' ');
                        }
                        display.push_str(&text);
                        send!(StateMsg::PartialText(display));
                    }
                }
                Err(e) => eprintln!("Partial transcription error: {e}"),
            }
            last_transcribe = Instant::now();
        }
    }

    level_done.store(true, Ordering::Relaxed);

    if accumulated_text.is_empty() {
        send!(StateMsg::SetResult("No speech detected".to_string()));
    } else if note_mode {
        match crate::output::notes::save_to_notes(&accumulated_text) {
            Ok(path) => {
                send!(StateMsg::NoteSaved(path.display().to_string()));
            }
            Err(e) => {
                send!(StateMsg::SetError(format!("Failed to save note: {e}")));
            }
        }
    } else {
        send!(StateMsg::SetResult(accumulated_text));
    }

    std::thread::sleep(Duration::from_millis(1500));
    send!(StateMsg::Quit);
}

/// Detect common whisper hallucinations (empty/repeated noise artifacts).
fn is_hallucination(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    let hallucinations = [
        "you",
        "thank you.",
        "thanks for watching!",
        "thank you for watching!",
        "subscribe",
        "like and subscribe",
        "(silence)",
        "[silence]",
        "[blank_audio]",
        "...",
        "the end.",
        "bye.",
    ];
    hallucinations.iter().any(|h| t == *h)
        || t.chars().all(|c| c == '.' || c == ' ')
        || t.len() < 2
        || (t.starts_with('[') && t.ends_with(']'))
}

fn apply_state(w: &PopupWidgets, state: State, text: Option<&str>) {
    w.state.set(state);
    match state {
        State::Loading => {
            w.dot.set_text("\u{25cf}");
            w.dot.remove_css_class("dot-recording");
            w.dot.remove_css_class("dot-done");
            w.dot.add_css_class("dot-loading");
            w.status_label.set_text("Loading model...");
            w.level_frame.set_visible(false);
            w.stop_btn.set_visible(false);
            w.hint.set_visible(false);
        }
        State::Listening => {
            w.dot.set_text("\u{25cf}");
            w.dot.remove_css_class("dot-loading");
            w.dot.remove_css_class("dot-done");
            w.dot.add_css_class("dot-recording");
            w.status_label.set_text(if w.note_mode { "Private note..." } else { "Listening..." });
            w.level_frame.set_visible(true);
            w.stop_btn.set_visible(true);
            w.hint.set_text("Speak naturally...");
            w.hint.set_visible(true);
        }
        State::Transcribing => {
            w.dot.remove_css_class("dot-recording");
            w.dot.add_css_class("dot-loading");
            w.status_label.set_text("Transcribing...");
            w.level_frame.set_visible(false);
            w.hint.set_visible(false);
        }
        State::Result => {
            w.dot.remove_css_class("dot-loading");
            w.dot.remove_css_class("dot-recording");
            w.dot.add_css_class("dot-done");
            w.dot.set_text("\u{2713}");
            w.status_label.set_text("Done");
            w.info_label.set_text(text.unwrap_or(""));
            w.info_label.remove_css_class("error-label");
            w.info_label.add_css_class("result-text");
            w.level_frame.set_visible(false);
            w.stop_btn.set_visible(false);
            w.hint.set_visible(false);
        }
        State::Error => {
            w.dot.set_text("\u{2717}");
            w.dot.remove_css_class("dot-loading");
            w.dot.remove_css_class("dot-recording");
            w.dot.add_css_class("dot-done");
            w.status_label.set_text("Error");
            w.info_label.set_text(text.unwrap_or("Unknown error"));
            w.info_label.remove_css_class("result-text");
            w.info_label.add_css_class("error-label");
            w.level_frame.set_visible(false);
            w.stop_btn.set_visible(false);
            w.hint.set_visible(false);
        }
    }
}
