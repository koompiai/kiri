use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::{self as gtk, gdk, glib, Application, ApplicationWindow, CssProvider, Label};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use super::states::State;
use super::styles::CSS;

/// Messages sent from the background thread to the GTK main thread.
enum StateMsg {
    SetState(State),
    SetResult(String),
    #[allow(dead_code)]
    SetError(String),
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

fn build_ui(app: &Application, _lang: String, _model_path: PathBuf) {
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
    });

    // --- Escape key handler ---
    let key_ctrl = gtk::EventControllerKey::new();
    {
        let app = app.clone();
        key_ctrl.connect_key_pressed(move |_ctrl, keyval, _keycode, _modifier| {
            if keyval == gdk::Key::Escape {
                app.quit();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    win.add_controller(key_ctrl);

    // --- Stop button handler ---
    {
        let app = app.clone();
        stop_btn.connect_clicked(move |_| {
            app.quit();
        });
    }

    // --- Background thread communication ---
    // Use std::sync::mpsc; the pulse timer polls for messages on the GTK thread.
    let (tx, rx) = mpsc::channel::<StateMsg>();

    // --- Pulse timer (80ms) — also drains the message channel ---
    {
        let w = Rc::clone(&widgets);
        let app = app.clone();
        glib::timeout_add_local(Duration::from_millis(80), move || {
            // Drain pending messages from background thread
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    StateMsg::SetState(state) => apply_state(&w, state, None),
                    StateMsg::SetResult(text) => apply_state(&w, State::Result, Some(&text)),
                    StateMsg::SetError(text) => apply_state(&w, State::Error, Some(&text)),
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
                    // Simulate a gentle pulse for the level bar (real audio level comes in Task 7)
                    let pulse_width = ((phase.sin() + 1.0) / 2.0 * 140.0) as i32;
                    w.level_fill.set_size_request(pulse_width, 8);
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

    // --- Dummy background thread (proves UI works end-to-end) ---
    std::thread::spawn(move || {
        // Simulate: Loading (2s) -> Listening (5s) -> Result (3s) -> quit
        std::thread::sleep(Duration::from_secs(2));
        let _ = tx.send(StateMsg::SetState(State::Listening));

        std::thread::sleep(Duration::from_secs(5));
        let _ = tx.send(StateMsg::SetResult("Test transcription text".to_string()));

        std::thread::sleep(Duration::from_secs(3));
        let _ = tx.send(StateMsg::Quit);
    });
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
