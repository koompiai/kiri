"""Copy text to system clipboard."""

import subprocess


def copy_to_clipboard(text: str) -> None:
    """Copy text to clipboard — pbcopy (macOS), wl-copy (Wayland), or xclip (X11)."""
    for cmd, label in [
        (["pbcopy"], "macOS"),
        (["wl-copy"], "Wayland"),
        (["xclip", "-selection", "clipboard"], "X11"),
    ]:
        try:
            subprocess.run(cmd, input=text.encode(), check=True)
            print(f"\U0001f4cb Copied to clipboard ({label})")
            return
        except (FileNotFoundError, subprocess.CalledProcessError):
            continue

    print("\u26a0\ufe0f  No clipboard tool found. Install pbcopy, wl-clipboard, or xclip.")
