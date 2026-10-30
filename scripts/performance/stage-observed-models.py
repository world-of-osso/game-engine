"""Authenticate the observed1002-root Northshire/Ruby terrain frontier privately."""
from pathlib import Path
from collections import Counter
import hashlib
import json
import os
import shutil
import sqlite3
import struct
import sys
import time
import subprocess
import argparse

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description="Stage authenticated M2 chains for a recorded receipt-miss frontier")
parser.add_argument("--roots", type=Path, required=True, help="JSON array of observed Retail M2 FDIDs")
parser.add_argument("--output", type=Path, required=True, help="New private output directory")
parser.add_argument("--canonical-data", type=Path, required=True)
parser.add_argument("--install", type=Path, required=True)
parser.add_argument("--extractor", type=Path, required=True)
args = parser.parse_args()
PROOF = args.output.resolve()
PROOF.mkdir(parents=True, exist_ok=True)
DATA = args.canonical_data.resolve()
INSTALL = args.install.resolve()
WORK = PROOF / 'model-closure'
STAGED = PROOF / 'staged-models'
sys.path.insert(0, str(ROOT / 'scripts'))
from asset_closure import RawByteGraph as Closure, chunks, records, u32
import closure_casc
import import_model_asset_chains as chains
import model_asset_publication as publication
import verify_legacy_product_assets as verifier
from import_forever_skyborne import validate_magic


def check_disk(initial=False):
    if shutil.disk_usage('/').free < (340 if initial else 335) * 2**30:
        raise RuntimeError('Terrain staging disk floor')


