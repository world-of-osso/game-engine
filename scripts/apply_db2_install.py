#!/usr/bin/env python3
"""Lead-only DB2 publication from a reviewed manifest, with backup and fingerprint diff.

Stop canonical clients/importers before applying. This command never imports world.db.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil


def fingerprint(path):
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def inspect_manifest(document):
    files, links = [], []
    for entry in document['files']:
        actual_before = fingerprint(Path(entry['destination']))
        actual_after = fingerprint(Path(entry['source']))
        files.append({**entry, 'actual_before': actual_before, 'actual_after': actual_after,
                      'matches': actual_before == entry['before'] and actual_after == entry['after']})
    for entry in document['links']:
        path = Path(entry['path'])
        actual = str(path.readlink()) if path.is_symlink() else None
        matches = actual == entry['before'] and (path.is_symlink() or not path.exists())
        links.append({**entry, 'actual_before': actual, 'matches': matches})
    return {'files': files, 'links': links}


def copy_with_backup(entry, backup, index):
    source, destination = Path(entry['source']), Path(entry['destination'])
    if destination.exists():
        shutil.copy2(destination, backup / 'files' / f'{index:06d}-{destination.name}')
    destination.parent.mkdir(parents=True, exist_ok=True)
    staging = destination.with_name(destination.name + f'.db2-stage-{os.getpid()}')
    if entry.get('hardlink', False):
        os.link(source, staging)
    else:
        shutil.copy2(source, staging)
    if fingerprint(staging) != entry['after']:
        raise ValueError(f'staged fingerprint differs: {staging}')
    staging.replace(destination)


def publish_links(links):
    for entry in links:
        path = Path(entry['path'])
        path.parent.mkdir(parents=True, exist_ok=True)
        staging = path.with_name(path.name + f'.db2-stage-{os.getpid()}')
        staging.symlink_to(entry['after'], target_is_directory=True)
        staging.replace(path)


def apply(manifest, backup):
    document = json.loads(manifest.read_text())
    diff = inspect_manifest(document)
    backup.mkdir(parents=True, exist_ok=False)
    (backup / 'fingerprint-diff.json').write_text(json.dumps(diff, indent=2) + '\n')
    if not all(row['matches'] for group in diff.values() for row in group):
        raise ValueError(f'fingerprint drift; no writes applied; inspect {backup / "fingerprint-diff.json"}')
    (backup / 'manifest.json').write_text(json.dumps(document, indent=2) + '\n')
    (backup / 'files').mkdir()
    for index, entry in enumerate(document['files']):
        copy_with_backup(entry, backup, index)
    publish_links(document['links'])
    after = {entry['destination']: fingerprint(Path(entry['destination'])) for entry in document['files']}
    (backup / 'published-fingerprints.json').write_text(json.dumps(after, indent=2) + '\n')
    if any(after[entry['destination']] != entry['after'] for entry in document['files']):
        raise ValueError(f'publication fingerprint mismatch; backup retained at {backup}')
    print(f'published {len(after)} files; backup and fingerprint diff: {backup}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest', type=Path, required=True)
    parser.add_argument('--backup', type=Path, required=True)
    parser.add_argument('--install', type=Path, default=Path('/syncthing/World of Warcraft'))
    parser.add_argument('--asset-data', type=Path, required=True)
    parser.add_argument('--asset-resolver', type=Path, required=True)
    parser.add_argument('--asset-resolver-cache', type=Path, required=True)
    args = parser.parse_args()
    from stage_mount_model_assets import authenticate_install
    from closure_casc import installed_products
    from migrate_model_asset_index import prepare_installed_migration
    identity, roots = authenticate_install(args.install)
    document = json.loads(args.manifest.read_text())
    if any(document['identity'][field] != identity[field] for field in ('build', 'build_key', 'build_config_sha256')):
        raise ValueError('Installed build differs from reviewed DB2 manifest; refresh before publication')
    staged = args.backup.with_name(args.backup.name + '-asset-stage')
    asset_manifest, report = prepare_installed_migration(args.asset_data, staged, identity, roots,
                                                       args.asset_resolver, args.asset_resolver_cache, args.install)
    document['files'].extend(asset_manifest['files'])
    active = [row for row in installed_products(args.install) if row['Product'] == 'wow']
    if len(active) != 1 or active[0]['Build Key'] != identity['build_key']:
        raise ValueError('Installed build changed during asset migration; no canonical writes applied')
    combined = staged / 'combined-apply.json'
    combined.write_text(json.dumps(document, indent=2) + '\n')
    print(f'Asset migration: {report["unchanged_bytes"]} identical, {report["changed_bytes"]} changed, '
          f'{len(report["unavailable"])} absent current-root IDs; report {staged / "migration-report.json"}')
    apply(combined, args.backup)


if __name__ == '__main__':
    main()
