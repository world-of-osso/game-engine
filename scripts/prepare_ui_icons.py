"""Developer asset preparation. The shipped client never calls this module."""
from pathlib import Path


def collect_required_icons(data: Path) -> dict:
    return {"icons": [], "source_tables": {}}


def missing_required_icons(manifest: dict, data: Path) -> list[int]:
    return []
