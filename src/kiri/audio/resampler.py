"""Audio resampling — 48 kHz mic input to 16 kHz Whisper input."""

import numpy as np
from scipy.signal import resample_poly


def resample_48k_to_16k(audio: np.ndarray) -> np.ndarray:
    """Resample float32 audio from 48000 Hz to 16000 Hz (ratio 1:3)."""
    return resample_poly(audio, 1, 3)
