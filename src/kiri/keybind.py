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

NOTES_PRESETS = {
    "copilot": "Ctrl+Shift+Meta+F23",
    "f9": "Ctrl+F9",
    "f10": "Ctrl+F10",
    "f12": "Ctrl+F12",
    "super+v": "Ctrl+Meta+V",
}

DESKTOP_ENTRY = "kiri-popup.desktop"
DESKTOP_ENTRY_NOTES = "kiri-popup-notes.desktop"
APP_DIR = Path.home() / ".local" / "share" / "applications"
BIN_DIR = Path.home() / ".local" / "bin"


def bind_key(key: str) -> None:
    """Register global shortcuts to launch kiri-popup in type and notes modes.

    Args:
        key: A preset name (e.g. "copilot") or a KDE shortcut string
             (e.g. "Shift+Meta+F23", "Meta+V").
    """
    shortcut = KEY_PRESETS.get(key.lower(), key)
    notes_shortcut = NOTES_PRESETS.get(key.lower(), f"Ctrl+{key}")
    de = _detect_de()

    if de == "kde":
        _bind_kde(shortcut, notes_shortcut)
    elif de == "gnome":
        _bind_gnome(shortcut, notes_shortcut)
    else:
        print("Unsupported desktop environment.")
        print(f"  Type mode: {shortcut}  ->  {BIN_DIR / 'kiri-popup'} --mode type")
        print(f"  Notes mode: {notes_shortcut}  ->  {BIN_DIR / 'kiri-popup'} --mode notes")
        return

    print(f"Bound '{shortcut}' -> type mode (voice typing)")
    print(f"Bound '{notes_shortcut}' -> notes mode (save to ~/kiri/)")
    print("Log out and back in (or restart your DE) to activate.")


def unbind_key() -> None:
    """Remove the kiri-popup global shortcuts."""
    de = _detect_de()

    if de == "kde":
        _unbind_kde()
    elif de == "gnome":
        _unbind_gnome()
    else:
        print("Unsupported desktop environment. Please remove the shortcuts manually.")
        return

    print("Keybindings removed.")


def show_binding() -> None:
    """Show the current kiri-popup keybindings."""
    de = _detect_de()

    if de == "kde":
        for entry, label in [(DESKTOP_ENTRY, "Type"), (DESKTOP_ENTRY_NOTES, "Notes")]:
            result = subprocess.run(
                ["kreadconfig6", "--file", "kglobalshortcutsrc",
                 "--group", entry, "--key", "_launch"],
                capture_output=True, text=True,
            )
            if result.returncode == 0 and result.stdout.strip():
                shortcut = result.stdout.strip().split(",")[0]
                if shortcut and shortcut != "none":
                    print(f"{label} mode: {shortcut}")
                    continue
            print(f"{label} mode: not set")
    elif de == "gnome":
        bindings = _gnome_get_custom_bindings()
        found = False
        for path in bindings:
            name = subprocess.run(
                ["gsettings", "get",
                 f"org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{path}",
                 "name"], capture_output=True, text=True,
            ).stdout.strip().strip("'")
            if "Kiri" in name:
                binding = subprocess.run(
                    ["gsettings", "get",
                     f"org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{path}",
                     "binding"], capture_output=True, text=True,
                ).stdout.strip().strip("'")
                print(f"{name}: {binding}")
                found = True
        if not found:
            print("No keybinding set. Use: kiri-popup --bind copilot")
    else:
        print("Unsupported desktop environment.")


# ── KDE ─────────────────────────────────────────────────────────────────────

def _bind_kde(shortcut: str, notes_shortcut: str) -> None:
    # Type mode binding
    _ensure_desktop_entry(DESKTOP_ENTRY, "type")
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

    # Notes mode binding
    _ensure_desktop_entry(DESKTOP_ENTRY_NOTES, "notes")
    subprocess.run([
        "kwriteconfig6", "--file", "kglobalshortcutsrc",
        "--group", DESKTOP_ENTRY_NOTES,
        "--key", "_launch",
        f"{notes_shortcut},none,Kiri Voice Notes",
    ], check=True)
    subprocess.run([
        "kwriteconfig6", "--file", "kglobalshortcutsrc",
        "--group", DESKTOP_ENTRY_NOTES,
        "--key", "_k_friendly_name", "Kiri Voice Notes",
    ], check=True)


def _unbind_kde() -> None:
    for entry in (DESKTOP_ENTRY, DESKTOP_ENTRY_NOTES):
        subprocess.run([
            "kwriteconfig6", "--file", "kglobalshortcutsrc",
            "--group", entry, "--key", "_launch", "--delete",
        ], capture_output=True)
        subprocess.run([
            "kwriteconfig6", "--file", "kglobalshortcutsrc",
            "--group", entry, "--key", "_k_friendly_name", "--delete",
        ], capture_output=True)
    # Clean up desktop entries
    for name in (DESKTOP_ENTRY, DESKTOP_ENTRY_NOTES):
        entry = APP_DIR / name
        if entry.exists():
            entry.unlink()


# ── GNOME ───────────────────────────────────────────────────────────────────

def _bind_gnome(shortcut: str, notes_shortcut: str) -> None:
    bindings = _gnome_get_custom_bindings()

    for mode, sc, label in [
        ("type", shortcut, "Kiri Voice Popup"),
        ("notes", notes_shortcut, "Kiri Voice Notes"),
    ]:
        kiri_path = None
        for path in bindings:
            name = subprocess.run(
                ["gsettings", "get",
                 f"org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{path}",
                 "name"], capture_output=True, text=True,
            ).stdout.strip().strip("'")
            if name == label:
                kiri_path = path
                break

        if kiri_path is None:
            idx = len(bindings)
            kiri_path = f"/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom{idx}/"
            bindings.append(kiri_path)

        schema = f"org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{kiri_path}"
        subprocess.run(["gsettings", "set", schema, "name", label], check=True)
        subprocess.run(["gsettings", "set", schema, "command",
                        f"{BIN_DIR / 'kiri-popup'} --mode {mode}"], check=True)
        subprocess.run(["gsettings", "set", schema, "binding", sc], check=True)

    subprocess.run([
        "gsettings", "set", "org.gnome.settings-daemon.plugins.media-keys",
        "custom-keybindings", str(bindings),
    ], check=True)


def _unbind_gnome() -> None:
    bindings = _gnome_get_custom_bindings()
    new_bindings = []
    for path in bindings:
        name = subprocess.run(
            ["gsettings", "get",
             f"org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{path}",
             "name"], capture_output=True, text=True,
        ).stdout.strip().strip("'")
        if "Kiri" not in name:
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


def _ensure_desktop_entry(entry_name: str, mode: str) -> None:
    APP_DIR.mkdir(parents=True, exist_ok=True)
    entry = APP_DIR / entry_name
    label = "Kiri Voice Popup" if mode == "type" else "Kiri Voice Notes"
    entry.write_text(
        "[Desktop Entry]\n"
        f"Name={label}\n"
        "Comment=Voice-to-text assistant\n"
        f"Exec={BIN_DIR / 'kiri-popup'} --mode {mode}\n"
        "Icon=audio-input-microphone\n"
        "Type=Application\n"
        "Categories=Utility;AudioVideo;\n"
        "Keywords=voice;transcribe;whisper;\n"
    )
