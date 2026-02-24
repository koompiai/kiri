"""First-run onboarding — ask user about notes storage preference."""

import subprocess
import sys

from kiri.config import NOTES_DIR
from kiri.sync import init_notes_repo, is_notes_repo


def needs_onboarding() -> bool:
    """True if ~/kiri/ doesn't exist yet (first run)."""
    return not NOTES_DIR.exists()


def run_onboarding() -> None:
    """Interactive first-run setup: local-only or GitHub sync."""
    print()
    print("Welcome to Kiri!")
    print()
    print(f"Your transcriptions will be saved to {NOTES_DIR}/")
    print()
    print("How would you like to store your notes?")
    print()
    print("  1) Local only — keep notes on this machine")
    print("  2) Sync to GitHub — back up notes to a private repo")
    print()

    choice = _ask("Choose [1/2]: ", valid=("1", "2"))

    if choice == "1":
        NOTES_DIR.mkdir(parents=True, exist_ok=True)
        print()
        print(f"Notes directory created: {NOTES_DIR}")
        print("You can enable GitHub sync later with: kiri-sync --setup")
        print()
    else:
        _setup_github_sync()


def _setup_github_sync() -> None:
    """Walk the user through connecting ~/kiri/ to a GitHub repo."""
    print()
    print("── GitHub sync setup ──")
    print()

    # Check if gh CLI is available for easy repo creation
    has_gh = _has_command("gh")

    if has_gh:
        print("GitHub CLI detected. You can:")
        print()
        print("  1) Create a new private repo automatically")
        print("  2) Use an existing repo URL")
        print()
        choice = _ask("Choose [1/2]: ", valid=("1", "2"))

        if choice == "1":
            _create_repo_with_gh()
            return

    # Manual URL entry
    print()
    print("Enter your GitHub repo URL (e.g. https://github.com/you/kiri-notes.git)")
    print()
    url = input("URL: ").strip()

    if not url:
        print("No URL provided. Creating local-only notes for now.")
        print("Run kiri-sync --setup to try again.")
        NOTES_DIR.mkdir(parents=True, exist_ok=True)
        return

    NOTES_DIR.mkdir(parents=True, exist_ok=True)
    init_notes_repo(url)
    print()
    print("Sync enabled! Notes will auto-commit after each transcription.")
    print("Push with: kiri-sync --push")
    print()


def _create_repo_with_gh() -> None:
    """Create a private GitHub repo using the gh CLI and set it up."""
    print()
    name = input("Repo name [kiri-notes]: ").strip() or "kiri-notes"

    result = subprocess.run(
        ["gh", "repo", "create", name, "--private", "--confirm"],
        capture_output=True, text=True,
    )

    if result.returncode != 0:
        # Try newer gh syntax without --confirm
        result = subprocess.run(
            ["gh", "repo", "create", name, "--private"],
            capture_output=True, text=True,
        )

    if result.returncode != 0:
        print(f"Failed to create repo: {result.stderr.strip()}")
        print("Create it manually on GitHub, then run: kiri-sync --setup")
        NOTES_DIR.mkdir(parents=True, exist_ok=True)
        return

    # Get the URL of the created repo
    url_result = subprocess.run(
        ["gh", "repo", "view", name, "--json", "url", "-q", ".url"],
        capture_output=True, text=True,
    )

    if url_result.returncode == 0:
        url = url_result.stdout.strip() + ".git"
    else:
        # Fallback: ask gh for authenticated user
        user_result = subprocess.run(
            ["gh", "api", "user", "-q", ".login"],
            capture_output=True, text=True,
        )
        username = user_result.stdout.strip() if user_result.returncode == 0 else "USER"
        url = f"https://github.com/{username}/{name}.git"

    print(f"Created private repo: {url}")

    NOTES_DIR.mkdir(parents=True, exist_ok=True)
    init_notes_repo(url)
    print()
    print("Sync enabled! Notes will auto-commit after each transcription.")
    print("Push with: kiri-sync --push")
    print()


def setup_sync_interactive() -> None:
    """Called by kiri-sync --setup. Works whether or not notes dir exists."""
    if is_notes_repo():
        from kiri.sync import status
        print("Sync is already configured:")
        print(status())
        return

    NOTES_DIR.mkdir(parents=True, exist_ok=True)
    _setup_github_sync()


def _ask(prompt: str, valid: tuple[str, ...]) -> str:
    """Prompt until we get a valid answer. Defaults to first option if non-interactive."""
    if not sys.stdin.isatty():
        return valid[0]
    while True:
        answer = input(prompt).strip()
        if answer in valid:
            return answer


def _has_command(cmd: str) -> bool:
    """Check if a command exists on PATH."""
    try:
        subprocess.run([cmd, "--version"], capture_output=True, check=True)
        return True
    except (FileNotFoundError, subprocess.CalledProcessError):
        return False
