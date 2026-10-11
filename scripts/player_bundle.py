#!/usr/bin/env python3
"""Shared Linux/Windows player project and runtime data assembly. Never copies auth tokens."""

import argparse
import fnmatch
import os
import shutil
import subprocess
import sys
from pathlib import Path

DATA_DIRS = (
    'cache', 'campsite-ui', 'db2', 'dbfilesclient', 'fogs', 'fonts',
    'forever-1.60.1.70205', 'gametables', 'glues', 'item-models', 'los',
    'minimap', 'models', 'music', 'reference', 'sounds', 'terrain', 'textures', 'ui',
)
DATA_FILE_GLOBS = ('*.csv', '*.ron', 'local-listfile-cache.sqlite')
LIBRARIES = {
    'linux-x86_64': ('linux.x86_64', 'libgame_engine_godot.so'),
    'windows-x86_64': ('windows.x86_64', 'game_engine_godot.dll'),
}


def data_files(root):
    data = root / 'data'
    for name in DATA_DIRS:
        source = data / name
        if not source.is_dir():
            raise FileNotFoundError(f'Missing runtime data directory: {source}')
        for directory, subdirs, files in os.walk(source, followlinks=True):
            if Path(directory) == data / 'cache':
                subdirs[:] = [name for name in subdirs if name != 'casc' and not fnmatch.fnmatch(name, 'pre-retail-*')]
            for filename in files:
                if not filename.endswith(('.missing', '.lock')):
                    yield (Path(directory) / filename).relative_to(data)
    for pattern in DATA_FILE_GLOBS:
        for path in data.glob(pattern):
            if path.is_file():
                yield path.relative_to(data)


def copy_file(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    # Preserve Linux's cheap reflink assembly; Windows uses ordinary file copies.
    if sys.platform == 'linux':
        subprocess.run(['cp', '-Lp', '--reflink=auto', str(source), str(target)], check=True)
    else:
        shutil.copy2(source, target)


def copy_data(root, bundle):
    for name in DATA_DIRS:
        (bundle / 'data' / name).mkdir(parents=True, exist_ok=True)
    for relative in data_files(root):
        copy_file(root / 'data' / relative, bundle / 'data' / relative)


def project_files(root):
    output = subprocess.check_output(['git', '-C', str(root), 'ls-files', '-z', 'godot'])
    files = [Path(p.decode()) for p in output.split(b'\0') if p]
    crates = {p.parent for p in files if p.name == 'Cargo.toml'}
    for path in files:
        if any(parent in crates for parent in path.parents):
            continue
        if path.parts[1] == 'tests' or path.name in ('Cargo.toml', 'Cargo.lock', '.gitignore', 'depot-test-assets.txt', 'game_engine.gdextension'):
            continue
        yield path


def write_extension_config(bundle, platform):
    selector, library = LIBRARIES[platform]
    project = bundle / 'godot'
    project.mkdir(parents=True, exist_ok=True)
    (project / 'game_engine.gdextension').write_text(
        '[configuration]\nentry_symbol = "gdext_rust_init"\ncompatibility_minimum = "4.7"\nreloadable = false\n\n'
        f'[libraries]\n{selector} = "res://../lib/{library}"\n'
    )
    cache = project / '.godot'
    cache.mkdir(exist_ok=True)
    (cache / 'extension_list.cfg').write_text('res://game_engine.gdextension\n')


def copy_project(root, bundle, platform):
    for relative in project_files(root):
        copy_file(root / relative, bundle / relative)
    write_extension_config(bundle, platform)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--platform', choices=LIBRARIES, required=True)
    args = parser.parse_args()
    copy_project(args.root, args.bundle, args.platform)
    copy_data(args.root, args.bundle)


if __name__ == '__main__':
    main()
