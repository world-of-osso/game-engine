#!/usr/bin/env python3
"""One private installed-build DB2 refresh: export, caches, census, publication manifest.

Run inside agent-run. This command does not apply data or import world.db.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys

import apply_db2_install
import db2_census
import refresh_db2_install


def run(argv, log):
    with log.open('w') as stream:
        result = subprocess.run([str(arg) for arg in argv], stdout=stream, stderr=subprocess.STDOUT)
    if result.returncode:
        raise RuntimeError(f'{argv[0]} exited {result.returncode}; inspect {log}')


def link_file(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.is_symlink():
        destination.unlink()
    elif destination.exists():
        raise ValueError(f'refuse overwriting private non-link {destination}')
    destination.symlink_to(source.resolve(), target_is_directory=source.is_dir())


def prepare_cache_root(args, receipt):
    root = args.output / 'client-data'
    snapshot = args.output / 'db2' / receipt['identity']['build']
    root.mkdir(parents=True, exist_ok=True)
    (root / 'cache').mkdir(exist_ok=True)
    link_file(snapshot, root / 'db2/active')
    # Root projections remain explicit references to the same authenticated snapshot.
    for source in (args.engine / 'data').glob('*.csv'):
        if source.stem in receipt['tables']:
            link_file(snapshot / source.name, root / source.name)
    # Published Forever inputs retain their original product/build identity.
    for name in ('1.60.1.70205', '1.60.1.69913'):
        source = args.engine / 'data/db2' / name
        if source.exists():
            destination = root / 'db2' / name
            destination.mkdir(parents=True, exist_ok=True)
            for item in source.iterdir():
                if item.name != 'cache':
                    link_file(item, destination / item.name)
            (destination / 'cache').mkdir(exist_ok=True)
    for name in ('community-listfile.csv', 'equipment_transforms.ron'):
        source = args.engine / 'data' / name
        if source.exists():
            link_file(source, root / name)
    return root


def regenerate_caches(args, receipt, root):
    run([sys.executable, args.engine / 'scripts/depot-build.py', '--root', args.engine,
         '--fixture', 'db2_cache_import'], args.output / 'cache-build.log')
    run([args.engine / 'target/debug/examples/db2_cache_import', root], args.output / 'cache-import.log')
    raw = args.output / 'raw' / receipt['identity']['build_key']
    definitions = args.definitions / 'definitions'
    items = args.output / 'db2' / receipt['identity']['build']
    run([sys.executable, args.engine / 'scripts/adventure_guide_import.py', '--db2', raw,
         '--definitions', definitions, '--output', root / 'cache/adventure_guide.sqlite',
         '--items', items, '--world', args.world], args.output / 'adventure-cache.log')
    run([sys.executable, args.engine / 'scripts/trading_post_import.py', '--db2', raw,
         '--definitions', definitions, '--output-sql', args.output / 'trading-post.sql',
         '--report', args.output / 'trading-post.json'], args.output / 'trading-post.log')
    run([sys.executable, args.engine / 'scripts/export_renown_catalog.py', '--db2', raw,
         '--definitions', definitions, '--output', args.output / 'renown-catalog.json'], args.output / 'renown.log')


def prepare_housing_cache(args, receipt, root):
    source = args.engine / 'data/cache/housing.sqlite'
    if not source.exists():
        return
    old = sqlite3.connect(f'file:{source}?mode=ro', uri=True)
    tables = [row[0] for row in old.execute('SELECT DISTINCT table_name FROM housing_catalog_source')]
    schema = '\n'.join(row[0] + ';' for row in old.execute("SELECT sql FROM sqlite_master WHERE type='table' AND name LIKE 'housing_catalog_%'"))
    old.close()
    path = root / 'cache/housing.sqlite'
    if path.exists():
        path.unlink()
    connection = sqlite3.connect(path)
    connection.executescript(schema)
    identity = receipt['identity']
    report_sha = hashlib.sha256(json.dumps(receipt, sort_keys=True).encode()).hexdigest()
    connection.execute('INSERT INTO housing_catalog_build VALUES(?,?,?,?)',
                       (identity['build_key'], identity['build'], report_sha, report_sha))
    for table in tables:
        info = receipt['tables'].get(table)
        if info is None:
            gap = receipt['unavailable'][table]
            connection.execute('INSERT INTO housing_catalog_source VALUES(?,?,?,?,?,?,?,?,?,?)',
                               (identity['build_key'], table, gap['fdid'], 'root_unavailable', '', 0, 0, '', '', json.dumps(gap)))
            continue
        connection.execute('INSERT INTO housing_catalog_source VALUES(?,?,?,?,?,?,?,?,?,?)',
                           (identity['build_key'], table, info['fdid'], 'exported', info['layout'], info['rows'],
                            info['unreadable_records'], info['csv_sha256'], info['db2_sha256'], json.dumps(info)))
        with (args.output / 'db2' / identity['build'] / f'{table}.csv').open(newline='') as stream:
            connection.executemany('INSERT INTO housing_catalog_record VALUES(?,?,?,?)',
                                   ((identity['build_key'], table, int(row[info['id_column']]), json.dumps(row))
                                    for row in csv.DictReader(stream)))
    connection.commit()
    connection.close()


def prepare_manifest(args, receipt, root):
    build = receipt['identity']['build']
    snapshot = args.output / 'db2' / build
    entries = []
    destinations = set()
    def add(source, destination):
        if str(destination) in destinations:
            raise ValueError(f'duplicate publication destination {destination}')
        destinations.add(str(destination))
        entries.append({'source': str(source.resolve()), 'destination': str(destination),
                        'before': apply_db2_install.fingerprint(destination),
                        'after': apply_db2_install.fingerprint(source)})
    for source in sorted(snapshot.iterdir()):
        if source.suffix not in ('.csv', '.json'):
            continue
        for repository in (args.canonical_engine, args.canonical_server):
            add(source, repository / 'data/db2' / build / source.name)
        if source.suffix == '.csv' and (args.canonical_engine / 'data' / source.name).exists():
            add(source, args.canonical_engine / 'data' / source.name)
    for source in sorted((root / 'cache').iterdir()):
        if source.is_file() and not source.is_symlink() and source.suffix in ('.sqlite', '.bin'):
            add(source, args.canonical_engine / 'data/cache' / source.name)
    links = []
    for repository in (args.canonical_engine, args.canonical_server):
        path = repository / 'data/db2/active'
        links.append({'path': str(path), 'before': str(path.readlink()) if path.is_symlink() else None, 'after': build})
    manifest = {'identity': receipt['identity'], 'schema_commit': receipt['schema_commit'],
                'files': entries, 'links': links,
                'excluded': ['world.db', 'auth tokens', 'historical product assets', 'unavailable local tables']}
    path = args.output / 'apply-manifest.json'
    path.write_text(json.dumps(manifest, indent=2) + '\n')
    print(f'Lead-only apply: python3 {args.canonical_engine / "scripts/apply_db2_install.py"} --manifest {path} --backup {args.output / "canonical-backup"}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('engine', 'server', 'definitions', 'output', 'resolver', 'resolver-cache',
                 'baseline', 'world', 'canonical-engine', 'canonical-server'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--schema-commit', default=refresh_db2_install.DBD_COMMIT)
    parser.add_argument('--install', type=Path, default=Path('/syncthing/World of Warcraft'))
    args = parser.parse_args()
    args.output = args.output.resolve()
    receipt = refresh_db2_install.export_tables(args)
    root = prepare_cache_root(args, receipt)
    regenerate_caches(args, receipt, root)
    prepare_housing_cache(args, receipt, root)
    census_output = args.output / 'census'
    census_output.mkdir(exist_ok=True)
    db2_census.census(argparse.Namespace(old=args.baseline, new=args.output / 'db2' / receipt['identity']['build'],
                                       output=census_output, engine=args.engine, server=args.server, world=args.world))
    prepare_manifest(args, receipt, root)


if __name__ == '__main__':
    main()
