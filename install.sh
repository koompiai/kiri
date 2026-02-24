#!/usr/bin/env bash
set -euo pipefail

# ── Kiri installer ──────────────────────────────────────────────────────────
# curl -fsSL https://raw.githubusercontent.com/koompiai/kiri/main/install.sh | bash

REPO="https://github.com/koompiai/kiri.git"
APP_DIR="$HOME/.local/share/kiri/app"
BIN_DIR="$HOME/.local/bin"
CMDS=(kiri)

# ── Colors ──────────────────────────────────────────────────────────────────

bold() { printf '\033[1m%s\033[0m' "$*"; }
green() { printf '\033[32m%s\033[0m' "$*"; }
yellow() { printf '\033[33m%s\033[0m' "$*"; }
red() { printf '\033[31m%s\033[0m' "$*"; }
info() { echo "  $(green "▸") $*"; }
warn() { echo "  $(yellow "▸") $*"; }
err()  { echo "  $(red "✗") $*" >&2; }

# ── Uninstall ───────────────────────────────────────────────────────────────

uninstall() {
    echo
    bold "Uninstalling Kiri..."; echo
    echo

    if [ -f "$BIN_DIR/kiri" ]; then
        rm "$BIN_DIR/kiri"
        info "Removed $BIN_DIR/kiri"
    fi

    if [ -d "$APP_DIR" ]; then
        rm -rf "$APP_DIR"
        info "Removed $APP_DIR"
    fi

    # systemd timer (Linux only)
    if [ -f "$HOME/.config/systemd/user/kiri-sync.timer" ]; then
        systemctl --user disable --now kiri-sync.timer 2>/dev/null || true
        rm -f "$HOME/.config/systemd/user/kiri-sync.timer"
        rm -f "$HOME/.config/systemd/user/kiri-sync.service"
        systemctl --user daemon-reload 2>/dev/null || true
        info "Removed systemd timer"
    fi

    # KDE shortcut + desktop entry
    if [ -f "$HOME/.local/share/applications/kiri-popup.desktop" ]; then
        rm -f "$HOME/.local/share/applications/kiri-popup.desktop"
        info "Removed desktop entry"
    fi
    if command -v kwriteconfig6 &>/dev/null; then
        kwriteconfig6 --file kglobalshortcutsrc --group "kiri-popup.desktop" --key "_launch" --delete 2>/dev/null || true
        kwriteconfig6 --file kglobalshortcutsrc --group "kiri-popup.desktop" --key "_k_friendly_name" --delete 2>/dev/null || true
    fi

    echo
    info "Uninstall complete. Notes in ~/kiri/ and models were kept."
    echo
    exit 0
}

SKIP_DEPS=false
LOCAL_SRC=""
for arg in "$@"; do
    case "$arg" in
        --uninstall) uninstall ;;
        --no-deps)   SKIP_DEPS=true ;;
        --local=*)   LOCAL_SRC="${arg#--local=}" ;;
    esac
done

# ── Detect OS ───────────────────────────────────────────────────────────────

OS="$(uname -s)"
case "$OS" in
    Linux)  PLATFORM=linux ;;
    Darwin) PLATFORM=macos ;;
    *)      err "Unsupported OS: $OS"; exit 1 ;;
esac

echo
bold "Installing Kiri ($PLATFORM)"; echo
echo

# ── Install Rust toolchain ─────────────────────────────────────────────────

if command -v cargo &>/dev/null; then
    info "Rust toolchain found ($(rustc --version))"
else
    info "Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    info "Rust installed ($(rustc --version))"
fi

# ── Install system dependencies ─────────────────────────────────────────────

install_system_deps() {
    case "$PLATFORM" in
        macos)
            if ! command -v brew &>/dev/null; then
                err "Homebrew not found. Install it first: https://brew.sh"
                exit 1
            fi
            info "Installing system deps via Homebrew..."
            brew install git cmake gtk4 libadwaita wl-clipboard ydotool
            ;;
        linux)
            if command -v pacman &>/dev/null; then
                info "Installing system deps via pacman..."
                sudo pacman -S --needed git cmake base-devel gtk4 libadwaita gtk4-layer-shell alsa-lib wl-clipboard ydotool
            elif command -v apt &>/dev/null; then
                info "Installing system deps via apt..."
                sudo apt install -y git cmake build-essential libgtk-4-dev libadwaita-1-dev libgtk-4-layer-shell-dev libasound2-dev wl-clipboard ydotool
            elif command -v dnf &>/dev/null; then
                info "Installing system deps via dnf..."
                sudo dnf install -y git cmake gcc-c++ gtk4-devel libadwaita-devel gtk4-layer-shell-devel alsa-lib-devel wl-clipboard ydotool
            elif command -v zypper &>/dev/null; then
                info "Installing system deps via zypper..."
                sudo zypper install -y git cmake gcc-c++ gtk4-devel libadwaita-devel wl-clipboard ydotool
            elif command -v apk &>/dev/null; then
                info "Installing system deps via apk..."
                sudo apk add git cmake build-base gtk4.0-dev libadwaita-dev alsa-lib-dev wl-clipboard ydotool
            elif command -v xbps-install &>/dev/null; then
                info "Installing system deps via xbps..."
                sudo xbps-install -Sy git cmake base-devel gtk4-devel libadwaita-devel alsa-lib-devel wl-clipboard ydotool
            elif command -v emerge &>/dev/null; then
                info "Installing system deps via portage..."
                sudo emerge --noreplace dev-vcs/git dev-util/cmake gui-libs/gtk:4 gui-libs/libadwaita media-libs/alsa-lib x11-misc/wl-clipboard app-misc/ydotool
            else
                warn "Unknown package manager. Please install manually: git, cmake, gtk4, libadwaita, alsa-lib, wl-clipboard, ydotool"
            fi
            ;;
    esac
}

