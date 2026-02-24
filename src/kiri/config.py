"""Kiri configuration — paths and audio constants."""

from pathlib import Path

# ── XDG-style paths ──────────────────────────────────────────────────────────

NOTES_DIR = Path.home() / "kiri"
MODELS_DIR = Path.home() / ".local" / "share" / "kiri" / "models"

# ── Audio ────────────────────────────────────────────────────────────────────

RECORD_RATE = 48000    # what the mic actually supports
WHISPER_RATE = 16000   # what Whisper expects
CHANNELS = 1
DTYPE = "int16"
MIC_HW = "hw:0,10"    # ALSA hardware name for the mic

# ── Model defaults ───────────────────────────────────────────────────────────

DEFAULT_MODEL = "whisper-large-v3-ov"

# ── Silence detection (GUI) ─────────────────────────────────────────────────

SILENCE_THRESHOLD = 0.008   # RMS below this = silence
SILENCE_DURATION = 2.5      # seconds of silence to auto-stop
SPEECH_MIN_DURATION = 0.5   # must detect speech for at least this long
MAX_DURATION = 120           # hard cap in seconds
