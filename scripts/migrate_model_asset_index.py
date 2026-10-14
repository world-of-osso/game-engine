#!/usr/bin/env python3
"""Prepare current-install model receipts for guarded lead-only publication.

Unchanged authenticated bytes are hard-linked into the current namespace during
apply, avoiding a second copy of the model tree. Historic files are not deleted.
Absent current-root FDIDs are reported and removed from the live index.
"""
import hashlib
import json
from pathlib import Path
import subprocess

from apply_db2_install import fingerprint
from import_model_asset_chains import create_asset_receipt
from import_forever_skyborne import validate_magic


def index_paths(data):
    cache = data / 'cache'
    return [cache / 'model-asset-index.json', *sorted(cache.glob('model-asset-index-*.json'))]


def prepare_migration(data, output, identity, roots, extracted):
    output.mkdir(parents=True, exist_ok=False)
    files, destinations, keys = [], set(), set()
    report = {'identity': identity, 'unchanged_bytes': 0, 'changed_bytes': 0, 'unavailable': [], 'unreadable': [], 'historical_chains': []}

    def add(source, destination, digest, hardlink=False):
        if destination in destinations:
            return
        destinations.add(destination)
        files.append({'source': str(source.resolve()), 'destination': str(destination),
                      'before': fingerprint(destination), 'after': digest, 'hardlink': hardlink})

    def migrate(row, alias=False):
        if row['product'] != 'wow':
            return row
        fdid, kind = row['fdid'], row['kind']
        content = roots.get(fdid)
        if content is None:
            report['unavailable'].append({'fdid': fdid, 'kind': kind, 'path': row['path']})
            return None
        key = content.hex()
        relative = Path(row['path']).relative_to(Path('products') / row['product'] / row['build_key'])
        source = data / row['path']
        if row.get('content_key') == key:
            receipt = {**row, **identity, 'path': str(Path('products/wow') / identity['build_key'] / relative)}
            if not alias:
                report['unchanged_bytes'] += 1
        else:
            source = extracted[fdid]
            raw = source.read_bytes()
            if hashlib.md5(raw).hexdigest() != key:
                # Encrypted content without a key extracts as zeros; never publish it or keep old-build bytes.
                report['unreadable'].append({'fdid': fdid, 'kind': kind, 'path': row['path']})
                return None
            validate_magic(raw, kind, key)
            # Keep alias metadata (owner_model_fdid, reference); the receipt replaces identity and bytes.
            receipt = {**row, **create_asset_receipt(identity, fdid, kind, str(relative), raw, key)}
            if not alias:
                report['changed_bytes'] += 1
        add(source, data / receipt['path'], receipt['sha256'], hardlink=True)
        return receipt

    for path in index_paths(data):
        document = json.loads(path.read_text())
        if document['version'] != 1:
            raise ValueError(f'Unsupported model asset index: {path}')
        assets = []
        for row in document['assets']:
            key = row['product'], row['fdid'], row['kind']
            if key in keys:
                raise ValueError(f'Duplicate model asset receipt: {key}')
            keys.add(key)
            current = migrate(row)
            if current is not None:
                assets.append(current)
        aliases = [current for row in document.get('aliases', []) if (current := migrate(row, alias=True)) is not None]
        chains = []
        for chain in document.get('chains', []):
            if chain['product'] == 'wow' and chain['build_key'] != identity['build_key']:
                report['historical_chains'].append(chain)
            else:
                chains.append(chain)
        result = {**document, 'assets': assets, 'aliases': aliases, 'chains': chains}
        source = output / path.name
        source.write_text(json.dumps(result, indent=2) + '\n')
        add(source, path, fingerprint(source))
    report['unavailable'] = sorted({(row['fdid'], row['kind']): row for row in report['unavailable']}.values(), key=lambda row: (row['fdid'], row['kind']))
    (output / 'migration-report.json').write_text(json.dumps(report, indent=2) + '\n')
    return {'identity': identity, 'files': files, 'links': []}, report


def needs_extraction(row, roots):
    # Rows without a recorded content key (NPC-appearance skin aliases) cannot be proven current.
    return row['product'] == 'wow' and row['fdid'] in roots and row.get('content_key') != roots[row['fdid']].hex()


def prepare_installed_migration(data, output, identity, roots, resolver, resolver_cache, install):
    needed = set()
    for path in index_paths(data):
        document = json.loads(path.read_text())
        for row in [*document['assets'], *document.get('aliases', [])]:
            if needs_extraction(row, roots):
                needed.add(row['fdid'])
    raw = output.with_name(output.name + '-raw')
    raw.mkdir(parents=True, exist_ok=False)
    if needed:
        import os
        fdids = sorted(needed)
        with (raw / 'extraction.log').open('w') as log:
            for start in range(0, len(fdids), 4096):
                result = subprocess.run([str(resolver), *map(str, fdids[start:start + 4096]), '-o', str(raw)],
                                        env={**os.environ, 'WOW_INSTALL_PATH': str(install), 'WOW_PRODUCT': 'wow',
                                             'ASSET_RESOLVER_CACHE_DIR': str(resolver_cache)}, stdout=log, stderr=subprocess.STDOUT)
                if result.returncode:
                    raise ValueError(f'Current local asset extraction failed; inspect {raw / "extraction.log"}')
    extracted = {}
    for fdid in needed:
        matches = list(raw.glob(f'{fdid}.*'))
        if len(matches) != 1:
            raise ValueError(f'FDID {fdid}: expected one extracted local payload, got {matches}')
        extracted[fdid] = matches[0]
    return prepare_migration(data, output, identity, roots, extracted)
