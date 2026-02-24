#!/usr/bin/env bash
set -euo pipefail

# ── Kiri installer ──────────────────────────────────────────────────────────
# curl -fsSL https://raw.githubusercontent.com/koompiai/kiri/main/install.sh | bash

REPO="https://github.com/koompiai/kiri.git"
APP_DIR="$HOME/.local/share/kiri/app"
BIN_DIR="$HOME/.local/bin"
CMDS=(kiri kiri-popup kiri-sync)

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

    for cmd in "${CMDS[@]}"; do
        if [ -L "$BIN_DIR/$cmd" ]; then
            rm "$BIN_DIR/$cmd"
            info "Removed $BIN_DIR/$cmd"
        fi
    done

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

# ── Install uv ──────────────────────────────────────────────────────────────

if command -v uv &>/dev/null; then
    info "uv already installed ($(uv --version))"
else
    info "Installing uv..."
    curl -LsSf https://astral.sh/uv/install.sh | sh
    export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
    info "uv installed ($(uv --version))"
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
            brew install git gtk4 libadwaita portaudio
            ;;
        linux)
            if command -v pacman &>/dev/null; then
                info "Installing system deps via pacman..."
                sudo pacman -S --needed git gtk4 libadwaita portaudio
            elif command -v apt &>/dev/null; then
                info "Installing system deps via apt..."
                sudo apt install -y git libgtk-4-dev libadwaita-1-dev portaudio19-dev
            elif command -v dnf &>/dev/null; then
                info "Installing system deps via dnf..."
                sudo dnf install -y git gtk4-devel libadwaita-devel portaudio-devel
            elif command -v zypper &>/dev/null; then
                info "Installing system deps via zypper..."
                sudo zypper install -y git gtk4-devel libadwaita-devel portaudio-devel
            elif command -v apk &>/dev/null; then
                info "Installing system deps via apk..."
                sudo apk add git gtk4.0-dev libadwaita-dev portaudio-dev
            elif command -v xbps-install &>/dev/null; then
                info "Installing system deps via xbps..."
                sudo xbps-install -Sy git gtk4-devel libadwaita-devel portaudio-devel
            elif command -v emerge &>/dev/null; then
                info "Installing system deps via portage..."
                sudo emerge --noreplace dev-vcs/git gui-libs/gtk:4 gui-libs/libadwaita media-libs/portaudio
            else
                warn "Unknown package manager. Please install manually: git, gtk4, libadwaita, portaudio"
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

# ── uv sync ─────────────────────────────────────────────────────────────────

info "Installing Python dependencies..."
cd "$APP_DIR"
uv sync

# ── Create wrapper scripts ──────────────────────────────────────────────────

mkdir -p "$BIN_DIR"
VENV_BIN="$APP_DIR/.venv/bin"

for cmd in "${CMDS[@]}"; do
    cat > "$BIN_DIR/$cmd" <<WRAPPER
#!/bin/sh
exec "$VENV_BIN/$cmd" "\$@"
WRAPPER
    chmod +x "$BIN_DIR/$cmd"
    info "Created $BIN_DIR/$cmd"
done

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
ExecStart=$VENV_BIN/kiri-sync
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
    printf "  Bind AI key (Copilot key) to kiri-popup? [y/N] "
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
Exec=$BIN_DIR/kiri-popup
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

        info "AI key bound to kiri-popup (log out/in to activate)"
    fi
fi

# ── Done ────────────────────────────────────────────────────────────────────

echo
bold "Kiri installed successfully!"; echo
echo
echo "  Usage:"
echo "    kiri                  — transcribe from microphone"
echo "    kiri-popup            — GUI popup recorder"
echo "    kiri-sync             — download/update whisper models"
echo "    kiri --check          — verify setup"
echo
echo "  First run:"
echo "    kiri-sync             — download the default model"
echo
echo "  Uninstall:"
echo "    curl -fsSL https://raw.githubusercontent.com/koompiai/kiri/main/install.sh | bash -s -- --uninstall"
echo
