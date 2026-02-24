"""Audio recording — fixed-duration and silence-detection modes."""

import sys
import threading
import time

import numpy as np
import sounddevice as sd

from kiri.audio.mic import find_mic_device
from kiri.audio.resampler import resample_48k_to_16k
from kiri.config import (
    CHANNELS,
    DTYPE,
    MAX_DURATION,
    RECORD_RATE,
    SILENCE_DURATION,
    SILENCE_THRESHOLD,
    SPEECH_MIN_DURATION,
)


class AudioRecorder:
    """Records audio from the mic and resamples to 16 kHz for Whisper."""

    def __init__(self):
        self.frames: list[np.ndarray] = []
        self.audio_level = 0.0
        self.stop_event = threading.Event()

    def record_fixed(self, duration: int = 60) -> np.ndarray:
        """Record for a fixed duration (Ctrl+C to stop early). Returns 16 kHz float32."""
        self.frames.clear()
        device = find_mic_device()

        def callback(indata, frame_count, time_info, status):
            if status:
                print(f"  {status}", file=sys.stderr)
            self.frames.append(indata.copy())

        print(f"\U0001f399\ufe0f  Recording... (max {duration}s, Ctrl+C to stop early)\n")
        with sd.InputStream(
            samplerate=RECORD_RATE,
            channels=CHANNELS,
            dtype=DTYPE,
            device=device,
            callback=callback,
        ):
            try:
                threading.Event().wait(timeout=duration)
            except KeyboardInterrupt:
                pass
        print("\u23f9\ufe0f  Recording stopped.\n")

        return self._finalize()

    def record_with_silence(self) -> np.ndarray:
        """Record until silence is detected after speech. Returns 16 kHz float32."""
        self.frames.clear()
        self.stop_event.clear()
        device = find_mic_device()
        speech_detected = False
        silence_start: float | None = None
        speech_start: float | None = None

        def callback(indata, frame_count, time_info, status):
            nonlocal speech_detected, silence_start, speech_start
            self.frames.append(indata.copy())
            rms = np.sqrt(np.mean(indata.astype(np.float32) ** 2)) / 32768.0
            self.audio_level = rms

            now = time.monotonic()

            if rms > SILENCE_THRESHOLD:
                silence_start = None
                if not speech_detected:
                    speech_detected = True
                    speech_start = now
            elif speech_detected:
                if speech_start and (now - speech_start) < SPEECH_MIN_DURATION:
                    return
                if silence_start is None:
                    silence_start = now
                elif now - silence_start >= SILENCE_DURATION:
                    self.stop_event.set()

        with sd.InputStream(
            samplerate=RECORD_RATE,
            channels=CHANNELS,
            dtype=DTYPE,
            device=device,
            callback=callback,
        ):
            self.stop_event.wait(timeout=MAX_DURATION)

        return self._finalize()

    def _finalize(self) -> np.ndarray:
        """Concatenate frames, convert to float32, resample to 16 kHz."""
        if not self.frames:
            return np.array([], dtype=np.float32)

        audio = np.concatenate(self.frames, axis=0)
        if audio.ndim == 2 and audio.shape[1] > 1:
            audio = audio[:, 0]
        audio = audio.flatten().astype(np.float32) / 32768.0
        return resample_48k_to_16k(audio)
