"""Register global keyboard shortcuts for kiri-popup."""

import os
import shutil
import subprocess
import sys
from pathlib import Path

# Known key presets (name -> KDE shortcut string)
KEY_PRESETS = {
    "copilot": "Shift+Meta+F23",
    "f9": "F9",
    "f10": "F10",
    "f12": "F12",
    "super+v": "Meta+V",
}

DESKTOP_ENTRY = "kiri-popup.desktop"
APP_DIR = Path.home() / ".local" / "share" / "applications"
BIN_DIR = Path.home() / ".local" / "bin"


def bind_key(key: str) -> None:
    """Register a global shortcut to launch kiri-popup.

    Args:
        key: A preset name (e.g. "copilot") or a KDE shortcut string
             (e.g. "Shift+Meta+F23", "Meta+V").
    """
    shortcut = KEY_PRESETS.get(key.lower(), key)
    de = _detect_de()

    if de == "kde":
        _bind_kde(shortcut)
    elif de == "gnome":
        _bind_gnome(shortcut)
    else:
        print(f"Unsupported desktop environment. Shortcut string: {shortcut}")
        print("Please add it manually in your DE's keyboard settings.")
        print(f"  Command: {BIN_DIR / 'kiri-popup'}")
        return

    print(f"Bound '{shortcut}' to kiri-popup")
    print("Log out and back in (or restart your DE) to activate.")


def unbind_key() -> None:
    """Remove the kiri-popup global shortcut."""
    de = _detect_de()

    if de == "kde":
        _unbind_kde()
    elif de == "gnome":
        _unbind_gnome()
    else:
        print("Unsupported desktop environment. Please remove the shortcut manually.")
        return

    print("Keybinding removed.")


def show_binding() -> None:
    """Show the current kiri-popup keybinding."""
    de = _detect_de()

    if de == "kde":
        result = subprocess.run(
            ["kreadconfig6", "--file", "kglobalshortcutsrc",
             "--group", DESKTOP_ENTRY, "--key", "_launch"],
            capture_output=True, text=True,
        )
        if result.returncode == 0 and result.stdout.strip():
            shortcut = result.stdout.strip().split(",")[0]
            if shortcut and shortcut != "none":
                print(f"Current binding: {shortcut}")
                return
        print("No keybinding set. Use: kiri-popup --bind copilot")
    elif de == "gnome":
        bindings = _gnome_get_custom_bindings()
        for path in bindings:
            name = subprocess.run(
                ["gsettings", "get", f"org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{path}",
                 "name"], capture_output=True, text=True,
            ).stdout.strip().strip("'")
            if name == "Kiri Voice Popup":
                binding = subprocess.run(
                    ["gsettings", "get",
                     f"org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{path}",
                     "binding"], capture_output=True, text=True,
                ).stdout.strip().strip("'")
                print(f"Current binding: {binding}")
                return
        print("No keybinding set. Use: kiri-popup --bind copilot")
    else:
        print("Unsupported desktop environment.")


# ── KDE ─────────────────────────────────────────────────────────────────────

def _bind_kde(shortcut: str) -> None:
    _ensure_desktop_entry()

    subprocess.run([
        "kwriteconfig6", "--file", "kglobalshortcutsrc",
        "--group", DESKTOP_ENTRY,
        "--key", "_launch",
        f"{shortcut},none,Kiri Voice Popup",
    ], check=True)
    subprocess.run([
        "kwriteconfig6", "--file", "kglobalshortcutsrc",
        "--group", DESKTOP_ENTRY,
        "--key", "_k_friendly_name", "Kiri Voice Popup",
    ], check=True)


def _unbind_kde() -> None:
    subprocess.run([
        "kwriteconfig6", "--file", "kglobalshortcutsrc",
        "--group", DESKTOP_ENTRY,
        "--key", "_launch", "--delete",
    ], capture_output=True)
    subprocess.run([
        "kwriteconfig6", "--file", "kglobalshortcutsrc",
        "--group", DESKTOP_ENTRY,
        "--key", "_k_friendly_name", "--delete",
    ], capture_output=True)


# ── GNOME ───────────────────────────────────────────────────────────────────

def _bind_gnome(shortcut: str) -> None:
    bindings = _gnome_get_custom_bindings()

    # Find existing kiri binding or create new slot
    kiri_path = None
    for path in bindings:
        name = subprocess.run(
            ["gsettings", "get",
             f"org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{path}",
             "name"], capture_output=True, text=True,
        ).stdout.strip().strip("'")
        if name == "Kiri Voice Popup":
            kiri_path = path
            break

    if kiri_path is None:
        idx = len(bindings)
        kiri_path = f"/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom{idx}/"
        bindings.append(kiri_path)
        subprocess.run([
            "gsettings", "set", "org.gnome.settings-daemon.plugins.media-keys",
            "custom-keybindings", str(bindings),
        ], check=True)

    schema = f"org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{kiri_path}"
    subprocess.run(["gsettings", "set", schema, "name", "Kiri Voice Popup"], check=True)
    subprocess.run(["gsettings", "set", schema, "command", str(BIN_DIR / "kiri-popup")], check=True)
    subprocess.run(["gsettings", "set", schema, "binding", shortcut], check=True)


def _unbind_gnome() -> None:
    bindings = _gnome_get_custom_bindings()
    new_bindings = []
    for path in bindings:
        name = subprocess.run(
            ["gsettings", "get",
             f"org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{path}",
             "name"], capture_output=True, text=True,
        ).stdout.strip().strip("'")
        if name != "Kiri Voice Popup":
            new_bindings.append(path)
    subprocess.run([
        "gsettings", "set", "org.gnome.settings-daemon.plugins.media-keys",
        "custom-keybindings", str(new_bindings),
    ], check=True)


def _gnome_get_custom_bindings() -> list[str]:
    result = subprocess.run(
        ["gsettings", "get", "org.gnome.settings-daemon.plugins.media-keys",
         "custom-keybindings"], capture_output=True, text=True,
    )
    raw = result.stdout.strip()
    if raw == "@as []" or not raw:
        return []
    # Parse ['path1', 'path2'] format
    return [s.strip().strip("'") for s in raw.strip("[]").split(",") if s.strip()]


# ── Helpers ─────────────────────────────────────────────────────────────────

def _detect_de() -> str:
    desktop = os.environ.get("XDG_CURRENT_DESKTOP", "").lower()
    if "kde" in desktop or os.environ.get("KDE_SESSION_VERSION"):
        return "kde"
    if "gnome" in desktop:
        return "gnome"
    # Fallback: check for tools
    if shutil.which("kwriteconfig6"):
        return "kde"
    if shutil.which("gsettings"):
        return "gnome"
    return "unknown"


def _ensure_desktop_entry() -> None:
    APP_DIR.mkdir(parents=True, exist_ok=True)
    entry = APP_DIR / DESKTOP_ENTRY
    if not entry.exists():
        entry.write_text(
            "[Desktop Entry]\n"
            "Name=Kiri Voice Popup\n"
            "Comment=Voice-to-text assistant\n"
            f"Exec={BIN_DIR / 'kiri-popup'}\n"
            "Icon=audio-input-microphone\n"
            "Type=Application\n"
            "Categories=Utility;AudioVideo;\n"
            "Keywords=voice;transcribe;whisper;\n"
        )
