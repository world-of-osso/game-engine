#!/usr/bin/env python3
"""Authenticate legacy bytes against local product roots before scoped publication.

Consumes a recorded no-install misses.json. Never extracts, downloads, substitutes
another product, overwrites asset bytes, or treats a filename as provenance.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

import closure_casc
from closure_extract import runtime_path
import import_model_asset_chains as chains
from import_forever_skyborne import validate_magic

EXPECTED_BUILDS = {'wow': '12.1.0.69933', 'wow_classic_beta': '1.60.1.70338'}


def authenticate_root(install, product, wanted, indices, output):
    if product['Version'] != EXPECTED_BUILDS[product['Product']]:
        raise ValueError(f"Unexpected actual build: {product['Product']} {product['Version']}")
    build = product['Build Key']
    config_path = install / 'Data/config' / build[:2] / build[2:4] / build
    config_bytes = config_path.read_bytes()
    if hashlib.md5(config_bytes).hexdigest() != build:
        raise ValueError('build config content hash mismatch')
    config = dict(line.split(' = ', 1) for line in config_bytes.decode().splitlines() if ' = ' in line)
    encoding_ck, encoding_ek = config['encoding'].split()
    encoding = closure_casc.read_metadata(install, indices, encoding_ek, encoding_ck)
    root_ck = config['root']
    record = closure_casc.encoding_keys(encoding, {bytes.fromhex(root_ck)})[bytes.fromhex(root_ck)]
    root_ek = next(key for key in record[1] if key[:9] in indices)
    root = closure_casc.read_metadata(install, indices, root_ek.hex(), root_ck)
    roots = closure_casc.root_keys(root, wanted)
    snapshot = output / (product['Product'] + '-root-keys.json')
    snapshot.write_text(json.dumps({'build_key': build, 'root_content_key': root_ck,
                                   'encoding_content_key': encoding_ck,
                                   'roots': {str(k): v.hex() for k, v in sorted(roots.items())}}, indent=2) + '\n')
    identity = {'product': product['Product'], 'build_key': build, 'build': product['Version'],
                'build_config_sha256': hashlib.sha256(config_bytes).hexdigest(),
                'resolution_sha256': hashlib.sha256(snapshot.read_bytes()).hexdigest()}
    return identity, roots


def publish_legacy(data, identity, asset, roots):
    if asset['product'] != identity['product']:
        raise ValueError('requested product differs from authenticated root')
    fdid, kind = asset['fdid'], asset['extension']
    row = dict(asset)
    content = roots.get(fdid)
    if content is None:
        return dict(row, status='root_absent')
    source = runtime_path(data, asset['legacy_candidate_path'])
    if not source.is_file():
        return dict(row, status='bytes_absent')
    raw = source.read_bytes()
    digest = hashlib.md5(raw).hexdigest()
    row.update(legacy_content_key=digest, expected_content_key=content.hex())
    if digest != content.hex():
        return dict(row, status='content_mismatch')
    validate_magic(raw, kind, content.hex())
    directory = 'textures' if kind == 'blp' else 'models'
    receipt = chains.create_asset_receipt(identity, fdid, kind, f'{directory}/{fdid}.{kind}', raw, content.hex())
    try:
        chains.write_asset_files(data, [(receipt, raw)])
    except ValueError as error:
        return dict(row, status='scoped_conflict', reason=str(error))
    chains.write_asset_index(data, identity, {'dependencies': {}, 'metadata': []}, [(receipt, raw)], [])
    return dict(row, status='verified', receipt=receipt)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--misses', type=Path, required=True)
    parser.add_argument('--install', type=Path, required=True)
    parser.add_argument('--data', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assets = [{key: row[key] for key in ('product', 'fdid', 'extension', 'legacy_candidate_path')}
              for row in json.loads(args.misses.read_text())['asset_misses']
              if row['kind'] == 'missing_verified_receipt']
    unknown = {row['product'] for row in assets} - EXPECTED_BUILDS.keys()
    if unknown:
        raise ValueError(f'Unsupported products: {sorted(unknown)}')
    args.output.mkdir(parents=True, exist_ok=True)
    products = {p['Product']: p for p in closure_casc.installed_products(args.install)}
    indices, _, sources = closure_casc.load_indices(args.install)
    wanted = {row['fdid'] for row in assets}
    authenticated = {name: authenticate_root(args.install, products[name], wanted, indices, args.output)
                     for name in sorted(EXPECTED_BUILDS)}
    results = []
    for asset in assets:
        identity, roots = authenticated[asset['product']]
        row = publish_legacy(args.data, identity, asset, roots)
        if row['status'] == 'content_mismatch':
            row['other_authenticated_matches'] = [name for name, (_, keys) in authenticated.items()
                                                  if keys.get(asset['fdid'], b'').hex() == row['legacy_content_key']]
        results.append(row)
    summary = {'counts': dict(Counter(r['status'] for r in results)), 'idx_sources': sources,
               'builds': {name: identity for name, (identity, _) in authenticated.items()}}
    (args.output / 'results.json').write_text(json.dumps({'summary': summary, 'assets': results}, indent=2) + '\n')
    print(json.dumps(summary['counts']))


if __name__ == '__main__':
    main()
