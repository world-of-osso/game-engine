#!/usr/bin/env python3
"""Shared Linux/Windows player project and runtime data assembly. Never copies auth tokens."""

import argparse
import csv
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
DATA_FILE_GLOBS = (
    '*.csv', '*.ron', 'local-listfile-cache.sqlite',
    'battle-pet-model-scenes.json', 'battle-pets.json',
)
LIBRARIES = {
    'linux-x86_64': ('linux.x86_64', 'libgame_engine_godot.so'),
    'windows-x86_64': ('windows.x86_64', 'game_engine_godot.dll'),
}


def model_index_state(root):
    cache = root / 'data/cache'
    indexes = [cache / 'model-asset-index.json', *sorted(cache.glob('model-asset-index-*.json'))]
    return {path.name: source_identity(path.stat()) for path in indexes}


def verify_model_index_state(root, expected):
    actual = model_index_state(root)
    for name in sorted(actual.keys() | expected.keys()):
        if actual.get(name) != expected.get(name):
            raise ValueError(f'Model asset index changed during selection: data/cache/{name}')


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


def data_files(root, additional_files=None):
    data = root / 'data'
    if additional_files is None:
        additional_files = {
            Path('textures') / f'{fdid}.blp' for fdid in ui_texture_fdids(root)
        }
    else:
        additional_files = set(additional_files)
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
                relative = (Path(directory) / filename).relative_to(data)
                if not filename.endswith(('.missing', '.lock')) and relative not in additional_files:
                    yield relative
    for pattern in DATA_FILE_GLOBS:
        for path in data.glob(pattern):
            if path.is_file():
                yield path.relative_to(data)
    yield from sorted(additional_files)


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


def read_db2_rows(root, table):
    path = root / 'data/db2/active' / f'{table}.csv'
    with path.open(encoding='utf-8-sig', newline='') as source:
        reader = csv.DictReader(source)
        if reader.fieldnames is None:
            raise ValueError(f'Empty DB2 table: {path}')
        return list(reader)


def is_blp_file(path):
    try:
        with path.open('rb') as texture:
            return texture.read(4) in (b'BLP1', b'BLP2')
    except OSError:
        return False


def ui_texture_fdids(root):
    maps = {int(row['ID']) for row in read_db2_rows(root, 'UiMap')}
    art_links = {
        int(row['UiMapArtID']) for row in read_db2_rows(root, 'UiMapXMapArt')
        if int(row['PhaseID']) == 0 and int(row['UiMapID']) in maps
    }
    arts = {int(row['ID']): row for row in read_db2_rows(root, 'UiMapArt')}
    layer_styles = {
        int(row['UiMapArtStyleID']) for row in read_db2_rows(root, 'UiMapArtStyleLayer')
        if int(row['LayerIndex']) == 0
    }
    art_ids = {
        art_id for art_id in art_links
        if art_id in arts and int(arts[art_id]['UiMapArtStyleID']) in layer_styles
    }
    fdids = {
        int(arts[art_id]['HighlightFileDataID']) for art_id in art_ids
        if int(arts[art_id]['HighlightFileDataID']) > 0
    }
    fdids.update(
        int(row['FileDataID']) for row in read_db2_rows(root, 'UiMapArtTile')
        if int(row['UiMapArtID']) in art_ids and int(row['LayerIndex']) == 0
        and int(row['FileDataID']) > 0
    )
    overlay_ids = {
        int(row['ID']) for row in read_db2_rows(root, 'WorldMapOverlay')
        if int(row['UiMapArtID']) in art_ids
    }
    fdids.update(
        int(row['FileDataID']) for row in read_db2_rows(root, 'WorldMapOverlayTile')
        if int(row['WorldMapOverlayID']) in overlay_ids and int(row['LayerIndex']) == 0
        and int(row['FileDataID']) > 0
    )
    atlas_ids = {
        int(row['UiTextureAtlasID']) for row in read_db2_rows(root, 'UiTextureAtlasMember')
    }
    fdids.update(
        int(row['FileDataID']) for row in read_db2_rows(root, 'UiTextureAtlas')
        if int(row['ID']) in atlas_ids and int(row['FileDataID']) > 0
    )
    return fdids


