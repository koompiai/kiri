"""Save transcribed text to notes files."""

import datetime
from pathlib import Path

from kiri.config import NOTES_DIR


def save_to_notes(text: str, filename: str | None = None) -> Path:
    """Append transcribed text to a markdown file in ~/kiri/."""
    NOTES_DIR.mkdir(parents=True, exist_ok=True)

    if filename:
        filepath = NOTES_DIR / f"{filename}.md"
    else:
        today = datetime.date.today().strftime("%Y-%m-%d")
        filepath = NOTES_DIR / f"{today}.md"

    timestamp = datetime.datetime.now().strftime("%H:%M")

    is_new = not filepath.exists()
    with open(filepath, "a") as f:
        if is_new:
            f.write(f"# {filepath.stem}\n\n")
        f.write(f"<!-- {timestamp} -->\n{text}\n\n")

    # Auto-commit if notes dir is a git repo
    from kiri.sync import commit_notes, is_notes_repo
    if is_notes_repo():
        commit_notes()

    return filepath
