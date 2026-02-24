# Kiri

Voice-to-text assistant powered by OpenVINO Whisper on Intel GPU.

## Usage

```bash
# CLI — record and save to ~/kiri/
kiri -d 10 -l en

# GUI — Siri-style popup with silence detection
kiri-popup

# Check OpenVINO devices
kiri --check
```

## Install (development)

```bash
uv sync
uv run kiri --check
```
