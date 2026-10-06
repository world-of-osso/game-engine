#!/usr/bin/env python3
"""Retired: fixed worktree slots must retain shared canonical asset links."""

import sys


if __name__ == "__main__":
    sys.exit(
        "first-use-data.py is retired: isolate/reset would break shared assets or "
        "delete canonical files. See docs/remote-builds.md#shared-worktree-data."
    )
