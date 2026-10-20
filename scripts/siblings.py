"""Locate sibling repository checkouts (game-server, shared-protocol, ...)."""

import os
from pathlib import Path


def sibling_repo(root, name):
    """`root`'s sibling checkout `name`, or `DEPOT_SIBLING_<NAME>` (for example
    DEPOT_SIBLING_UI_TOOLKIT for a toolkit branch worktree)."""
    override = os.environ.get("DEPOT_SIBLING_" + name.upper().replace("-", "_"))
    return Path(override).resolve() if override else root.parent / name
