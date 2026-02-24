"""Copy text to system clipboard."""

import subprocess


def copy_to_clipboard(text: str) -> None:
    """Copy text to clipboard using wl-copy (Wayland) or xclip (X11)."""
    try:
        subprocess.run(["wl-copy"], input=text.encode(), check=True)
        print("\U0001f4cb Copied to clipboard (Wayland)")
    except (FileNotFoundError, subprocess.CalledProcessError):
        try:
            subprocess.run(
                ["xclip", "-selection", "clipboard"],
                input=text.encode(), check=True,
            )
            print("\U0001f4cb Copied to clipboard (X11)")
        except FileNotFoundError:
            print("\u26a0\ufe0f  No clipboard tool found. Install wl-clipboard or xclip.")
