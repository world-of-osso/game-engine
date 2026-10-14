#!/usr/bin/env python3
"""Retail mount closure with installed-build extraction and authored DB2 metadata.

No canonical mutation. New receipts authenticate against the current local content
root; historical receipts retain their original authenticated byte identity.
"""
from collections import defaultdict
import csv
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3

import asset_closure as audit
import closure_casc as casc
from closure_seeds import Catalogs, seed_displays
from import_forever_skyborne import validate_magic
from import_model_asset_chains import create_asset_receipt, write_asset_files
from apply_mount_model_assets import index_hashes, sha

from db2_source import active_directory, source_identity
SUPPORTED = {'m2', 'skin', 'skel', 'anim', 'bone', 'phys', 'blp'}


def read_rows(path):
    with path.open(newline='', encoding='utf-8-sig') as stream:
        return list(csv.DictReader(stream))


def authenticate_install(install):
    rows = [row for row in casc.installed_products(install) if row['Product'] == 'wow']
    if len(rows) != 1:
        raise ValueError(f'expected one active Retail build; got {len(rows)}')
    info = rows[0]
    build_key, build = info['Build Key'], info['Version']
    config_path = install / 'Data/config' / build_key[:2] / build_key[2:4] / build_key
    config_bytes = config_path.read_bytes()
    if hashlib.md5(config_bytes).hexdigest() != build_key:
        raise ValueError('build config content hash differs')
    config = dict(line.split(' = ', 1) for line in config_bytes.decode().splitlines() if ' = ' in line)
    indices, _, _ = casc.load_indices(install)
    encoding_ck, encoding_ek = config['encoding'].split()
    encoding = casc.read_metadata(install, indices, encoding_ek, encoding_ck)
    root_ck = config['root']
    root_record = casc.encoding_keys(encoding, {bytes.fromhex(root_ck)})[bytes.fromhex(root_ck)]
    root_ek = next(key for key in root_record[1] if key[:9] in indices)
    root = casc.read_metadata(install, indices, root_ek.hex(), root_ck)
    identity = {'product': 'wow', 'build_key': build_key, 'build': build,
                'build_config_sha256': hashlib.sha256(config_bytes).hexdigest(),
                'root_content_key': root_ck, 'encoding_content_key': encoding_ck,
                'extraction_source': 'authenticated-local-CASC-content-root'}
    return identity, casc.root_keys(root, range(2**32))


def transitive_assets(roots, assets):
    children = defaultdict(set)
    by_fdid = defaultdict(set)
    for key, asset in assets.items():
        by_fdid[key[0]].add(key)
        for parent, _ in asset['edges']:
            if parent is not None:
                children[parent].add(key)
    result, pending = set(), list(roots)
    while pending:
        key = pending.pop()
        if key in result:
            continue
        result.add(key)
        pending.extend(children[key[0]] - result)
    return result


def census(links, mounts, displays, models, assets, receipts, failures):
    grouped = defaultdict(list)
    rows = []
    for link in links:
        mid, did = int(link['MountID']), int(link['CreatureDisplayInfoID'])
        display = displays.get(did, {})
        model = int(models.get(int(display.get('ModelID', 0)), {}).get('FileDataID', 0))
        roots = {(model, 'm2')} if model else set()
        roots.update((int(value), 'blp') for key, value in display.items() if key.startswith('TextureVariationFileDataID_') and int(value))
        # The graph includes full authored customization/item roots for each Extra.
        extra = int(display.get('ExtendedDisplayInfoID', 0))
        if extra:
            roots.update(key for key, asset in assets.items() if any(f'display {did} ' in reason for _, reason in asset['edges']))
        required = transitive_assets(roots, assets)
        missing = sorted(key for key in required if key not in receipts)
        reasons = [f'{fdid}.{kind}: {failures.get((fdid, kind), "no verified asset receipt")}' for fdid, kind in missing]
        if not model:
            reasons.append('missing display/model metadata')
        row = {'mount_id': mid, 'mount_row_available': mid in mounts, 'display_id': did,
               'model_fdid': model, 'extra_id': extra,
               'model_receipt': (model, 'm2') in receipts,
               'verified_asset_chain': bool(model) and not reasons,
               'required_assets': [f'{fdid}.{kind}' for fdid, kind in sorted(required)], 'reasons': reasons}
        grouped[mid].append(row)
        rows.append(row)
    return {'mount_ids': len(grouped), 'mount_rows_available': len(set(grouped) & mounts.keys()),
            'mounts_with_model_receipt': sum(any(row['model_receipt'] for row in group) for group in grouped.values()),
            'mounts_with_verified_asset_chain': sum(any(row['verified_asset_chain'] for row in group) for group in grouped.values()),
            'mounts_all_display_chains_verified': sum(all(row['verified_asset_chain'] for row in group) for group in grouped.values()),
            'distinct_models': len({row['model_fdid'] for row in rows} - {0}), 'display_links': rows}


