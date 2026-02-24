# Kiri

Voice-to-text assistant powered by whisper.cpp.

## Build

```bash
cargo build --release
```

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/koompiai/kiri/main/install.sh | bash
```

## Usage

```bash
kiri popup            # Voice popup — transcribe and paste into active app
kiri listen           # CLI transcription to stdout
kiri sync             # Notes git sync status
```

## Requirements

- Linux (Wayland)
- GTK4 + gtk4-layer-shell
- ydotool + wl-clipboard (for paste)
- Whisper GGML model (~1.5GB, downloaded during install)
