#!/usr/bin/env python3
"""Lead-only DB2/assets publication with full pre-write backup and rollback.

Stop canonical clients/importers first. Never imports world.db.
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
        before, after = fingerprint(Path(entry['destination'])), fingerprint(Path(entry['source']))
        current = before == entry['after']
        files.append({**entry, 'actual_before': before, 'actual_after': after, 'already_current': current,
                      'matches': after is not None and after == entry['after'] and (before == entry['before'] or current)})
    for entry in document['links']:
        path = Path(entry['path'])
        before = str(path.readlink()) if path.is_symlink() else None
        current = before == entry['after']
        matches = (before == entry['before'] or current) and (path.is_symlink() or not path.exists())
        links.append({**entry, 'actual_before': before, 'already_current': current, 'matches': matches})
    return {'files': files, 'links': links}


def back_up_files(files, backup):
    records = {}
    (backup / 'files').mkdir()
    for index, entry in enumerate(files):
        destination = Path(entry['destination'])
        record = {'copy': None, 'symlink': str(destination.readlink()) if destination.is_symlink() else None}
        if destination.is_file():
            copy = backup / 'files' / f'{index:06d}-{destination.name}'
            shutil.copy2(destination, copy)
            if fingerprint(copy) != entry['actual_before']:
                raise ValueError(f'destination changed during backup: {destination}')
            record['copy'] = str(copy)
        records[str(destination)] = record
    (backup / 'recovery.json').write_text(json.dumps(records, indent=2) + '\n')
    return records


def stage_files(files, staged):
    for index, entry in enumerate(files):
        source, destination = Path(entry['source']), Path(entry['destination'])
        destination.parent.mkdir(parents=True, exist_ok=True)
        temporary = destination.with_name(destination.name + f'.db2-stage-{os.getpid()}-{index}')
        if temporary.exists() or temporary.is_symlink():
            raise FileExistsError(f'publication staging path exists: {temporary}')
        staged.append((temporary, destination))
        if entry.get('hardlink', False):
            os.link(source, temporary)
        else:
            shutil.copy2(source, temporary)
        if fingerprint(temporary) != entry['after']:
            raise ValueError(f'staged fingerprint differs: {temporary}')


def stage_links(links, staged):
    for index, entry in enumerate(links):
        destination = Path(entry['path'])
        destination.parent.mkdir(parents=True, exist_ok=True)
        temporary = destination.with_name(destination.name + f'.db2-link-stage-{os.getpid()}-{index}')
        if temporary.exists() or temporary.is_symlink():
            raise FileExistsError(f'publication staging path exists: {temporary}')
        staged.append((temporary, destination))
        temporary.symlink_to(entry['after'], target_is_directory=True)


def restore_published(published, records, links):
    prior_links = {entry['path']: entry['actual_before'] for entry in links}
    for destination in reversed(published):
        name = str(destination)
        if name in prior_links:
            previous = prior_links[name]
            destination.unlink()
            if previous is not None:
                destination.symlink_to(previous, target_is_directory=True)
            continue
        record = records[name]
        if record['symlink'] is not None:
            destination.unlink()
            destination.symlink_to(record['symlink'])
        elif record['copy'] is None:
            destination.unlink()
        else:
            temporary = destination.with_name(destination.name + f'.db2-restore-{os.getpid()}')
            shutil.copy2(record['copy'], temporary)
            temporary.replace(destination)


def apply(manifest, backup):
    document = json.loads(manifest.read_text())
    diff = inspect_manifest(document)
    valid = all(row['matches'] for group in diff.values() for row in group)
    if valid and all(row['already_current'] for group in diff.values() for row in group):
        print('already current; no publication or backup writes')
        return
    backup.mkdir(parents=True, exist_ok=False)
    (backup / 'fingerprint-diff.json').write_text(json.dumps(diff, indent=2) + '\n')
    if not valid:
        raise ValueError(f'fingerprint drift; no writes applied; inspect {backup / "fingerprint-diff.json"}')
    (backup / 'manifest.json').write_text(json.dumps(document, indent=2) + '\n')
    files = [entry for entry in diff['files'] if not entry['already_current']]
    links = [entry for entry in diff['links'] if not entry['already_current']]
    records = back_up_files(files, backup)
    staged, published = [], []
    try:
        stage_files(files, staged)
        stage_links(links, staged)
        # Asset bytes precede indexes; active DB2 links come last.
        indexes = lambda pair: pair[1].name.startswith('model-asset-index') and pair[1].suffix == '.json'
        ordered = [pair for pair in staged if str(pair[1]) in records]
        ordered.sort(key=indexes)
        ordered.extend(pair for pair in staged if str(pair[1]) not in records)
        for temporary, destination in ordered:
            temporary.replace(destination)
            published.append(destination)
        after = {entry['destination']: fingerprint(Path(entry['destination'])) for entry in files}
        if any(after[entry['destination']] != entry['after'] for entry in files):
            raise ValueError('publication fingerprint mismatch')
        (backup / 'published-fingerprints.json').write_text(json.dumps(after, indent=2) + '\n')
    except Exception as failure:
        try:
            restore_published(published, records, links)
        except Exception as rollback:
            raise ExceptionGroup(f'publication and rollback failed; recovery data: {backup}', [failure, rollback]) from failure
        raise
    finally:
        for temporary, _ in staged:
            temporary.unlink(missing_ok=True)
    print(f'published {len(files)} files; complete backup and fingerprint diff: {backup}')


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
    staged = args.backup.with_name(args.backup.name + f'-asset-stage-{os.getpid()}')
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