def extract_ui_texture_ids(root, texture_dir, missing):
    texture_dir.mkdir(parents=True, exist_ok=True)
    missing = sorted(missing)
    if not missing:
        return 0
    extractor = Path(os.environ.get('CASC_LOCAL', root.parent / 'asset-resolver/target/debug/casc-local'))
    if not extractor.is_file():
        raise FileNotFoundError(
            f'Cannot assemble UI texture closure: {len(missing)} DB2-referenced BLPs are absent; '
            f'local CASC extractor not found at {extractor}'
        )
    environment = dict(os.environ, WOW_PRODUCT='wow')
    result = subprocess.run(
        [str(extractor), *map(str, missing), '-o', str(texture_dir)],
        cwd=root,
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    for fdid in missing:
        dat = texture_dir / f'{fdid}.dat'
        blp = texture_dir / f'{fdid}.blp'
        if not blp.is_file() and dat.is_file():
            with dat.open('rb') as source:
                is_blp = source.read(4) in (b'BLP1', b'BLP2')
            if is_blp:
                dat.rename(blp)
    unresolved = [fdid for fdid in missing if not is_blp_file(texture_dir / f'{fdid}.blp')]
    if result.returncode or unresolved:
        raise ValueError(
            f'Local CASC UI texture extraction failed (exit {result.returncode}); '
            f'{len(unresolved)} required FDIDs remain absent: {unresolved[:32]}. '
            f'Extractor output: {result.stdout[-2000:]}'
        )
    added_bytes = sum((texture_dir / f'{fdid}.blp').stat().st_size for fdid in missing)
    print(f'Added {len(missing)} DB2-referenced UI textures from local CASC ({added_bytes} bytes)')
    return added_bytes


def extract_missing_ui_textures(root, bundle, required_fdids):
    texture_dir = bundle / 'data/textures'
    missing = []
    for fdid in sorted(required_fdids):
        path = texture_dir / f'{fdid}.blp'
        if is_blp_file(path):
            continue
        path.unlink(missing_ok=True)
        missing.append(fdid)
    extract_ui_texture_ids(root, texture_dir, missing)


def copy_data(root, bundle):
    for name in DATA_DIRS:
        (bundle / 'data' / name).mkdir(parents=True, exist_ok=True)
    required_fdids = ui_texture_fdids(root)
    required_ui_textures = {Path('textures') / f'{fdid}.blp' for fdid in required_fdids}
    # Catalogs can change while the client runs; only immutable payloads share storage.
    files = iter(data_files(root, required_ui_textures))
    while batch := list(itertools.islice(files, 512)):
        copied, linked = [], []
        for relative in batch:
            if not (root / 'data' / relative).is_file():
                if relative in required_ui_textures:
                    continue
                raise FileNotFoundError(f'Missing selected player data file: {relative}')
            selected = copied if is_mutable_data(relative) else linked
            selected.append(relative)
        copy_selected(root / 'data', bundle / 'data', copied)
        copy_selected(root / 'data', bundle / 'data', linked, link=True)
    extract_missing_ui_textures(root, bundle, required_fdids)


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
        if path.parts[1] == 'tests' or path.name in ('Cargo.toml', 'Cargo.lock', '.gitignore', 'test-assets.txt', 'game_engine.gdextension'):
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
    # Players get a maximized window (Retail's default); the dev project keeps its fixed
    # 1280x720 window because capture fixtures depend on it.
    (project / 'override.cfg').write_text('[display]\nwindow/size/mode=2\n')
    cache = project / '.godot'
    cache.mkdir(exist_ok=True)
    (cache / 'extension_list.cfg').write_text('res://game_engine.gdextension\n')


def copy_project(root, bundle, platform):
    bundle.mkdir(parents=True, exist_ok=True)
    copy_selected(root, bundle, project_files(root))
    write_extension_config(bundle, platform)


def source_map(root, generated, windows_artifacts):
    """Map shipped paths to source files and stage DB2-required UI textures."""
    required_fdids = ui_texture_fdids(root)
    required_ui_textures = {Path('textures') / f'{fdid}.blp' for fdid in required_fdids}
    staged_textures = generated / 'data/textures'
    shutil.rmtree(staged_textures, ignore_errors=True)
    missing = [
        fdid for fdid in sorted(required_fdids)
        if not is_blp_file(root / 'data/textures' / f'{fdid}.blp')
        and not is_blp_file(staged_textures / f'{fdid}.blp')
    ]
    extract_ui_texture_ids(root, staged_textures, missing)
    files = {path.as_posix(): root / path for path in project_files(root)}
    data_sources = {}
    for path in data_files(root, required_ui_textures):
        source = root / 'data' / path
        if path in required_ui_textures and not is_blp_file(source):
            source = generated / 'data' / path
        data_sources[(Path('data') / path).as_posix()] = source
    files.update(data_sources)
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
