#!/usr/bin/env python3
"""Resumable batches using the existing casc-local executable, never CDN."""
import os
from pathlib import Path
import re
import shutil
import subprocess

from closure_extract import file_hashes, publish, runtime_path


def pending_assets(assets, data, receipts, failed):
    pending = []
    for asset in assets:
        if asset['fdid'] in failed:
            continue
        for relative in asset['locations']:
            target = runtime_path(data, relative)
            if not target.is_file():
                pending.append(asset)
                break
            receipt = receipts.get(relative)
            if receipt and file_hashes(target)[2] != receipt['sha256']:
                pending.append(asset)
                break
    return pending


def extract_batch(assets, identities, executable, data, output, batch, cache, receipts, environment):
    stage = output / f'batch-{batch:05d}'
    stage.mkdir(parents=True, exist_ok=True)
    log = output / f'batch-{batch:05d}.log'
    fdids = sorted({a['fdid'] for a in assets})
    env = dict(os.environ, WOW_PRODUCT='wow', GAME_ENGINE_ASSET_MODE='local-casc')
    env.update(environment)
    with log.open('w') as stream:
        process = subprocess.run([str(executable), *map(str, fdids), '-o', str(stage)],
                                 stdout=stream, stderr=stream, env=env, check=False)
    text = log.read_text()
    if f'using wow cache {cache}' not in text:
        raise RuntimeError(f'extractor did not confirm expected product/build/cache; see {log}')
    failures = {int(n): reason for n, reason in re.findall(r'Failed FDID (\d+): (.*)', text)}
    encrypted = {int(n) for n in re.findall(r'FDID (\d+) missing TACT keys', text)}
    files = {int(p.stem): p for p in stage.iterdir() if p.is_file() and p.stem.isdecimal()}
    result = {'files': 0, 'bytes': 0, 'by_type': {}, 'failures': failures,
              'extractor_exit': process.returncode, 'log': str(log)}
    for asset in assets:
        fdid = asset['fdid']
        if fdid in encrypted:
            failures[fdid] = 'encrypted/unknown TACT key; zero-filled output rejected'
            continue
        path = files.get(fdid)
        if path is None:
            failures.setdefault(fdid, f'extractor returned no file (exit {process.returncode}); see {log}')
            continue
        try:
            identity = dict(identities[fdid], type=asset['type'])
            publication = publish(path, data, asset['locations'], identity,
                                  data / 'provenance/closure-extract.jsonl', receipts)
        except (ValueError, OSError) as error:
            failures[fdid] = str(error)
            continue
        if publication['conflicts']:
            failures[fdid] = 'existing runtime bytes differ: ' + ', '.join(publication['conflicts'])
        result['files'] += publication['created']
        result['bytes'] += publication['bytes']
        totals = result['by_type'].setdefault(asset['type'], {'files': 0, 'bytes': 0})
        totals['files'] += publication['created']
        totals['bytes'] += publication['bytes']
    shutil.rmtree(stage)
    return result
