"""VoicePopup — Siri-style GTK4 overlay for voice transcription."""

import math
import threading

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
gi.require_version("Gtk4LayerShell", "1.0")
from gi.repository import Gdk, GLib, Gtk, Gtk4LayerShell

from kiri.audio.recorder import AudioRecorder
from kiri.config import WHISPER_RATE
from kiri.gui.states import State
from kiri.gui.styles import CSS
from kiri.output.notes import save_to_notes
from kiri.transcription.engine import WhisperEngine


class VoicePopup(Gtk.Application):
    def __init__(self, language: str, model_name: str, device: str):
        super().__init__(application_id="com.kiri.popup")
        self.language = language
        self.model_name = model_name
        self.ov_device = device
        self.state = State.LOADING
        self.recorder = AudioRecorder()
        self.engine = WhisperEngine(model_name=model_name, device=device)
        self.pulse_phase = 0.0

    def do_activate(self):
        # CSS
        provider = Gtk.CssProvider()
        provider.load_from_string(CSS)
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(), provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
        )

        # Window
        self.win = Gtk.ApplicationWindow(application=self, title="Kiri")
        self.win.set_default_size(340, 140)
        self.win.set_resizable(False)
        self.win.set_decorated(False)

        # Layer shell: anchor top-right
        Gtk4LayerShell.init_for_window(self.win)
        Gtk4LayerShell.set_layer(self.win, Gtk4LayerShell.Layer.TOP)
        Gtk4LayerShell.set_anchor(self.win, Gtk4LayerShell.Edge.TOP, True)
        Gtk4LayerShell.set_anchor(self.win, Gtk4LayerShell.Edge.RIGHT, True)
        Gtk4LayerShell.set_margin(self.win, Gtk4LayerShell.Edge.TOP, 12)
        Gtk4LayerShell.set_margin(self.win, Gtk4LayerShell.Edge.RIGHT, 12)

        # Escape key
        key_ctrl = Gtk.EventControllerKey()
        key_ctrl.connect("key-pressed", self._on_key)
        self.win.add_controller(key_ctrl)

        # Layout
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        box.add_css_class("container")

        # Top row: dot + status
        top = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        top.set_halign(Gtk.Align.CENTER)
        self.dot = Gtk.Label(label="\u25cf")
        self.dot.add_css_class("dot-loading")
        top.append(self.dot)
        self.status_label = Gtk.Label(label="Loading model...")
        self.status_label.add_css_class("status-label")
        top.append(self.status_label)
        box.append(top)

        # Level bar
        level_frame = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        level_frame.add_css_class("level-bar-bg")
        level_frame.set_size_request(280, 8)
        self.level_fill = Gtk.Box()
        self.level_fill.add_css_class("level-bar-fg")
        self.level_fill.set_size_request(0, 8)
        self.level_fill.set_hexpand(False)
        level_frame.append(self.level_fill)
        self.level_frame = level_frame
        box.append(level_frame)

        # Result / info label
        self.info_label = Gtk.Label(label="")
        self.info_label.add_css_class("result-text")
        self.info_label.set_wrap(True)
        self.info_label.set_max_width_chars(40)
        box.append(self.info_label)

        # Hint
        self.hint = Gtk.Label(label="Esc to stop")
        self.hint.add_css_class("hint-label")
        self.hint.set_halign(Gtk.Align.CENTER)
        self.hint.set_visible(False)
        box.append(self.hint)

        self.win.set_child(box)
        self.win.present()

        # Pulse animation
        GLib.timeout_add(80, self._pulse_tick)

        # Load model + record in background
        threading.Thread(target=self._load_and_record, daemon=True).start()

    def _on_key(self, controller, keyval, keycode, state):
        if keyval == Gdk.KEY_Escape:
            if self.state == State.LISTENING:
                self.recorder.stop_event.set()
            else:
                self.quit()
            return True
        return False

    def _pulse_tick(self):
        self.pulse_phase += 0.15
        if self.state == State.LISTENING:
            width = int(self.recorder.audio_level * 280 * 5)
            width = min(width, 280)
            self.level_fill.set_size_request(width, 8)
        elif self.state == State.TRANSCRIBING:
            dots = "." * (int(self.pulse_phase) % 4)
            self.status_label.set_text(f"Transcribing{dots}")
        return True

    def _set_state(self, state: State, **kwargs):
        def _update():
            self.state = state
            if state == State.LOADING:
                self.dot.set_text("\u25cf")
                self.dot.remove_css_class("dot-recording")
                self.dot.remove_css_class("dot-done")
                self.dot.add_css_class("dot-loading")
                self.status_label.set_text("Loading model...")
                self.level_frame.set_visible(False)
            elif state == State.LISTENING:
                self.dot.set_text("\u25cf")
                self.dot.remove_css_class("dot-loading")
                self.dot.remove_css_class("dot-done")
                self.dot.add_css_class("dot-recording")
                self.status_label.set_text("Listening...")
                self.level_frame.set_visible(True)
                self.info_label.set_text("")
                self.hint.set_visible(True)
            elif state == State.TRANSCRIBING:
                self.dot.remove_css_class("dot-recording")
                self.dot.add_css_class("dot-loading")
                self.status_label.set_text("Transcribing...")
                self.level_frame.set_visible(False)
                self.hint.set_visible(False)
            elif state == State.RESULT:
                self.dot.remove_css_class("dot-loading")
                self.dot.remove_css_class("dot-recording")
                self.dot.add_css_class("dot-done")
                self.dot.set_text("\u2713")
                text = kwargs.get("text", "")
                self.status_label.set_text("Saved")
                self.info_label.set_text(text)
                self.info_label.remove_css_class("error-label")
                self.info_label.add_css_class("result-text")
                self.level_frame.set_visible(False)
            elif state == State.ERROR:
                self.dot.set_text("\u2717")
                self.dot.remove_css_class("dot-loading")
                self.dot.remove_css_class("dot-recording")
                self.dot.add_css_class("dot-done")
                msg = kwargs.get("message", "Error")
                self.status_label.set_text("Error")
                self.info_label.set_text(msg)
                self.info_label.remove_css_class("result-text")
                self.info_label.add_css_class("error-label")
                self.level_frame.set_visible(False)
        GLib.idle_add(_update)

    def _load_and_record(self):
        # Load model
        try:
            self._set_state(State.LOADING)
            self.engine.load()
        except Exception as e:
            self._set_state(State.ERROR, message=str(e))
            GLib.timeout_add(3000, lambda: self.quit())
            return

        # Record with silence detection
        self._set_state(State.LISTENING)
        try:
            audio = self.recorder.record_with_silence()
        except Exception as e:
            self._set_state(State.ERROR, message=f"Recording failed: {e}")
            GLib.timeout_add(3000, lambda: self.quit())
            return

        if len(audio) < WHISPER_RATE:
            self._set_state(State.ERROR, message="Too short, discarded")
            GLib.timeout_add(2000, lambda: self.quit())
            return

        # Transcribe
        self._set_state(State.TRANSCRIBING)
        try:
            text = self.engine.transcribe(audio, language=self.language)
        except Exception as e:
            self._set_state(State.ERROR, message=f"Transcription failed: {e}")
            GLib.timeout_add(3000, lambda: self.quit())
            return

        if not text:
            self._set_state(State.ERROR, message="No speech detected")
            GLib.timeout_add(2000, lambda: self.quit())
            return

        # Save
        filepath = save_to_notes(text)
        self._set_state(State.RESULT, text=text, filepath=str(filepath))
        GLib.timeout_add(3000, lambda: self.quit())
