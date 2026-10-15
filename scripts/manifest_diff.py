#!/usr/bin/env python3
"""Write the upload and delete lists that turn the live R2 bundle into the new one.

A path is uploaded when the new manifest names it and the live manifest does not name it
with the same (sha256, size); it is deleted when only the live manifest names it. Both
lists are rclone --files-from inputs relative to the bundle root. A missing live manifest
(first publish) uploads every path and deletes nothing.

usage: manifest_diff.py --new <bundle>/manifest.json [--live <live manifest.json>]
                        --upload <file> --delete <file>
"""
import argparse
import json
from pathlib import Path


def manifest_entries(path: Path) -> dict[str, tuple[str, int]]:
    files = json.loads(path.read_text())["files"]
    return {entry["path"]: (entry["sha256"], entry["size"]) for entry in files}


def diff_manifests(
    live: dict[str, tuple[str, int]], new: dict[str, tuple[str, int]]
) -> tuple[list[str], list[str]]:
    upload = sorted(path for path, identity in new.items() if live.get(path) != identity)
    delete = sorted(path for path in live if path not in new)
    return upload, delete


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--new", type=Path, required=True)
    parser.add_argument("--live", type=Path)
    parser.add_argument("--upload", type=Path, required=True)
    parser.add_argument("--delete", type=Path, required=True)
    args = parser.parse_args()
    live = manifest_entries(args.live) if args.live and args.live.exists() else {}
    upload, delete = diff_manifests(live, manifest_entries(args.new))
    args.upload.write_text("".join(f"{path}\n" for path in upload))
    args.delete.write_text("".join(f"{path}\n" for path in delete))
    print(f"{len(upload)} to upload, {len(delete)} to delete, {len(live)} live paths")


if __name__ == "__main__":
    main()