def check_staging_budget():
    paths = [str(path) for path in [WORK, STAGED] if path.exists()]
    if not paths:
        return
    result = subprocess.run(['du', '-sb', *paths], text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError('Private staging size check failed: ' + result.stderr)
    used = sum(int(line.split()[0]) for line in result.stdout.splitlines())
    if used > 20 * 2**30:
        raise RuntimeError(f'Private staging budget20GiB exceeded: {used}')
    (WORK / 'staging-budget.json').write_text(json.dumps({'apparent_bytes': used, 'limit_bytes': 20 * 2**30}))


def admit_decode():
    check_staging_budget()
    while True:
        result = subprocess.run(['/home/osso/.worktrees/logs/capacity.sh'], env=dict(os.environ, CAPACITY_ID='fps120-assets'), text=True, capture_output=True)
        with (WORK / 'capacity.log').open('a') as stream:
            stream.write(result.stdout)
        if 'SHED:' in result.stdout:
            raise RuntimeError('Terrain decoder SHED')
        if 'ADD:' in result.stdout:
            check_disk()
            return
        time.sleep(60)


def receipt_owners():
    return {(row['product'], row['fdid'], row['kind']): row
            for path in (DATA / 'cache').glob('model-asset-index*.json')
            for row in json.loads(path.read_text()).get('assets', [])}


def placement_models(case, product):
    roots = json.loads(args.roots.read_text())
    if not roots or any(not isinstance(fdid, int) or fdid <= 0 for fdid in roots):
        raise ValueError('Expected a nonempty JSON array of positive FDIDs')
    return sorted(set(roots))


def graph_at(data, product, model_roots, round_number):
    graph = Closure(data, {}, product, 'active', path_cache=data / 'local-listfile-cache.sqlite')
    for fdid in model_roots:
        graph.add(fdid, 'm2', f'{product} selected-map M2 placement closure')
    result = graph.run()
    (WORK / f'{product}-graph{round_number}.json').write_text(json.dumps(result, indent=2))
    return result


if os.environ.get('GAME_ENGINE_ASSET_MODE') != 'extracted-only':
    raise RuntimeError('Closure host must remain extracted-only')
cap = subprocess.run(['/home/osso/.worktrees/logs/capacity.sh'], env=dict(os.environ, CAPACITY_PEEK='1'), text=True, capture_output=True)
if 'SHED:' in cap.stdout:
    raise RuntimeError('Metadata authentication SHED')
check_disk(initial=True)
publication.validate_staged_fragment(DATA, {'version': 1, 'assets': [], 'aliases': [], 'chains': []})
WORK.mkdir(exist_ok=False)
indices, _, _ = closure_casc.load_indices(INSTALL)
products = {row['Product']: row for row in closure_casc.installed_products(INSTALL)}
owners = receipt_owners()
plans = []
case_summaries = []
for product, build, case in [('wow', products['wow']['Version'], 'observed-frontier')]:
    data = WORK / product
    data.mkdir()
    for directory in ['models', 'textures']:
        (data / directory).mkdir()
    (data / 'db2').symlink_to(DATA / 'db2', target_is_directory=True)
    (data / 'cache').mkdir()
    with sqlite3.connect((DATA / 'local-listfile-cache.sqlite').resolve().as_uri() + '?mode=ro', uri=True) as src, sqlite3.connect(data / 'local-listfile-cache.sqlite') as dst:
        src.backup(dst)
    roots = placement_models(case, product)
    failures = {}
    authenticated = {}
    identity = None
    for round_number in range(1, 13):
        check_staging_budget()
        graph = graph_at(data, product, roots, round_number)
        pending = [row for row in graph['assets']
                   if not row['present'] and (row['fdid'], row['type']) not in failures]
        if not pending:
            break
        # Existing receipts retain their actual historical identity; never relabel them.
        copied = 0
        unresolved = []
        for row in pending:
            owner = owners.get((product, row['fdid'], row['type']))
            if owner is None:
                unresolved.append(row)
                continue
            relative = f"{'textures' if row['type'] == 'blp' else 'models'}/{row['fdid']}.{row['type']}"
            destination = data / relative
            if not destination.exists():
                destination.symlink_to(DATA / owner['path'])
                copied += 1
        if unresolved:
            output = WORK / f'{product}-authentication{round_number}'
            output.mkdir()
            identity, source_roots = verifier.authenticate_root(INSTALL, products[product], {row['fdid'] for row in unresolved}, indices, output, expected_build=build)
            decode = []
            for row in unresolved:
                fdid, kind = row['fdid'], row['type']
                relative = f"{'textures' if kind == 'blp' else 'models'}/{fdid}.{kind}"
                content = source_roots.get(fdid)
                if content is None:
                    failures[fdid, kind] = dict(row, status='root_absent')
                    continue
                authenticated[fdid, kind] = content.hex()
                source = DATA / relative
                if source.is_file() and hashlib.md5(source.read_bytes()).digest() == content:
                    check_disk()
                    raw = source.read_bytes()
                    validate_magic(raw, kind, content.hex())
                    (data / relative).write_bytes(raw)
                    copied += 1
                else:
                    decode.append({'fdid': fdid, 'type': kind, 'locations': [relative]})
            if decode:
                inventory = WORK / f'{product}-inventory{round_number}.json'
                inventory.write_text(json.dumps({product: decode}, indent=2))
                admit_decode()
                extracted = WORK / f'{product}-extraction{round_number}'
                argv = [str(ROOT / 'scripts/agent/agent-run'), 'observedmodels', 'python3', str(ROOT / 'scripts/recheck_local_asset_gaps.py'), '--inventory', str(inventory), '--install', str(INSTALL), '--data', str(data), '--executable', str(args.extractor.resolve()), '--output', str(extracted)]
                result = subprocess.run(argv, text=True, capture_output=True)
                (WORK / f'{product}-decode{round_number}.log').write_text(result.stdout + result.stderr)
                if result.returncode:
                    raise RuntimeError(f'{product} terrain decoder exit{result.returncode}')
                for row in json.loads((extracted / product / 'results.json').read_text()):
                    if row['status'] != 'readable':
                        failures[row['fdid'], row['type']] = row
        print('TERRAIN_FRONTIER', product, round_number, len(pending), copied, flush=True)
    else:
        raise RuntimeError(f'{product} model graph did not converge')
    # Authenticate all present rows: identical historical owners may require current-build companion aliases.
    publishable = [row for row in graph['assets'] if row['present']]
    if publishable:
        final_auth = WORK / f'{product}-authentication-final'
        final_auth.mkdir()
        identity, final_roots = verifier.authenticate_root(INSTALL, products[product], {row['fdid'] for row in publishable}, indices, final_auth, expected_build=build)
    for row in publishable:
        fdid, kind = row['fdid'], row['type']
        relative = f"{'textures' if kind == 'blp' else 'models'}/{fdid}.{kind}"
        raw = (data / relative).read_bytes()
        current_content = final_roots.get(fdid)
        if current_content is None:
            failures[fdid, kind] = dict(row, status='current_root_absent_existing_owner')
            continue
        content = current_content.hex()
        if hashlib.md5(raw).hexdigest() != content:
            failures[fdid, kind] = dict(row, status='existing_owner_differs_current_root', current_content_key=content, existing_content_key=hashlib.md5(raw).hexdigest())
            continue
        validate_magic(raw, kind, content)
        plans.append((chains.create_asset_receipt(identity, fdid, kind, relative, raw, content), data / relative))
    case_summaries.append({'product': product, 'case': case, 'models': len(roots), 'graph': graph['summary'], 'source_gaps': list(failures.values())})
fragment = publication.deduplicate_staged_fragment(DATA, {'version': 1, 'assets': [r for r, _ in plans], 'aliases': [], 'chains': []})
publication.validate_staged_fragment(DATA, fragment)
new_paths = {row['path'] for row in fragment['assets'] + fragment['aliases']}
for receipt, source in plans:
    check_disk()
    if receipt['path'] in new_paths:
        chains.write_asset_files(STAGED, [(receipt, source.read_bytes())])
publication.validate_staged_fragment(DATA, fragment)
path = STAGED / 'cache/model-asset-index-observed-models.json'
path.parent.mkdir(parents=True, exist_ok=True)
with path.open('x') as stream:
    json.dump(fragment, stream, indent=2)
summary = {'fragment': str(path), 'assets': len(fragment['assets']), 'aliases': len(fragment['aliases']), 'bytes': sum(row['bytes'] for row in fragment['assets'] + fragment['aliases']), 'deduplicated': len(plans) - len(fragment['assets']) - len(fragment['aliases']), 'cases': case_summaries, 'fragment_sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
check_staging_budget()
(WORK / 'summary.json').write_text(json.dumps(summary, indent=2))
print('TERRAIN_MODEL_STAGE_DONE', json.dumps(summary), flush=True)
