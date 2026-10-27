#!/usr/bin/env python3
"""Generate local player manifests and packs without materializing the bundle."""

import argparse
import json
import subprocess
from pathlib import Path

from launcher_packs import build_packs
from manifest_diff import diff_manifests
from player_bundle import mapped_manifest, source_map
from verified_upload import upload_file


def identities(manifest):
    return {entry['path']: (entry['sha256'], entry['size']) for entry in manifest['files']}


def build(root, generated, windows, output, pack_cache):
    output.mkdir(parents=True, exist_ok=True)
    manifest_path = output / 'manifest.json'
    previous = json.loads(manifest_path.read_text()) if manifest_path.exists() else {'files': []}
    sources = source_map(root, generated, windows)
    source_stats = {}
    manifest = mapped_manifest(sources, output / 'digest-cache.json', source_stats)
    build_packs(root, pack_cache, manifest, source_map=sources, source_stats=source_stats)
    upload, delete = diff_manifests(identities(previous), identities(manifest))
    (output / 'upload.txt').write_text(''.join(f'{name}\n' for name in upload))
    (output / 'delete.txt').write_text(''.join(f'{name}\n' for name in delete))
    manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False))
    return sources, manifest


def publish(sources, manifest, live, output, pack_cache, destination, rclone='rclone'):
    upload, _ = diff_manifests(live, identities(manifest))
    entries = {entry['path']: entry for entry in manifest['files']}
    for name in upload:
        upload_file(sources[name], destination, entries[name], rclone)
    headers = ['--header-upload', 'Cache-Control: no-cache']
    subprocess.run([rclone, 'copyto', str(output / 'manifest.json'),
                    f'{destination}/manifest.json', *headers], check=True)
    subprocess.run([rclone, 'copy', str(pack_cache / 'packs'), f'{destination}/packs',
                    '--immutable', '--transfers', '48', '--checkers', '16',
                    '--low-level-retries', '20'], check=True)
    subprocess.run([rclone, 'check', str(pack_cache / 'packs'), f'{destination}/packs',
                    '--one-way', '--size-only', '--checkers', '16',
                    '--low-level-retries', '20'], check=True)
    subprocess.run([rclone, 'copyto', str(pack_cache / 'manifest-v2.json'),
                    f'{destination}/manifest-v2.json', *headers], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', required=True, type=Path)
    parser.add_argument('--generated', required=True, type=Path)
    parser.add_argument('--windows-artifacts', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--pack-cache', required=True, type=Path)
    parser.add_argument('--destination', help='publish to this rclone remote')
    parser.add_argument('--live-manifest', type=Path)
    args = parser.parse_args()
    sources, manifest = build(args.root, args.generated, args.windows_artifacts, args.output, args.pack_cache)
    if args.destination:
        live = identities(json.loads(args.live_manifest.read_text())) if args.live_manifest and args.live_manifest.exists() else {}
        publish(sources, manifest, live, args.output, args.pack_cache, args.destination)
    else:
        print(f'Generated manifests for {len(sources)} files; no publication performed')


if __name__ == '__main__':
    main()
