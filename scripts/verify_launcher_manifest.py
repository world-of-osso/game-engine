#!/usr/bin/env python3
"""Verify v2 pack ranges reconstruct every file named by the v1 manifest."""

import argparse
from collections import OrderedDict
import hashlib
import json
import os
from pathlib import Path
import re
import struct

HEADER = b'OSSPACK\x01'


def pack_sizes(manifest):
    sizes = {}
    for file in manifest['files']:
        for chunk in file['chunks']:
            digest, size = chunk['pack'], chunk['pack_size']
            if not re.fullmatch('[0-9a-f]{64}', digest) or size < len(HEADER):
                raise ValueError(f"invalid pack in {file['path']}")
            if digest in sizes and sizes[digest] != size:
                raise ValueError(f'inconsistent pack size: {digest}')
            sizes[digest] = size
    return sizes


def additional_pack_bytes(published, current):
    old = pack_sizes(published)
    new = pack_sizes(current)
    missing = {digest: size for digest, size in new.items() if digest not in old}
    return len(missing), sum(missing.values())


def verify(v1, v2, packdir):
    originals = {entry['path']: entry for entry in v1['files']}
    entries = {entry['path']: entry for entry in v2['files']}
    if len(originals) != len(v1['files']) or len(entries) != len(v2['files']):
        raise ValueError('duplicate file path')
    if originals.keys() != entries.keys():
        raise ValueError('v1 and v2 path sets differ')
    sizes = pack_sizes(v2)
    by_pack = {digest: set() for digest in sizes}
    multiple = []
    for name, entry in entries.items():
        original = originals[name]
        if (entry['size'], entry['sha256']) != (original['size'], original['sha256']):
            raise ValueError(f'v1/v2 file identity differs: {name}')
        chunks = entry['chunks']
        if not chunks or sum(chunk['size'] for chunk in chunks) != entry['size']:
            raise ValueError(f'invalid chunk sizes: {name}')
        if len(chunks) == 1 and chunks[0]['sha256'] != entry['sha256']:
            raise ValueError(f'file hash differs from chunk: {name}')
        if len(chunks) > 1:
            multiple.append(entry)
        for chunk in chunks:
            by_pack[chunk['pack']].add((chunk['offset'], chunk['size'], chunk['sha256']))
    for digest, ranges in by_pack.items():
        path = packdir / digest
        try:
            data = path.read_bytes()
        except OSError as error:
            raise ValueError(f'missing pack: {path}') from error
        if len(data) != sizes[digest] or not data.startswith(HEADER) or hashlib.sha256(data).hexdigest() != digest:
            raise ValueError(f'corrupt pack: {path}')
        for offset, size, expected in ranges:
            if (offset < len(HEADER) + 4 or size < 0 or offset + size > len(data)
                    or struct.unpack_from('<I', data, offset - 4)[0] != size
                    or hashlib.sha256(data[offset:offset + size]).hexdigest() != expected):
                raise ValueError(f'corrupt pack range: {path}:{offset}+{size}')
    handles = OrderedDict()
    try:
        for entry in multiple:
            digest = hashlib.sha256()
            for chunk in entry['chunks']:
                name = chunk['pack']
                if name in handles:
                    handles.move_to_end(name)
                else:
                    if len(handles) == 64:
                        _, old = handles.popitem(last=False)
                        os.close(old)
                    handles[name] = os.open(packdir / name, os.O_RDONLY)
                data = os.pread(handles[name], chunk['size'], chunk['offset'])
                if len(data) != chunk['size'] or hashlib.sha256(data).hexdigest() != chunk['sha256']:
                    raise ValueError(f"pack range changed for {entry['path']}")
                digest.update(data)
            if digest.hexdigest() != entry['sha256']:
                raise ValueError(f"reconstructed file hash differs: {entry['path']}")
    finally:
        for descriptor in handles.values():
            os.close(descriptor)
    return len(entries), len(sizes)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--v1', type=Path, required=True)
    parser.add_argument('--v2', type=Path, required=True)
    parser.add_argument('--packs', type=Path, required=True)
    parser.add_argument('--published', type=Path)
    args = parser.parse_args()
    v1 = json.loads(args.v1.read_text())
    v2 = json.loads(args.v2.read_text())
    files, packs = verify(v1, v2, args.packs)
    print(f'Verified {files} files and {packs} referenced packs')
    if args.published:
        count, size = additional_pack_bytes(json.loads(args.published.read_text()), v2)
        print(f'Compared with a full current published install: {count} new packs, {size} bytes')


if __name__ == '__main__':
    main()
