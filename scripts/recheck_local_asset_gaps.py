#!/usr/bin/env python3
"""Retry a recorded gap inventory against active local products, with no-clobber receipts."""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

import closure_casc
from closure_extract import publish


def resolve_product(install, product, assets, indices, archives):
    build = product['Build Key']
    config_path = install / 'Data/config' / build[:2] / build[2:4] / build
    config_bytes = config_path.read_bytes()
    if hashlib.md5(config_bytes).hexdigest() != build:
        raise ValueError('build config content hash mismatch')
    config = dict(line.split(' = ', 1) for line in config_bytes.decode().splitlines() if ' = ' in line)
    encoding_ck, encoding_ek = config['encoding'].split()
    encoding = closure_casc.read_metadata(install, indices, encoding_ek, encoding_ck)
    root_ck = config['root']
    root_record = closure_casc.encoding_keys(encoding, {bytes.fromhex(root_ck)})[bytes.fromhex(root_ck)]
    root_ek = next(key for key in root_record[1] if key[:9] in indices)
    root = closure_casc.read_metadata(install, indices, root_ek.hex(), root_ck)
    wanted = {asset['fdid'] for asset in assets if asset['fdid'] > 0}
    roots = closure_casc.root_keys(root, wanted)
    encodings = closure_casc.encoding_keys(encoding, set(roots.values()))
    audit = closure_casc.resolve_missing(wanted, roots, encodings, indices, archives)
    identities = {}
    for row in audit['files']:
        fdid = row['fdid']
        if row['status'] == 'local_index_present':
            identities[fdid] = {'product': product['Product'], 'version': product['Version'],
                                'build_config': build, 'fdid': fdid,
                                'content_key': roots[fdid].hex(),
                                'encoding_key': row['encoding_key'], 'size': row['content_bytes']}
    return audit, identities


def run_batch(assets, executable, stage, log, environment):
    stage.mkdir(parents=True, exist_ok=False)
    command = [str(executable), *map(str, sorted({a['fdid'] for a in assets})), '-o', str(stage)]
    with log.open('w') as stream:
        result = subprocess.run(command, env=dict(os.environ, **environment), stdout=stream,
                                stderr=stream, check=False)
    text = log.read_text()
    failures = {int(fdid): reason for fdid, reason in re.findall(r'Failed FDID (\d+): (.*)', text)}
    encrypted = {int(fdid) for fdid in re.findall(r'FDID (\d+) missing TACT keys', text)}
    files = {int(path.stem): path for path in stage.iterdir() if path.is_file() and path.stem.isdecimal()}
    return result.returncode, failures, encrypted, files


def publish_result(asset, identity, path, data):
    try:
        result = publish(path, data, asset['locations'], dict(identity, type=asset['type']),
                         data / 'provenance/closure-extract.jsonl', {})
    except (ValueError, OSError) as error:
        return {'status': 'validation_failed', 'reason': str(error)}
    if result['conflicts']:
        return dict(result, status='conflict')
    return dict(result, status='readable')


def extract_resolved(assets, identities, executable, data, output, environment, batch_size=128):
    output.mkdir(parents=True, exist_ok=True)
    pending = [asset for asset in assets if asset['fdid'] in identities]
    rows = []
    for start in range(0, len(pending), batch_size):
        batch = pending[start:start + batch_size]
        stage = output / f'batch-{start // batch_size:05d}'
        log = stage.with_suffix('.log')
        exit_code, failures, encrypted, files = run_batch(batch, executable, stage, log, environment)
        for asset in batch:
            fdid = asset['fdid']
            row = dict(asset, extractor_exit=exit_code, log=str(log))
            if fdid in encrypted:
                row.update(status='encrypted', reason='unknown TACT keys; output not published')
            elif fdid in failures or fdid not in files:
                row.update(status='extract_failed', reason=failures.get(fdid, 'no extracted file'))
            else:
                row.update(publish_result(asset, identities[fdid], files[fdid], data))
            rows.append(row)
        (output / 'partial-results.json').write_text(json.dumps(rows, indent=2) + '\n')
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inventory', type=Path, required=True)
    parser.add_argument('--install', type=Path, required=True)
    parser.add_argument('--data', type=Path, required=True)
    parser.add_argument('--executable', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    inventory = json.loads(args.inventory.read_text())
    indices, archives, sources = closure_casc.load_indices(args.install)
    products = {p['Product']: p for p in closure_casc.installed_products(args.install)}
    args.output.mkdir(parents=True, exist_ok=True)
    for name, assets in inventory.items():
        output = args.output / name
        output.mkdir(exist_ok=True)
        audit, identities = resolve_product(args.install, products[name], assets, indices, archives)
        (output / 'resolution.json').write_text(json.dumps(audit, indent=2) + '\n')
        environment = {'WOW_PRODUCT': name, 'WOW_INSTALL_PATH': str(args.install),
                       'GAME_ENGINE_ASSET_MODE': 'local-casc',
                       'ASSET_RESOLVER_DATA_DIR': str(args.data),
                       'ASSET_RESOLVER_SHARED_DATA_DIR': str(args.data),
                       'ASSET_RESOLVER_CACHE_DIR': str(args.output / 'resolver-cache')}
        rows = extract_resolved(assets, identities, args.executable, args.data, output, environment)
        resolved = {row['fdid'] for row in rows}
        for asset in assets:
            if asset['fdid'] not in resolved:
                status = next((r['status'] for r in audit['files'] if r['fdid'] == asset['fdid']), 'invalid_fdid')
                rows.append(dict(asset, status=status))
        (output / 'results.json').write_text(json.dumps(rows, indent=2) + '\n')
        print(json.dumps({'product': name, 'counts': dict(Counter(r['status'] for r in rows)),
                          'created': sum(r.get('created', 0) for r in rows), 'idx_sources': sources}), flush=True)


if __name__ == '__main__':
    main()
