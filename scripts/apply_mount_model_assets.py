#!/usr/bin/env python3
"""Validate a staged Mount Journal package; only --apply publishes canonical files."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import tempfile


def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def index_hashes(data):
    cache = data / 'cache'
    paths = [cache / 'model-asset-index.json', *sorted(cache.glob('model-asset-index-*.json'))]
    return {path.name: sha(path) for path in paths if path.exists()}


def validate(package):
    plan = json.loads((package / 'manifest.json').read_text())
    canonical = Path(plan['canonical_data'])
    if index_hashes(canonical) != plan['expected_index_inputs']:
        raise ValueError('canonical receipt inputs changed; rebase package')
    for artifact in plan['artifacts']:
        relative = Path(artifact['path'])
        if relative.is_absolute() or '..' in relative.parts:
            raise ValueError(f'unsafe artifact path: {relative}')
        source, target = package / relative, canonical / relative
        if sha(source) != artifact['source_sha256'] or source.stat().st_size != artifact['bytes']:
            raise ValueError(f'source hash/size changed: {source}')
        before = sha(target) if target.exists() else None
        if before != artifact['expected_before_sha256']:
            raise ValueError(f'canonical before hash changed: {target}: {before}')
        if relative.suffix == '.sqlite':
            with sqlite3.connect(source.resolve().as_uri() + '?mode=ro', uri=True) as db:
                if db.execute('PRAGMA integrity_check').fetchone() != ('ok',):
                    raise ValueError(f'invalid SQLite: {source}')
            if target.exists():
                with sqlite3.connect(source.resolve().as_uri() + '?mode=ro', uri=True) as new, sqlite3.connect(target.resolve().as_uri() + '?mode=ro', uri=True) as old:
                    tables = old.execute("SELECT name FROM sqlite_master WHERE type='table' AND name!='source_files'")
                    for (table,) in tables:
                        quoted = '"' + table.replace('"', '""') + '"'
                        if not set(old.execute('SELECT * FROM ' + quoted)) <= set(new.execute('SELECT * FROM ' + quoted)):
                            raise ValueError(f'canonical rows lost: {relative}: {table}')
    print(f'VALIDATED {len(plan["artifacts"])} artifacts; no canonical mutation')
    return plan


def publish(package):
    plan = validate(package)
    canonical = Path(plan['canonical_data'])
    backup = package.parent / 'canonical-before'
    for artifact in plan['artifacts']:
        source = package / artifact['path']
        target = canonical / artifact['path']
        target.parent.mkdir(parents=True, exist_ok=True)
        # Recheck at the mutation boundary, not only at package validation.
        before = sha(target) if target.exists() else None
        if before != artifact['expected_before_sha256']:
            raise ValueError(f'canonical before hash changed during apply: {target}')
        if target.exists():
            saved = backup / artifact['path']
            saved.parent.mkdir(parents=True, exist_ok=True)
            with saved.open('xb') as out, target.open('rb') as original:
                shutil.copyfileobj(original, out)
        with tempfile.NamedTemporaryFile(dir=target.parent, prefix='.mount-model-assets-', delete=False) as out:
            temporary = Path(out.name)
            with source.open('rb') as original:
                shutil.copyfileobj(original, out)
            out.flush()
            os.fsync(out.fileno())
        if sha(temporary) != artifact['source_sha256']:
            temporary.unlink()
            raise ValueError(f'copy hash differs: {source}')
        temporary.replace(target)
    print(f'PUBLISHED {len(plan["artifacts"])} artifacts')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package', type=Path, default=Path(__file__).parent / 'cache-package')
    parser.add_argument('--apply', action='store_true')
    args = parser.parse_args()
    if args.apply:
        publish(args.package)
    else:
        validate(args.package)


if __name__ == '__main__':
    main()
