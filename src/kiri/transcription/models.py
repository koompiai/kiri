"""Model path resolution and listing."""

from pathlib import Path

from kiri.config import DEFAULT_MODEL, MODELS_DIR


def model_path(name: str = DEFAULT_MODEL) -> Path:
    """Return the full path for a model by name."""
    return MODELS_DIR / name


def check_model(name: str = DEFAULT_MODEL) -> bool:
    """Check if a model directory exists."""
    p = model_path(name)
    if not p.exists():
        print(f"\u274c Model not found at {p}")
        return False
    return True


def list_models() -> list[str]:
    """List available model directories."""
    if not MODELS_DIR.exists():
        return []
    return sorted(
        d.name for d in MODELS_DIR.iterdir()
        if d.is_dir() and d.name.startswith("whisper-")
    )
