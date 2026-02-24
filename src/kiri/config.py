"""Kiri configuration — paths and audio constants."""

import sys
from pathlib import Path

# ── XDG-style paths ──────────────────────────────────────────────────────────

NOTES_DIR = Path.home() / "kiri"

if sys.platform == "darwin":
    MODELS_DIR = Path.home() / "Library" / "Application Support" / "kiri" / "models"
else:
    MODELS_DIR = Path.home() / ".local" / "share" / "kiri" / "models"

# ── Audio ────────────────────────────────────────────────────────────────────

RECORD_RATE = 48000    # what the mic actually supports
WHISPER_RATE = 16000   # what Whisper expects
CHANNELS = 1
DTYPE = "int16"
MIC_HW = None          # None = auto-detect default input device

# ── Model defaults ───────────────────────────────────────────────────────────

DEFAULT_MODEL = "whisper-large-v3-ov"

# ── Silence detection (GUI) ─────────────────────────────────────────────────

SILENCE_THRESHOLD = 0.008   # RMS below this = silence
SILENCE_DURATION = 2.5      # seconds of silence to auto-stop
SPEECH_MIN_DURATION = 0.5   # must detect speech for at least this long
MAX_DURATION = 120           # hard cap in seconds