if [ "$SKIP_DEPS" = false ]; then
    install_system_deps
else
    info "Skipping system deps (--no-deps)"
fi

# ── Clone or update repo ────────────────────────────────────────────────────

if [ -n "$LOCAL_SRC" ]; then
    info "Copying from local source: $LOCAL_SRC"
    mkdir -p "$(dirname "$APP_DIR")"
    rm -rf "$APP_DIR"
    cp -a "$LOCAL_SRC" "$APP_DIR"
elif [ -d "$APP_DIR/.git" ]; then
    info "Updating existing installation..."
    git -C "$APP_DIR" pull --ff-only
else
    info "Cloning kiri..."
    mkdir -p "$(dirname "$APP_DIR")"
    git clone "$REPO" "$APP_DIR"
fi

# ── Build ──────────────────────────────────────────────────────────────────

info "Building kiri (release)..."
cd "$APP_DIR"
cargo build --release

# ── Install binary ─────────────────────────────────────────────────────────

mkdir -p "$BIN_DIR"
cp "$APP_DIR/target/release/kiri" "$BIN_DIR/kiri"
chmod +x "$BIN_DIR/kiri"
info "Installed $BIN_DIR/kiri"

# ── Download whisper model ─────────────────────────────────────────────────

MODEL_DIR="$HOME/.local/share/kiri/models"
MODEL_FILE="$MODEL_DIR/ggml-medium.bin"

if [ -f "$MODEL_FILE" ]; then
    info "Whisper model already downloaded"
else
    mkdir -p "$MODEL_DIR"
    info "Downloading Whisper medium model (~1.5GB)..."
    curl -L --progress-bar -o "$MODEL_FILE" \
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"
    info "Model downloaded to $MODEL_FILE"
fi

# ── PATH check ──────────────────────────────────────────────────────────────

if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BIN_DIR"; then
    echo
    warn "$BIN_DIR is not in your PATH. Add this to your shell profile:"
    echo "    export PATH=\"$BIN_DIR:\$PATH\""
fi

# ── Linux: offer systemd timer ──────────────────────────────────────────────

if [ "$PLATFORM" = "linux" ] && command -v systemctl &>/dev/null; then
    echo
    printf "  Set up daily model sync (systemd timer)? [y/N] "
    # When piped from curl, stdin is the script itself — skip the prompt
    if [ -t 0 ]; then
        read -r answer
    else
        answer="n"
        echo "n (non-interactive)"
    fi

    if [ "$answer" = "y" ] || [ "$answer" = "Y" ]; then
        UNIT_DIR="$HOME/.config/systemd/user"
        mkdir -p "$UNIT_DIR"

        cat > "$UNIT_DIR/kiri-sync.service" <<EOF
[Unit]
Description=Kiri model sync

[Service]
Type=oneshot
ExecStart=$BIN_DIR/kiri sync
EOF

        cat > "$UNIT_DIR/kiri-sync.timer" <<EOF
[Unit]
Description=Daily Kiri model sync

[Timer]
OnCalendar=daily
Persistent=true

[Install]
WantedBy=timers.target
EOF

        systemctl --user daemon-reload
        systemctl --user enable --now kiri-sync.timer
        info "Systemd timer enabled (daily sync)"
    fi
fi

# ── KDE: offer AI key binding ────────────────────────────────────────────────

if [ -n "${KDE_SESSION_VERSION:-}" ] && command -v kwriteconfig6 &>/dev/null; then
    echo
    printf "  Bind AI key (Copilot key) to kiri popup? [y/N] "
    if [ -t 0 ]; then
        read -r answer
    else
        answer="n"
        echo "n (non-interactive)"
    fi

    if [ "$answer" = "y" ] || [ "$answer" = "Y" ]; then
        # Desktop entry
        mkdir -p "$HOME/.local/share/applications"
        cat > "$HOME/.local/share/applications/kiri-popup.desktop" <<DESKTOP
[Desktop Entry]
Name=Kiri Voice Popup
Comment=Voice-to-text assistant
Exec=$BIN_DIR/kiri popup
Icon=audio-input-microphone
Type=Application
Categories=Utility;AudioVideo;
Keywords=voice;transcribe;whisper;
DESKTOP

        # KDE global shortcut: Shift+Meta+F23 (AI/Copilot key)
        kwriteconfig6 --file kglobalshortcutsrc \
            --group "kiri-popup.desktop" \
            --key "_launch" "Shift+Meta+F23,none,Kiri Voice Popup"
        kwriteconfig6 --file kglobalshortcutsrc \
            --group "kiri-popup.desktop" \
            --key "_k_friendly_name" "Kiri Voice Popup"

        info "AI key bound to kiri popup (log out/in to activate)"
    fi
fi

# ── Done ────────────────────────────────────────────────────────────────────

echo
bold "Kiri installed successfully!"; echo
echo
echo "  Usage:"
echo "    kiri popup            — voice popup (default)"
echo "    kiri listen           — CLI transcription"
echo "    kiri sync             — notes git status"
echo
echo "  Uninstall:"
echo "    curl -fsSL https://raw.githubusercontent.com/koompiai/kiri/main/install.sh | bash -s -- --uninstall"
echo
