"""Microphone device discovery."""

import sounddevice as sd

from kiri.config import MIC_HW


def find_mic_device() -> int:
    """Find the mic device index.

    If MIC_HW is set, search for it by ALSA hardware name.
    Otherwise (or if not found), fall back to the system default input device.
    """
    if MIC_HW is not None:
        for i, dev in enumerate(sd.query_devices()):
            if MIC_HW in dev["name"] and dev["max_input_channels"] > 0:
                return i

    # Fall back to system default input device
    default = sd.default.device[0]
    if default is not None and default >= 0:
        return default

    raise RuntimeError(
        "No input device found. Available inputs: "
        + ", ".join(
            f'{i}: {d["name"]}'
            for i, d in enumerate(sd.query_devices())
            if d["max_input_channels"] > 0
        )
    )
