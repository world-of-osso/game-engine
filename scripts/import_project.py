#!/usr/bin/env python3
"""Import a generated Godot project with read-only source data."""

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def import_project(root, generated, godot):
    data = generated / 'data'
    data.symlink_to(root / 'data', target_is_directory=True)
    try:
        with tempfile.TemporaryDirectory() as home:
            env = dict(os.environ, XDG_CONFIG_HOME=f'{home}/config',
                       XDG_DATA_HOME=f'{home}/data', XDG_CACHE_HOME=f'{home}/cache')
            subprocess.run(['bwrap', '--ro-bind', '/', '/', '--dev', '/dev', '--bind', str(generated), str(generated),
                            '--bind', home, home, '--', str(godot), '--headless',
                            '--path', str(generated / 'godot'), '--import'], env=env, check=True)
    finally:
        data.unlink()
    editor = generated / 'godot/.godot/editor'
    if editor.exists():
        shutil.rmtree(editor)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--generated', type=Path, required=True)
    parser.add_argument('--godot', type=Path, required=True, help='host editor used for import')
    args = parser.parse_args()
    import_project(args.root, args.generated, args.godot)


if __name__ == '__main__':
    main()
