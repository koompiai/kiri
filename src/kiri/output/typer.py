"""Type text into the focused application via virtual keyboard."""

import shutil
import subprocess


def type_text(text: str) -> bool:
    """Type text into the focused window using wtype (Wayland) or xdotool (X11).

    Returns True if typing succeeded, False if no tool is available.
    """
    if shutil.which("wtype"):
        subprocess.run(["wtype", "--", text], check=True)
        return True

    if shutil.which("xdotool"):
        subprocess.run(["xdotool", "type", "--clearmodifiers", "--", text], check=True)
        return True

    return False
