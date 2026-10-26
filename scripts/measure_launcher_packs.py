#!/usr/bin/env python3
"""Read-only live launcher sample. Supply a previously downloaded manifest path.

Fetches a seeded uniform sample with at most 8 MiB per selected file, verifies
SHA-256, writes/renames in an isolated temporary directory, and removes it.
It is not the Rust launcher and does not measure a full install or update.
"""
import concurrent.futures
import hashlib
import json
import random
import tempfile
import time
import urllib.parse
import urllib.request
from pathlib import Path
import sys

BASE = 'https://files.worldofosso.com/'
UA = 'WorldOfOsso-Launcher/0.4.0'
LIMIT = 8 * 1024 * 1024


def transfer(entry, directory):
    url = BASE + urllib.parse.quote(entry['path'], safe='/')
    start = time.monotonic()
    request = urllib.request.Request(url, headers={'User-Agent': UA})
    with urllib.request.urlopen(request, timeout=60) as response:
        headers = time.monotonic()
        data = response.read()
    received = time.monotonic()
    digest = hashlib.sha256(data).hexdigest()
    hashed = time.monotonic()
    if len(data) != entry['size'] or digest != entry['sha256']:
        raise ValueError('size or hash mismatch: ' + entry['path'])
    path = Path(directory) / entry['path']
    path.parent.mkdir(parents=True, exist_ok=True)
    partial = path.with_name(path.name + '.part')
    partial.write_bytes(data)
    partial.rename(path)
    done = time.monotonic()
    return {'bytes': len(data), 'headers_s': headers-start, 'body_s': received-headers,
            'hash_s': hashed-received, 'write_rename_s': done-hashed, 'total_s': done-start}


def main():
    manifest = json.loads(Path(sys.argv[1]).read_bytes())
    entries = [e for e in manifest['files'] if not e['path'].startswith('game-engine-')
               or e['path'] == 'game-engine-windows-x86_64']
    limits = [0, 1024, 4096, 16384, 65536, 262144, 1048576, 8388608]
    bins = {str(bound): {'count': 0, 'bytes': 0} for bound in limits}
    for entry in entries:
        size = entry['size']
        key = str(next((b for b in reversed(limits) if size >= b), 0))
        bins[key]['count'] += 1
        bins[key]['bytes'] += size
    eligible = [e for e in entries if e['size'] <= LIMIT]
    sample = random.Random(20261026).sample(eligible, min(256, len(eligible)))
    start = time.monotonic()
    with tempfile.TemporaryDirectory(prefix='launcher-pack-', dir=Path(sys.argv[1]).parent) as directory:
        with concurrent.futures.ThreadPoolExecutor(max_workers=32) as pool:
            results = list(pool.map(lambda e: transfer(e, directory), sample))
        elapsed = time.monotonic() - start
    result = {'version': manifest.get('version'), 'manifest_files': len(manifest['files']),
              'windows_files': len(entries), 'windows_bytes': sum(e['size'] for e in entries),
              'bins_lower_bound_bytes': bins, 'eligible_count': len(eligible),
              'sample_count': len(sample), 'sample_elapsed_s': elapsed,
              'sample_bytes': sum(r['bytes'] for r in results), 'samples': results}
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
