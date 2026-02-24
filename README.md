# Kiri

Voice-to-text assistant powered by OpenVINO Whisper. Works on Linux and macOS.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/koompiai/kiri/main/install.sh | bash
```

Then download a model:

```bash
kiri-sync
```

## Usage

```bash
# CLI — record and save to ~/kiri/
kiri -d 10 -l en

# GUI — Siri-style popup with silence detection
kiri-popup

# Check OpenVINO devices
kiri --check

# Sync notes to GitHub
kiri-sync --status
```

## Uninstall

```bash
curl -fsSL https://raw.githubusercontent.com/koompiai/kiri/main/install.sh | bash -s -- --uninstall
```

This removes the app and CLI wrappers but keeps your notes (`~/kiri/`) and models.

## Development

```bash
git clone https://github.com/koompiai/kiri.git
cd kiri
uv sync
uv run kiri --check
```
