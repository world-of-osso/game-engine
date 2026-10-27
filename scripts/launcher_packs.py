#!/usr/bin/env python3
"""Build immutable launcher packs and a persistent chunk index from a staged bundle."""
import argparse
import hashlib
import json
import struct
import subprocess
from pathlib import Path

MIB = 1024 * 1024
TARGET = 32 * MIB
MIN_CHUNK = 256 * 1024
MAX_CHUNK = 4 * MIB
HEADER = b'OSSPACK\x01'
CHUNKER = Path(__file__).resolve().parents[1] / 'target/release/launcher-pack-chunks'


def build_packs(source, output, input_manifest=None, source_map=None):
    source, output = Path(source), Path(output)
    packdir = output / 'packs'
    packdir.mkdir(parents=True, exist_ok=True)
    index_path = output / 'chunk-index.json'
    index = json.loads(index_path.read_text()) if index_path.exists() else {'version': 1, 'chunks': {}}
    if index['version'] != 1:
        raise ValueError('unsupported chunk index version')
    existing = index['chunks']
    entries, pending = [], {}
    if input_manifest is None:
        paths = [(p.relative_to(source).as_posix(), None) for p in source.rglob('*') if p.is_file()]
    else:
        paths = [(item['path'], item) for item in input_manifest['files']]
    if len({name for name, _ in paths}) != len(paths):
        raise ValueError('duplicate staged path')
    paths.sort()
    for relative, _ in paths:
        if any(part in ('', '.', '..') for part in relative.split('/')) or '\\' in relative or ':' in relative:
            raise ValueError(f'unsafe staged path: {relative}')
    input_paths = [str(Path(source_map[name]).resolve()) if source_map is not None else name
                   for name, _ in paths]
    result = subprocess.run([str(CHUNKER), str(source)], input=json.dumps(input_paths),
                            text=True, stdout=subprocess.PIPE, check=True)
    indexed = json.loads(result.stdout)
    for (relative, expected), file in zip(paths, indexed, strict=True):
        path = Path(source_map[relative]) if source_map is not None else source / relative
        parts = relative.split('/')
        group = '/'.join(parts[:2]) if parts[0] == 'data' and len(parts) > 2 else (parts[0] if len(parts) > 1 else '_root')
        refs = []
        offset = 0
        for chunk in file['chunks']:
            length = chunk['size']
            key = f"{group}:{chunk['sha256']}"
            refs.append(key)
            if key not in existing and key not in pending:
                pending[key] = (path, offset, length)
            offset += length
        size, digest = file['size'], file['sha256']
        if expected is not None and (size != expected['size'] or digest != expected['sha256']):
            raise ValueError(f'staged file does not match manifest: {relative}')
        entries.append((relative, size, digest, refs))

    def flush(records):
        data = bytearray(HEADER)
        locations = []
        for key, chunk in records:
            data.extend(struct.pack('<I', len(chunk)))
            offset = len(data)
            data.extend(chunk)
            locations.append((key, offset, len(chunk)))
        digest = hashlib.sha256(data).hexdigest()
        path = packdir / digest
        if path.exists() and hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise ValueError(f'corrupt existing pack: {path}')
        if not path.exists():
            temporary = path.with_suffix('.part')
            temporary.write_bytes(data)
            temporary.replace(path)
        for key, offset, size in locations:
            existing[key] = {'sha256': key.rsplit(':', 1)[1], 'size': size, 'pack': digest, 'offset': offset, 'pack_size': len(data)}

    records, amount, previous_group = [], len(HEADER), None
    for key, (path, offset, length) in sorted(pending.items()):
        with path.open('rb') as source_file:
            source_file.seek(offset)
            chunk = source_file.read(length)
        if len(chunk) != length or hashlib.sha256(chunk).hexdigest() != key.rsplit(':', 1)[1]:
            raise ValueError(f'staged file changed: {path}')
        group = key.rsplit(':', 1)[0]
        if records and (group != previous_group or amount + 4 + len(chunk) > TARGET):
            flush(records)
            records, amount = [], len(HEADER)
        records.append((key, chunk))
        amount += 4 + len(chunk)
        previous_group = group
    if records:
        flush(records)
    referenced = {key for _, _, _, refs in entries for key in refs}
    by_pack = {}
    for key in referenced:
        by_pack.setdefault(existing[key]['pack'], []).append(key)
    for digest, keys in by_pack.items():
        path = packdir / digest
        try:
            data = path.read_bytes()
        except OSError as error:
            raise ValueError(f'missing pack: {path}') from error
        if not data.startswith(HEADER) or hashlib.sha256(data).hexdigest() != digest:
            raise ValueError(f'corrupt pack: {path}')
        for key in keys:
            record = existing[key]
            offset, size = record['offset'], record['size']
            chunk = data[offset:offset + size]
            if (record['pack_size'] != len(data) or offset < len(HEADER) + 4
                    or len(chunk) != size or hashlib.sha256(chunk).hexdigest() != record['sha256']
                    or record['sha256'] != key.rsplit(':', 1)[1]):
                raise ValueError(f'corrupt chunk index: {key}')
    manifest = {'version': 2, 'format_version': 1, 'chunking': {'version': 1, 'min': MIN_CHUNK, 'average': MIB, 'max': MAX_CHUNK},
                'files': [{'path': path, 'size': size, 'sha256': digest, 'chunks': [existing[key] for key in refs]} for path, size, digest, refs in entries]}
    for path, value in ((index_path, index), (output / 'manifest-v2.json', manifest)):
        temporary = path.with_suffix(path.suffix + '.part')
        temporary.write_text(json.dumps(value, sort_keys=True, separators=(',', ':')))
        temporary.replace(path)
    return manifest


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--manifest', type=Path, required=True)
    args = parser.parse_args()
    build_packs(args.source, args.output, json.loads(args.manifest.read_text()))


if __name__ == '__main__':
    main()
