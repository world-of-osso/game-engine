#!/usr/bin/env python3
"""Shared Linux/Windows player project and runtime data assembly. Never copies auth tokens."""

import argparse
import fnmatch
import itertools
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

DATA_DIRS = (
    'cache', 'campsite-ui', 'db2', 'dbfilesclient', 'fogs', 'fonts',
    'forever-1.60.1.70205', 'gametables', 'glues', 'item-models', 'los',
    'minimap', 'models', 'music', 'products', 'reference', 'sounds', 'terrain', 'textures', 'ui',
)
DATA_FILE_GLOBS = ('*.csv', '*.ron', 'local-listfile-cache.sqlite')
LIBRARIES = {
    'linux-x86_64': ('linux.x86_64', 'libgame_engine_godot.so'),
    'windows-x86_64': ('windows.x86_64', 'game_engine_godot.dll'),
}


def model_asset_files(data):
    cache = data / 'cache'
    indexes = [cache / 'model-asset-index.json', *sorted(cache.glob('model-asset-index-*.json'))]
    paths = set()
    for index in indexes:
        document = json.loads(index.read_text())
        if document['version'] != 1:
            raise ValueError(f'Unsupported model asset index version: {index}')
        for row in [*document['assets'], *document.get('aliases', [])]:
            path = Path(row['path'])
            namespace = Path('products') / row['product'] / row['build_key']
            if not path.is_relative_to(namespace) or '..' in path.parts:
                raise ValueError(f'Invalid asset namespace: {path}')
            if not (data / path).is_file():
                raise FileNotFoundError(f'Missing indexed model asset: {path}')
            paths.add(path)
            sidecar = Path(str(path) + '.receipt.json')
            if (data / sidecar).is_file():
                paths.add(sidecar)
    return sorted(paths)


def data_files(root):
    data = root / 'data'
    for name in DATA_DIRS:
        source = data / name
        if not source.is_dir():
            raise FileNotFoundError(f'Missing runtime data directory: {source}')
        if name == 'products':
            yield from model_asset_files(data)
            continue
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


def copy_selected(root, destination, files, link=False):
    if sys.platform != 'linux':
        for relative in files:
            copy_file(root / relative, destination / relative)
        return
    mode = ['--link'] if link else ['-p', '--reflink=auto']
    # Batch argv, not one cp process per asset (the shipped set exceeds a million files).
    files = iter(files)
    while batch := list(itertools.islice(files, 512)):
        subprocess.run(['cp', '-L', *mode, '--parents', '-t', str(destination),
                        *map(str, batch)], cwd=root, check=True)


def copy_data(root, bundle):
    for name in DATA_DIRS:
        (bundle / 'data' / name).mkdir(parents=True, exist_ok=True)
    # Hard links: the ~190 GiB data set is only read by the publish, and the host's ext4
    # cannot reflink, so copies would double its disk use. Nothing writes bundle data.
    copy_selected(root / 'data', bundle / 'data', data_files(root), link=True)


def project_files(root):
    output = subprocess.check_output(['git', '-C', str(root), 'ls-files', '-z', 'godot'])
    files = [Path(p.decode()) for p in output.split(b'\0') if p]
    crates = {p.parent for p in files if p.name == 'Cargo.toml' and p.parent != Path('godot')}
    for path in files:
        if any(parent in crates for parent in path.parents):
            continue
        if path.parts[1] == 'tests' or path.name in ('Cargo.toml', 'Cargo.lock', '.gitignore', 'depot-test-assets.txt', 'game_engine.gdextension'):
            continue
        yield path


def write_extension_config(bundle, platform):
    if platform not in LIBRARIES:
        raise ValueError(f'Unknown player platform: {platform}')
    libraries = '\n'.join(f'{selector} = "res://../lib/{library}"' for selector, library in LIBRARIES.values())
    project = bundle / 'godot'
    project.mkdir(parents=True, exist_ok=True)
    (project / 'game_engine.gdextension').write_text(
        '[configuration]\nentry_symbol = "gdext_rust_init"\ncompatibility_minimum = "4.7"\nreloadable = false\n\n'
        f'[libraries]\n{libraries}\n'
    )
    cache = project / '.godot'
    cache.mkdir(exist_ok=True)
    (cache / 'extension_list.cfg').write_text('res://game_engine.gdextension\n')


def copy_project(root, bundle, platform):
    bundle.mkdir(parents=True, exist_ok=True)
    copy_selected(root, bundle, project_files(root))
    write_extension_config(bundle, platform)


def copy_windows_native(artifacts, bundle):
    for source, target in (
        ('godot.exe', 'runtime/godot.exe'),
        ('game_engine_godot.dll', 'lib/game_engine_godot.dll'),
        ('game-engine-windows-x86_64.exe', 'game-engine-windows-x86_64'),
    ):
        copy_file(artifacts / source, bundle / target)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--platform', choices=LIBRARIES, required=True)
    parser.add_argument('--windows-artifacts', type=Path)
    args = parser.parse_args()
    copy_project(args.root, args.bundle, args.platform)
    copy_data(args.root, args.bundle)
    if args.windows_artifacts:
        copy_windows_native(args.windows_artifacts, args.bundle)


if __name__ == '__main__':
    main()
