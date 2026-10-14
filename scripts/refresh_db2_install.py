#!/usr/bin/env python3
"""Re-export consumed Retail DB2s from the authenticated active local install.

No canonical writes. Definitions must be a clean WoWDBDefs checkout at DBD_COMMIT.
The server supplies its existing sparse WDC5 decoder; world.db stays untouched.
"""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import struct
import sys
import sqlite3

import db2_dbd
import export_db2_csv as wdc
from stage_mount_model_assets import authenticate_install

DBD_COMMIT = 'd6f50d4f351193e84226aa62a36faa0f58528afa'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def consumed_tables(engine, server, definitions):
    names = {p.stem for p in definitions.glob('*.dbd')}
    consumed = set()
    for root in (engine / 'data', server / 'data/db2'):
        for path in root.glob('*.csv'):
            if path.stem in names:
                consumed.add(path.stem)
        if root.name == 'db2':
            for path in root.glob('*/*.csv'):
                if path.stem in names:
                    consumed.add(path.stem)
    # Both existing snapshots and code consumers, including SQL/JSON-only catalogs.
    for root in (engine / 'scripts', engine / 'godot', engine / 'tools', server / 'scripts', server / 'src', server / 'crates'):
        for path in root.rglob('*'):
            if path.suffix not in ('.py', '.rs', '.sql') or 'fixtures' in path.parts or 'target' in path.parts:
                continue
            tokens = set(re.findall(r'''["']([A-Z][A-Za-z0-9]+)(?:\.csv)?["']''', path.read_text()))
            consumed.update(names & tokens)
    catalog = engine / 'data/cache/housing.sqlite'
    if catalog.exists():
        with sqlite3.connect(f'file:{catalog}?mode=ro', uri=True) as connection:
            consumed.update(row[0] for row in connection.execute(
                'SELECT DISTINCT table_name FROM housing_catalog_source'))
    return sorted(consumed)


def table_fdids(listfile, tables):
    wanted = {t.lower(): t for t in tables}
    result = {}
    with listfile.open() as stream:
        for line in stream:
            fdid, name = line.rstrip().split(';', 1)
            if name.lower().startswith('dbfilesclient/') and name.lower().endswith('.db2'):
                table = wanted.get(Path(name).stem.lower())
                if table:
                    result[table] = int(fdid)
    missing = set(tables) - result.keys()
    if missing:
        raise ValueError(f'listfile missing DB2 IDs: {sorted(missing)}')
    return result


def validate_definitions(checkout, commit):
    revision = subprocess.check_output(['git', '-C', str(checkout), 'rev-parse', 'HEAD'], text=True).strip()
    dirty = subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain', '--untracked-files=no'], text=True)
    if revision != commit or dirty:
        raise ValueError(f'WoWDBDefs must be clean at {commit}; got {revision}')
    return checkout / 'definitions'


def export_tables(args):
    definitions = validate_definitions(args.definitions, args.schema_commit)
    identity, roots = authenticate_install(args.install)
    output = args.output.resolve()
    for canonical in (args.engine / 'data', args.server / 'data'):
        if output.is_relative_to(canonical.resolve()):
            raise ValueError(f'output must be private, not {canonical}')
    directory = output / 'db2' / identity['build']
    raw = output / 'raw' / identity['build_key']
    directory.mkdir(parents=True, exist_ok=True)
    raw.mkdir(parents=True, exist_ok=True)
    tables = table_fdids(args.engine / 'data/community-listfile.csv', consumed_tables(args.engine, args.server, definitions))
    sys.path.insert(0, str(args.server / 'scripts'))
    import db2_casc
    receipt = {'identity': identity, 'schema_commit': args.schema_commit, 'tables': {}, 'unavailable': {}}
    pending = []
    for fdid in tables.values():
        expected = roots.get(fdid)
        source = raw / f'{fdid}.db2'
        if expected is not None and (not source.exists() or hashlib.md5(source.read_bytes()).digest() != expected):
            if source.exists():
                source.unlink()
            pending.append(str(fdid))
    result = subprocess.run([str(args.resolver), *pending, '-o', str(raw)], cwd=args.engine,
                            env={**os.environ, 'WOW_INSTALL_PATH': str(args.install), 'WOW_PRODUCT': 'wow',
                                 'ASSET_RESOLVER_CACHE_DIR': str(args.resolver_cache)}, capture_output=True, text=True) if pending else None
    if result is not None:
        with (output / 'extraction.log').open('a') as log:
            log.write(result.stdout + result.stderr)
    for table, fdid in tables.items():
        definition = definitions / f'{table}.dbd'
        expected = roots.get(fdid)
        if expected is None:
            receipt['unavailable'][table] = {'fdid': fdid, 'reason': 'absent from active enUS local root'}
            continue
        source = raw / f'{fdid}.db2'
        if not source.exists():
            receipt['unavailable'][table] = {'fdid': fdid, 'reason': 'local extraction failed; see extraction.log'}
            continue
        data = source.read_bytes()
        authenticated = hashlib.md5(data).digest() == expected
        try:
            details = db2_dbd.export(data, definition, directory / f'{table}.csv', wdc, db2_casc)
            receipt['tables'][table] = {**details, 'fdid': fdid, 'db2_sha256': sha(source),
                                       'schema_sha256': sha(definition), 'csv_sha256': sha(directory / f'{table}.csv'),
                                       'content_authenticated': authenticated}
            print(f'{table}: {details}', flush=True)
        except (ValueError, IndexError, KeyError, struct.error) as error:
            receipt['unavailable'][table] = {'fdid': fdid, 'reason': str(error)}
            print(f'{table}: {error}', file=sys.stderr, flush=True)
    from closure_casc import installed_products
    active = [row for row in installed_products(args.install) if row['Product'] == 'wow']
    if len(active) != 1 or active[0]['Build Key'] != identity['build_key'] or active[0]['Version'] != identity['build']:
        raise ValueError('active install changed during DB2 export; private outputs have no valid receipt')
    (directory / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    (raw / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    (output / 'active-build.json').write_text(json.dumps(identity, indent=2) + '\n')
    print(f'exported {len(receipt["tables"])}; unavailable {len(receipt["unavailable"])}')
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('engine', 'server', 'definitions', 'output', 'resolver', 'resolver-cache'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--schema-commit', default=DBD_COMMIT, help='exact clean WoWDBDefs commit; recorded in receipts')
    parser.add_argument('--install', type=Path, default=Path('/syncthing/World of Warcraft'))
    export_tables(parser.parse_args())


if __name__ == '__main__':
    main()
