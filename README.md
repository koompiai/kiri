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

## Keybinding

Bind the popup to a key so you can record with a single press:

```bash
# Bind to the Copilot/AI key (Lenovo, etc.)
kiri-popup --bind copilot

# Or any key combo
kiri-popup --bind "Meta+V"
kiri-popup --bind F9

# Check current binding
kiri-popup --keybinding

# Remove binding
kiri-popup --unbind
```

Supports KDE Plasma and GNOME. Log out and back in to activate.

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
