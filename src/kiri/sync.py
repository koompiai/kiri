"""Git sync for kiri notes — commit locally + push to GitHub."""

import subprocess
from pathlib import Path

from kiri.config import NOTES_DIR


def _git(args: list[str], check: bool = True) -> subprocess.CompletedProcess:
    """Run a git command in the notes directory."""
    return subprocess.run(
        ["git", *args],
        cwd=NOTES_DIR,
        capture_output=True,
        text=True,
        check=check,
    )


def is_notes_repo() -> bool:
    """Check if ~/kiri/ is a git repository."""
    return (NOTES_DIR / ".git").exists()


def init_notes_repo(remote_url: str) -> None:
    """Initialize ~/kiri/ as a git repo with a GitHub remote."""
    NOTES_DIR.mkdir(parents=True, exist_ok=True)

    if is_notes_repo():
        print(f"Already a git repo: {NOTES_DIR}")
        # Update remote if provided
        result = _git(["remote", "get-url", "origin"], check=False)
        if result.returncode != 0:
            _git(["remote", "add", "origin", remote_url])
            print(f"Added remote: {remote_url}")
        elif result.stdout.strip() != remote_url:
            _git(["remote", "set-url", "origin", remote_url])
            print(f"Updated remote: {remote_url}")
        return

    _git_init_args = ["init"]
    subprocess.run(
        ["git", "init"],
        cwd=NOTES_DIR,
        check=True,
    )
    # Rename default branch to main
    subprocess.run(
        ["git", "branch", "-m", "main"],
        cwd=NOTES_DIR,
        check=False,
    )
    _git(["remote", "add", "origin", remote_url])

    # Create .gitignore
    gitignore = NOTES_DIR / ".gitignore"
    if not gitignore.exists():
        gitignore.write_text("*.swp\n*.swo\n.DS_Store\n")

    # Initial commit
    _git(["add", "-A"])
    result = _git(["status", "--porcelain"])
    if result.stdout.strip():
        _git(["commit", "-m", "Initial commit from kiri"])
        print(f"Initialized notes repo at {NOTES_DIR}")
        print(f"Remote: {remote_url}")
    else:
        _git(["commit", "--allow-empty", "-m", "Initial commit from kiri"])
        print(f"Initialized empty notes repo at {NOTES_DIR}")


def commit_notes(message: str | None = None) -> bool:
    """Stage all changes in ~/kiri/ and commit. Returns True if a commit was made."""
    if not is_notes_repo():
        return False

    _git(["add", "-A"])
    result = _git(["diff", "--cached", "--quiet"], check=False)
    if result.returncode == 0:
        return False  # nothing to commit

    if message is None:
        import datetime
        message = f"kiri: notes update {datetime.datetime.now().strftime('%Y-%m-%d %H:%M')}"

    _git(["commit", "-m", message])
    return True


def push_notes() -> bool:
    """Push notes to the remote. Returns True on success."""
    if not is_notes_repo():
        print("Not a git repo. Run: kiri-sync --init <github-url>")
        return False

    # Check if remote exists
    result = _git(["remote", "get-url", "origin"], check=False)
    if result.returncode != 0:
        print("No remote configured. Run: kiri-sync --init <github-url>")
        return False

    result = _git(["push", "-u", "origin", "main"], check=False)
    if result.returncode != 0:
        print(f"Push failed: {result.stderr.strip()}")
        return False

    print("Pushed to remote.")
    return True


def status() -> str:
    """Return git status summary for the notes repo."""
    if not is_notes_repo():
        return f"Not a git repo: {NOTES_DIR}"

    lines = [f"Notes dir: {NOTES_DIR}"]

    result = _git(["remote", "get-url", "origin"], check=False)
    if result.returncode == 0:
        lines.append(f"Remote: {result.stdout.strip()}")
    else:
        lines.append("Remote: (none)")

    result = _git(["log", "--oneline", "-5"], check=False)
    if result.stdout.strip():
        lines.append(f"Recent commits:\n{result.stdout.strip()}")

    result = _git(["status", "--short"])
    if result.stdout.strip():
        lines.append(f"Uncommitted:\n{result.stdout.strip()}")
    else:
        lines.append("Working tree clean.")

    return "\n".join(lines)
