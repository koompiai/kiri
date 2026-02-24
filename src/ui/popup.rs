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

/// Short silence duration for chunk boundaries in streaming mode.
const CHUNK_SILENCE: f32 = 1.0;

/// Messages sent from the background thread to the GTK main thread.
enum StateMsg {
    SetState(State),
    SetResult(String),
    SetError(String),
    AudioLevel(f32),
    PartialText(String),
    PasteText(String),
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
}

pub fn run_popup(lang: String, model_path: PathBuf) -> anyhow::Result<()> {
    let app = Application::builder()
        .application_id("com.kiri.popup")
        .build();

    app.connect_activate(move |app| {
        build_ui(app, lang.clone(), model_path.clone());
    });

    // GTK application expects &[&str] args; pass empty since we use clap.
    let empty: Vec<String> = vec![];
    app.run_with_args(&empty);
    Ok(())
}

fn build_ui(app: &Application, lang: String, model_path: PathBuf) {
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

    let status_label = Label::new(Some("Loading model..."));
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
            streaming_transcription(tx, stop, lang, model_path);
        });
    }
}

/// Background thread: load model, capture audio in chunks, transcribe, paste.
fn streaming_transcription(
    tx: mpsc::Sender<StateMsg>,
    stop: Arc<AtomicBool>,
    lang: String,
    model_path: PathBuf,
) {
    // Helper to send and bail on closed channel
    macro_rules! send {
        ($msg:expr) => {
            if tx.send($msg).is_err() {
                return; // UI closed
            }
        };
    }

    // --- Load whisper model ---
    let engine = match crate::transcribe::whisper::WhisperEngine::load(&model_path) {
        Ok(e) => e,
        Err(e) => {
            send!(StateMsg::SetError(format!("Model load failed: {e}")));
            std::thread::sleep(Duration::from_secs(3));
            send!(StateMsg::Quit);
            return;
        }
    };

    send!(StateMsg::SetState(State::Listening));

    let capture = crate::audio::capture::AudioCapture::new();
    let capture_stop = capture.stop_flag();
    let audio_level = capture.audio_level.clone();

    let mut accumulated_text = String::new();
    let mut had_speech = false;
    let mut last_speech_time = Instant::now();

    // --- Chunk loop ---
    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        // Spawn a watcher that propagates the UI stop flag to AudioCapture
        // so record_with_silence_opts unblocks promptly when user hits Stop/Escape.
        let chunk_done = Arc::new(AtomicBool::new(false));
        {
            let stop = stop.clone();
            let capture_stop = capture_stop.clone();
            let chunk_done = chunk_done.clone();
            std::thread::spawn(move || {
                loop {
                    if chunk_done.load(Ordering::Relaxed) {
                        break;
                    }
                    if stop.load(Ordering::Relaxed) {
                        capture_stop.store(true, Ordering::Relaxed);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            });
        }

        // Spawn a level reporter that sends audio levels to the UI
        {
            let audio_level = audio_level.clone();
            let tx = tx.clone();
            let chunk_done = chunk_done.clone();
            std::thread::spawn(move || {
                loop {
                    if chunk_done.load(Ordering::Relaxed) {
                        break;
                    }
                    let level = *audio_level.lock().unwrap();
                    let _ = tx.send(StateMsg::AudioLevel(level));
                    std::thread::sleep(Duration::from_millis(60));
                }
            });
        }

        // Record one chunk with short silence boundary
        let raw_audio = match capture.record_with_silence_opts(CHUNK_SILENCE) {
            Ok(audio) => audio,
            Err(e) => {
                chunk_done.store(true, Ordering::Relaxed);
                send!(StateMsg::SetError(format!("Audio error: {e}")));
                std::thread::sleep(Duration::from_secs(3));
                send!(StateMsg::Quit);
                return;
            }
        };

        // Signal helper threads to exit
        chunk_done.store(true, Ordering::Relaxed);

        // If user requested stop, break immediately
        if stop.load(Ordering::Relaxed) {
            break;
        }

        // Check if chunk has meaningful audio (at least 0.3s at 48kHz)
        let min_samples = (crate::config::RECORD_RATE as f32 * 0.3) as usize;
        if raw_audio.len() < min_samples {
            if had_speech {
                // Had speech before but this chunk was too short -- session done
                break;
            }
            continue;
        }

        // Check done timeout: if we had speech and it's been too long since last text
        if had_speech
            && last_speech_time.elapsed().as_secs_f32() >= crate::config::DONE_TIMEOUT
        {
            break;
        }

        // Resample 48k -> 16k
        send!(StateMsg::SetState(State::Transcribing));
        let audio_16k = crate::audio::resample::resample_48k_to_16k(&raw_audio);

        // Transcribe
        let text = match engine.transcribe(&audio_16k, &lang) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Transcription error: {e}");
                send!(StateMsg::SetState(State::Listening));
                continue;
            }
        };

        send!(StateMsg::SetState(State::Listening));

        // Skip empty or hallucinated results
        let text = text.trim().to_string();
        if text.is_empty() || is_hallucination(&text) {
            if had_speech
                && last_speech_time.elapsed().as_secs_f32() >= crate::config::DONE_TIMEOUT
            {
                break;
            }
            continue;
        }

        // We got real text
        had_speech = true;
        last_speech_time = Instant::now();

        // Paste this chunk into the active app
        send!(StateMsg::PasteText(text.clone()));

        // Update accumulated display
        if !accumulated_text.is_empty() {
            accumulated_text.push(' ');
        }
        accumulated_text.push_str(&text);
        send!(StateMsg::PartialText(accumulated_text.clone()));
    }

    // Session complete
    if accumulated_text.is_empty() {
        send!(StateMsg::SetResult("No speech detected".to_string()));
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
        "...",
        "the end.",
        "bye.",
    ];
    hallucinations.iter().any(|h| t == *h)
        || t.chars().all(|c| c == '.' || c == ' ')
        || t.len() < 2
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
            w.status_label.set_text("Listening...");
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
