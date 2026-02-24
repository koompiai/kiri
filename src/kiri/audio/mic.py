"""Microphone device discovery."""

import sounddevice as sd

from kiri.config import MIC_HW


def find_mic_device() -> int:
    """Find the mic device index by ALSA hardware name."""
    for i, dev in enumerate(sd.query_devices()):
        if MIC_HW in dev["name"] and dev["max_input_channels"] > 0:
            return i
    raise RuntimeError(
        f"Microphone {MIC_HW} not found. Available inputs: "
        + ", ".join(
            f'{i}: {d["name"]}'
            for i, d in enumerate(sd.query_devices())
            if d["max_input_channels"] > 0
        )
    )
