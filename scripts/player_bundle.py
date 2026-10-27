#!/usr/bin/env python3
"""Shared Linux/Windows player project and runtime data assembly. Never copies auth tokens."""

import argparse
import datetime
import filecmp
import fnmatch
import hashlib
import itertools
import json
import os
import shutil
import sqlite3
import subprocess
import sys
from contextlib import closing
from pathlib import Path

DATA_DIRS = (
    'cache', 'campsite-ui', 'db2', 'dbfilesclient', 'fogs', 'fonts',
    'forever-1.60.1.70205', 'gametables', 'glues', 'item-models', 'los',
    'minimap', 'models', 'music', 'products', 'reference', 'sounds', 'terrain', 'textures', 'ui',
)
DATA_FILE_GLOBS = ('*.csv', '*.ron', 'local-listfile-cache.sqlite', 'battle-pet-model-scenes.json')
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
    # Catalogs can change while the client runs; only immutable payloads share storage.
    files = iter(data_files(root))
    while batch := list(itertools.islice(files, 512)):
        copied, linked = [], []
        for relative in batch:
            selected = copied if is_mutable_data(relative) else linked
            selected.append(relative)
        copy_selected(root / 'data', bundle / 'data', copied)
        copy_selected(root / 'data', bundle / 'data', linked, link=True)


def is_mutable_data(relative):
    catalog_directory = relative.parts[0] in (
        'cache', 'db2', 'dbfilesclient', 'gametables', 'reference',
    )
    catalog_file = relative.suffix.lower() in ('.sqlite', '.csv', '.ron', '.json', '.txt')
    return catalog_directory or catalog_file


def snapshot_sqlite_sources(paths, snapshot_root):
    """Use consistent SQLite backups for mutable database sources."""
    selected = paths.copy()
    retained = set()
    for name, source in paths.items():
        relative = Path(name)
        if relative.parts[0] != 'data' or relative.suffix.lower() != '.sqlite':
            continue
        destination = snapshot_root / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        temporary = destination.with_suffix(destination.suffix + '.part')
        temporary.unlink(missing_ok=True)
        try:
            with closing(sqlite3.connect(source.resolve().as_uri() + '?mode=ro', uri=True)) as original:
                with closing(sqlite3.connect(temporary)) as snapshot:
                    original.backup(snapshot)
            if destination.exists() and filecmp.cmp(destination, temporary, shallow=False):
                temporary.unlink()
            else:
                temporary.replace(destination)
        except (OSError, sqlite3.Error) as error:
            temporary.unlink(missing_ok=True)
            raise ValueError(f'Cannot snapshot SQLite source {name}: {error}') from error
        retained.add(destination)
        selected[name] = destination
    if snapshot_root.exists():
        for path in snapshot_root.rglob('*'):
            if path.is_file() and path not in retained:
                path.unlink()
    return selected


def source_identity(stat):
    return (stat.st_dev, stat.st_ino, stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns)


def manifest_order(item):
    name = item[0]
    relative = Path(name)
    mutable = relative.parts[0] == 'data' and is_mutable_data(relative.relative_to('data'))
    return mutable, name


def mapped_manifest(paths, cache_path, source_stats=None):
    cache = json.loads(cache_path.read_text()) if cache_path.exists() else {}
    fresh = {}
    files = []
    for name, path in sorted(paths.items(), key=manifest_order):
        stat = path.stat()
        cached = cache.get(name)
        identity = source_identity(stat)
        if cached and cached.get('identity') == list(identity):
            digest = cached['sha256']
        else:
            with path.open('rb') as stream:
                digest = hashlib.file_digest(stream, 'sha256').hexdigest()
            after = path.stat()
            if identity != source_identity(after):
                raise ValueError(f'Source changed during hashing: {name}')
        fresh[name] = {'size': stat.st_size, 'mtime_ns': stat.st_mtime_ns,
                       'identity': identity, 'sha256': digest}
        if source_stats is not None:
            source_stats[name] = identity
        files.append({'path': name, 'sha256': digest, 'size': stat.st_size})
    cache_path.parent.mkdir(parents=True, exist_ok=True)
    cache_path.write_text(json.dumps(fresh, separators=(',', ':')))
    files.sort(key=lambda entry: entry['path'])
    return {'version': datetime.date.today().isoformat(), 'files': files}


def verify_mapped_manifest(paths, manifest, only=None):
    for entry in manifest['files']:
        name = entry['path']
        if only is not None and name not in only:
            continue
        path = paths[name]
        if path.stat().st_size != entry['size']:
            raise ValueError(f'Source size mismatch: {name}')
        with path.open('rb') as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        if digest != entry['sha256']:
            raise ValueError(f'Source SHA256 mismatch: {name}')


def verify_manifest(bundle, only=None):
    """Re-hash staged files against manifest.json, or just the paths listed in `only`:
    publishing uploads only those, so only their bytes become published content."""
    manifest = json.loads((bundle / 'manifest.json').read_text())
    entries = manifest['files']
    if only is not None:
        listed = set(only.read_text().splitlines())
        entries = [entry for entry in entries if entry['path'] in listed]
    for entry in entries:
        path = bundle / entry['path']
        if path.stat().st_size != entry['size']:
            raise ValueError(f"Staged file size mismatch: {entry['path']}")
        with path.open('rb') as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        if digest != entry['sha256']:
            raise ValueError(f"Staged file SHA256 mismatch: {entry['path']}")
    return len(entries)


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


def source_map(root, generated, windows_artifacts):
    """Map shipped paths to source files without copying the runtime data."""
    files = {path.as_posix(): root / path for path in project_files(root)}
    files.update({(Path('data') / path).as_posix(): root / 'data' / path
                  for path in data_files(root)})
    for source, target in (
        ('godot.exe', 'runtime/godot.exe'),
        ('game_engine_godot.dll', 'lib/game_engine_godot.dll'),
        ('game-engine-windows-x86_64.exe', 'game-engine-windows-x86_64'),
    ):
        files[target] = windows_artifacts / source
    for path in generated.rglob('*'):
        if path.is_file():
            files[path.relative_to(generated).as_posix()] = path
    return files


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
    parser.add_argument('--project-only', action='store_true')
    parser.add_argument('--verify-manifest', action='store_true')
    parser.add_argument('--only', type=Path, help='with --verify-manifest: newline list of paths to check')
    args = parser.parse_args()
    if args.verify_manifest:
        try:
            count = verify_manifest(args.bundle, args.only)
        except (OSError, ValueError) as error:
            parser.exit(1, f'{error}\n')
        print(f'Verified {count} staged files against manifest.json')
        return
    copy_project(args.root, args.bundle, args.platform)
    if args.project_only:
        return
    copy_data(args.root, args.bundle)
    if args.windows_artifacts:
        copy_windows_native(args.windows_artifacts, args.bundle)


if __name__ == '__main__':
    main()