def seed_mounts(graph, catalogs):
    links = catalogs.rows('MountXDisplay')
    mounts = catalogs.index('Mount')
    displays, models = catalogs.index('CreatureDisplayInfo'), catalogs.index('CreatureModelData')
    seed_displays(graph, catalogs, [int(row['CreatureDisplayInfoID']) for row in links])
    return lambda assets, receipts, failures: census(links, mounts, displays, models, assets, receipts, failures)


def stage(data, package, install, extract):
    return stage_closure(data, package, install, extract, seed_mounts,
                         'every MountXDisplay row, all alternative displays; pinned Retail 69933',
                         'model-asset-index-mount-model-assets-20261012.json')


def read_existing(data, packages):
    """Receipts already published in data, or staged by other unpublished packages."""
    existing = {}
    for root in [data, *packages]:
        names = index_hashes(data) if root == data else sorted(path.name for path in (root / 'cache').glob('model-asset-index-*.json'))
        for name in names:
            for receipt in json.loads((root / 'cache' / name).read_text())['assets']:
                if receipt['product'] == 'wow':
                    existing[receipt['fdid'], receipt['kind']] = (receipt, root)
    return existing


def stage_closure(data, package, install, extract, seed, scope, fragment_name, packages=()):
    """`seed(graph, catalogs)` adds roots and returns `summarize(assets, receipts, failures)`."""
    package.mkdir(parents=True, exist_ok=True)
    proof = package / '_proof'
    proof.mkdir(exist_ok=True)
    before_hashes = index_hashes(data)
    (proof / 'before-index-hashes.json').write_text(json.dumps(before_hashes, indent=2))
    existing = read_existing(data, packages)
    identity, roots = authenticate_install(install)
    (proof / 'source-identity.json').write_text(json.dumps(identity, indent=2))
    paths, runtime_paths = audit.read_listfile(data / 'community-listfile.csv')
    verified, new, failures, checked = {}, {}, {}, {}

    class AuthenticatedClosure(audit.Closure):
        def inspect(self, asset):
            key = asset['fdid'], asset['type']
            if key in checked:
                path = checked[key]
            else:
                path = None
                receipt, receipt_root = existing.get(key, (None, None))
                candidates = [package / r['path'] for r in [new[key]] if key in new] if key in new else []
                if receipt:
                    candidates.append(receipt_root / receipt['path'])
                candidates.extend(data / name for name in sorted(asset['locations']))
                expected = roots.get(key[0])
                for candidate in candidates:
                    if not candidate.is_file():
                        continue
                    raw = candidate.read_bytes()
                    historical = receipt is not None and candidate == receipt_root / receipt['path']
                    content_key = bytes.fromhex(receipt['content_key']) if historical else expected
                    if content_key is None or hashlib.md5(raw).digest() != content_key:
                        continue
                    if historical and hashlib.sha256(raw).hexdigest() != receipt['sha256']:
                        raise ValueError(f'existing receipt hash disagrees: {key}')
                    if asset['type'] in SUPPORTED:
                        if asset['type'] != 'phys':
                            validate_magic(raw, asset['type'], content_key.hex())
                        directory = 'textures' if asset['type'] == 'blp' else 'models'
                        record = create_asset_receipt(identity, key[0], key[1], f'{directory}/{key[0]}.{key[1]}', raw, content_key.hex())
                        if historical:
                            verified[key] = receipt
                        else:
                            write_asset_files(package, [(record, raw)])
                            new[key] = record
                            verified[key] = record
                            candidate = package / record['path']
                    path = candidate
                    break
                checked[key] = path
            asset['present'] = path is not None
            asset['size'] = path.stat().st_size if path else None
            asset['sha256'] = sha(path) if path else None
            asset['present_files'] = [{'path': str(path), 'size': asset['size'], 'sha256': asset['sha256']}] if path else []
            asset['identity_status'] = 'authenticated_local_root' if path else 'missing'
            asset['source_product'] = 'wow' if path else None
            record = verified.get(key)
            asset['actual_build'] = record['build_key'] if record else None
            return path

    metadata_build = source_identity(active_directory(data))['build']
    graph = AuthenticatedClosure(data, paths, 'wow', metadata_build, runtime_paths=runtime_paths)
    graph.seeds = {'scope': scope}
    summarize = seed(graph, Catalogs(graph))
    graph.run()
    before_receipts = {key: receipt for key, receipt in verified.items() if key in existing}
    initial = summarize(graph.assets, before_receipts, failures)
    (proof / 'census-before.json').write_text(json.dumps(initial, indent=2))
    print('BEFORE', {key: value for key, value in initial.items() if key != 'display_links'}, flush=True)
    for roundno in range(1, 16):
        pending = [key for key, asset in graph.assets.items() if not asset['present'] and key[1] in SUPPORTED and key not in failures]
        if not pending:
            break
        for offset in range(0, len(pending), 160):
            batch = sorted(pending[offset:offset + 160])
            output = proof / 'extraction' / f'r{roundno}-b{offset//160}'
            output.mkdir(parents=True, exist_ok=True)
            result = extract(sorted({key[0] for key in batch}), output)
            (output / 'extract.log').write_text(result.stdout + result.stderr)
            payloads = {int(path.stem): path for path in output.iterdir() if path.stem.isdecimal()}
            for fdid, kind in batch:
                key = fdid, kind
                expected, path = roots.get(fdid), payloads.get(fdid)
                if expected is None or path is None:
                    message = next((line for line in result.stderr.splitlines() if f'Failed FDID {fdid}:' in line), 'absent local CASC payload/root')
                    failures[key] = message
                    continue
                raw = path.read_bytes()
                if hashlib.md5(raw).digest() != expected:
                    failures[key] = 'local payload content hash differs (unknown TACT key zero-fill or wrong content)'
                    continue
                try:
                    if kind != 'phys':
                        validate_magic(raw, kind, expected.hex())
                    directory = 'textures' if kind == 'blp' else 'models'
                    record = create_asset_receipt(identity, fdid, kind, f'{directory}/{fdid}.{kind}', raw, expected.hex())
                    write_asset_files(package, [(record, raw)])
                    new[key] = record
                    checked.pop(key, None)
                    graph.queue.append(key)
                except (ValueError, OSError) as error:
                    failures[key] = str(error)
            print('EXTRACT', roundno, offset, 'new', len(new), 'failures', len(failures), flush=True)
        graph.run()
    else:
        raise ValueError('asset closure failed to converge')
    final_graph = graph.run()
    final_graph['summary']['unverified_identity'] = sum(asset['identity_status'] != 'authenticated_local_root' for asset in final_graph['assets'])
    aliases = []
    for key, asset in graph.assets.items():
        receipt = verified.get(key)
        if not receipt:
            continue
        source = package / receipt['path'] if key in new else existing[key][1] / receipt['path']
        for location in sorted(asset['locations']):
            if location == f'{"textures" if key[1] == "blp" else "models"}/{key[0]}.{key[1]}':
                continue
            alias = {**receipt, 'path': f'products/wow/{receipt["build_key"]}/{location}'}
            target = next((root / alias['path'] for root in [data, *packages] if (root / alias['path']).exists()), None)
            if target:
                if sha(target) != receipt['sha256']:
                    raise ValueError(f'alias conflicts: {target}')
            else:
                write_asset_files(package, [(alias, source.read_bytes())])
                aliases.append(alias)
    # Runtime model-asset-index accepts only native kinds; PFID satellites are not
    # consumed by previews. Keep their authenticated audit, not runtime mappings.
    physics = [receipt for receipt in new.values() if receipt['kind'] == 'phys']
    for receipt in physics:
        previous = package / receipt['path']
        receipt['path'] = f'_proof/optional-physics/{receipt["fdid"]}.phys'
        destination = package / receipt['path']
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(previous, destination)
        previous.unlink()
        for asset in final_graph['assets']:
            if asset['fdid'] == receipt['fdid'] and asset['type'] == 'phys':
                asset['present_files'][0]['path'] = str(destination)
    (proof / 'optional-physics-satellites.json').write_text(json.dumps(physics, indent=2))
    fragment = {'version': 1, 'assets': [receipt for receipt in new.values() if receipt['kind'] != 'phys'], 'aliases': aliases,
                'chains': [{**identity, 'scope': graph.seeds, 'metadata': list(graph.inputs.values())}]}
    cache = package / 'cache'
    cache.mkdir(exist_ok=True)
    (cache / fragment_name).write_text(json.dumps(fragment, indent=2) + '\n')
    final = summarize(graph.assets, verified, failures)
    (proof / 'census-after.json').write_text(json.dumps(final, indent=2))
    (proof / 'asset-graph.json').write_text(json.dumps(final_graph, indent=2))
    (proof / 'failures.json').write_text(json.dumps([{'fdid': key[0], 'kind': key[1], 'reason': reason} for key, reason in sorted(failures.items())], indent=2))
    print('AFTER', {key: value for key, value in final.items() if key != 'display_links'}, flush=True)
    return fragment


def manifest(data, package):
    inputs = json.loads((package / '_proof/before-index-hashes.json').read_text())
    artifacts = []
    for directory in ['products', 'cache']:
        for path in sorted((package / directory).rglob('*')):
            if not path.is_file():
                continue
            relative = str(path.relative_to(package))
            target = data / relative
            artifacts.append({'path': relative, 'source_sha256': sha(path), 'bytes': path.stat().st_size,
                              'expected_before_sha256': sha(target) if target.exists() else None})
    plan = {'canonical_data': str(data), 'package': str(package), 'expected_index_inputs': inputs,
            'artifacts': artifacts, 'added_bytes': sum(row['bytes'] for row in artifacts if row['expected_before_sha256'] is None)}
    (package / 'manifest.json').write_text(json.dumps(plan, indent=2) + '\n')
    return plan
