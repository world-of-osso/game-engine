#!/usr/bin/env python3
"""Compare deploy manifests, allowing Godot UID cache record reordering.

Two cold imports of the same old bundle at the same path produced UID-cache
SHA256 33ddfcfece0d8cf3e084aa9ca495d08adb43c90c52c7d3cabfb411768ff9d45c
and c42750ddc37d5551343d20d4d578219f11290563ab88ae2a33843006f2cfcc93.
The binary file itself must not be rewritten to force the comparison to pass.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

UID_PATH = 'godot/.godot/uid_cache.bin'


def uid_pairs(path):
    data = path.read_bytes()
    try:
        count = struct.unpack_from('<I', data, 0)[0]
        offset = 4
        pairs = {}
        paths = set()
        for _ in range(count):
            uid, length = struct.unpack_from('<QI', data, offset)
            offset += 12
            raw = data[offset:offset + length]
            if len(raw) != length:
                raise ValueError('truncated path')
            name = raw.decode('utf-8')
            offset += length
            if uid in pairs or name in paths:
                raise ValueError('duplicate UID or path')
            pairs[uid] = name
            paths.add(name)
        if offset != len(data):
            raise ValueError('trailing data')
        return pairs
    except (struct.error, UnicodeDecodeError, ValueError) as error:
        raise ValueError(f'invalid uid_cache.bin {path}: {error}') from error


def compare_manifests(old, new, old_uid, new_uid):
    if {key: value for key, value in old.items() if key != 'files'} != {
        key: value for key, value in new.items() if key != 'files'
    }:
        raise ValueError('manifest metadata differs')
    old_files = {entry['path']: entry for entry in old['files']}
    new_files = {entry['path']: entry for entry in new['files']}
    if len(old_files) != len(old['files']) or len(new_files) != len(new['files']):
        raise ValueError('duplicate manifest paths')
    if old_files.keys() != new_files.keys():
        missing = sorted(old_files.keys() - new_files.keys())
        extra = sorted(new_files.keys() - old_files.keys())
        raise ValueError(f'manifest paths differ: missing={missing[:10]}, extra={extra[:10]}')
    if UID_PATH not in old_files:
        raise ValueError(f'missing {UID_PATH}')
    different = [name for name in old_files if name != UID_PATH and old_files[name] != new_files[name]]
    if different:
        raise ValueError(f'{len(different)} manifest entries differ: {different[:10]}')
    for entry, path in ((old_files[UID_PATH], old_uid), (new_files[UID_PATH], new_uid)):
        data = path.read_bytes()
        if len(data) != entry['size'] or hashlib.sha256(data).hexdigest() != entry['sha256']:
            raise ValueError(f'uid_cache.bin does not match manifest: {path}')
    if uid_pairs(old_uid) != uid_pairs(new_uid):
        raise ValueError('uid_cache.bin UID-to-path mappings differ')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--old', type=Path, required=True)
    parser.add_argument('--new', type=Path, required=True)
    parser.add_argument('--old-uid', type=Path, required=True)
    parser.add_argument('--new-uid', type=Path, required=True)
    args = parser.parse_args()
    compare_manifests(json.loads(args.old.read_text()), json.loads(args.new.read_text()),
                      args.old_uid, args.new_uid)
    print('Manifests match; UID caches have identical UID-to-path pairs')


if __name__ == '__main__':
    main()
