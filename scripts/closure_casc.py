#!/usr/bin/env python3
"""Estimate missing-asset local CASC presence; never extract asset payloads.

Reads latest local IDX journals, then decodes only encoding/root metadata in
memory. Layouts follow cascette-rs root/block.rs, encoding/entry.rs and
cascette-client-storage index/{mod,update}.rs. enUS, last root variant wins.
Index presence is not a decryption/extraction guarantee.
"""
import argparse
from collections import Counter
import csv
import hashlib
import json
from pathlib import Path
import struct
import zlib


def take(data, offset, size):
    if offset < 0 or offset + size > len(data):
        raise ValueError(f'truncated metadata at {offset}: need {size}, have {len(data)}')
    return data[offset:offset + size]


def parse_idx(data):
    size = struct.unpack('<I', take(data, 0, 4))[0]
    header = struct.unpack('<H6BQ', take(data, 8, size))
    if header[0] != 7 or header[3:7] != (4, 5, 9, 30):
        raise ValueError(f'unsupported IDX header {header}')
    block = (8 + size + 15) // 16 * 16
    length = struct.unpack('<I', take(data, block, 4))[0]
    entries = take(data, block + 8, length)
    if length % 18:
        raise ValueError('partial IDX sorted entry')
    result = {}

    def insert(entry, status=0):
        key = entry[:9]
        packed = int.from_bytes(entry[9:14], 'big')
        if status in {3, 6, 7}:
            result.pop(key, None)
        else:
            result[key] = (packed >> 30, packed & 0x3fffffff, int.from_bytes(entry[14:18], 'little'))

    for offset in range(0, length, 18):
        insert(entries[offset:offset + 18])
    start = (block + 8 + length + 65535) // 65536 * 65536
    for page in range(start, len(data), 512):
        if not any(take(data, page, 4)):
            break
        for offset in range(page, min(page + 504, len(data)), 24):
            entry = take(data, offset, 24)
            if not any(entry[:4]):
                break
            insert(entry[4:22], entry[22])
    return result


def root_keys(data, wanted):
    version, offset = 1, 0
    if data[:4] in {b'TSFM', b'MFST'}:
        endian = '<' if data[:4] == b'TSFM' else '>'
        size, candidate = struct.unpack(endian + 'II', take(data, 4, 8))
        if 16 <= size < 100 and candidate in {2, 3, 4}:
            version, offset = candidate, max(24, size)
        else:
            version, offset = 2, 12
    result = {}
    while offset < len(data):
        if version == 1:
            count, flags, locale = struct.unpack('<III', take(data, offset, 12))
            offset += 12
        else:
            count, locale = struct.unpack('<II', take(data, offset, 8))
            width = 5 if version == 4 else 4
            flags = int.from_bytes(take(data, offset + 8, width), 'little')
            flags |= int.from_bytes(take(data, offset + 8 + width, 4), 'little')
            flags |= take(data, offset + 12 + width, 1)[0] << 17
            offset += 13 + width
        deltas = take(data, offset, count * 4)
        offset += count * 4
        named = version == 1 or not flags & 0x10000000
        stride = 24 if version == 1 else 16
        keys = take(data, offset, count * stride)
        offset += count * stride
        if named and version != 1:
            take(data, offset, count * 8)
            offset += count * 8
        if not locale & 2:
            continue
        fdid = 0
        for i, (delta,) in enumerate(struct.iter_unpack('<I', deltas)):
            fdid = (fdid + delta) & 0xffffffff
            if fdid in wanted:
                result[fdid] = keys[i * stride:i * stride + 16]
            fdid = (fdid + 1) & 0xffffffff
    return result


def encoding_keys(data, wanted):
    magic, version, ck_size, ek_size, page_kb, _, count, _, _, espec = struct.unpack('>2sBBBHHIIBI', take(data, 0, 22))
    if (magic, version, ck_size, ek_size) != (b'EN', 1, 16, 16):
        raise ValueError('unsupported encoding header')
    index = 22 + espec
    start = index + count * (ck_size + 16)
    page_size = page_kb * 1024
    result = {}
    for i in range(count):
        page = take(data, start + i * page_size, page_size)
        expected = take(data, index + i * 32 + 16, 16)
        if hashlib.md5(page).digest() != expected:
            raise ValueError(f'encoding page {i} checksum mismatch')
        offset = 0
        while offset < page_size and page[offset]:
            keys = page[offset]
            size = int.from_bytes(take(page, offset + 1, 5), 'big')
            content = take(page, offset + 6, ck_size)
            enc = take(page, offset + 6 + ck_size, keys * ek_size)
            if content in wanted:
                result[content] = (size, [enc[j:j + ek_size] for j in range(0, len(enc), ek_size)])
            offset += 6 + ck_size + keys * ek_size
    return result


def decode_blte(data):
    if data[:4] != b'BLTE':
        raise ValueError('no BLTE metadata header')
    header_size = struct.unpack('>I', take(data, 4, 4))[0]

    def decode(block):
        if block[:1] == b'N':
            return block[1:]
        if block[:1] == b'Z':
            return zlib.decompress(block[1:])
        raise ValueError(f'unsupported/encrypted metadata BLTE mode {block[:1]!r}')

    if header_size == 0:
        return decode(take(data, 8, len(data) - 8))
    count = int.from_bytes(take(data, 9, 3), 'big')
    offset = header_size
    output = []
    for i in range(count):
        compressed, plain = struct.unpack('>II', take(data, 12 + 24 * i, 8))
        block = take(data, offset, compressed)
        if hashlib.md5(block).digest() != take(data, 20 + 24 * i, 16):
            raise ValueError('BLTE chunk checksum mismatch')
        decoded = decode(block)
        if len(decoded) != plain:
            raise ValueError('BLTE chunk decoded size mismatch')
        output.append(decoded)
        offset += compressed
    return b''.join(output)


def resolve_missing(wanted, roots, encodings, indices, archives):
    counts = Counter()
    files = []
    local_bytes = 0
    for fdid in sorted(wanted):
        row = {'fdid': fdid}
        content = roots.get(fdid)
        encoded = encodings.get(content)
        if content is None:
            status = 'root_missing'
        elif encoded is None:
            status = 'encoding_missing'
        else:
            size, keys = encoded
            row['content_bytes'] = size
            locations = [(key, indices[key[:9]]) for key in keys if key[:9] in indices]
            status = 'idx_missing' if not locations else 'archive_missing'
            for key, (archive, offset, length) in locations:
                if archive in archives and offset + length <= archives[archive]:
                    status = 'local_index_present'
                    row['encoding_key'] = key.hex()
                    local_bytes += size
                    break
        row['status'] = status
        counts[status] += 1
        files.append(row)
    return {'counts': dict(sorted(counts.items())), 'local_content_bytes': local_bytes, 'files': files}


def load_indices(install):
    selected = {}
    for path in (install / 'Data/data').glob('*.idx'):
        bucket = path.name[:2]
        if bucket not in selected or path.name > selected[bucket].name:
            selected[bucket] = path
    result = {}
    for path in sorted(selected.values()):
        result.update(parse_idx(path.read_bytes()))
    archives = {int(p.suffix[1:]): p.stat().st_size for p in (install / 'Data/data').glob('data.[0-9][0-9][0-9]')}
    return result, archives, [str(p) for p in sorted(selected.values())]


def read_metadata(install, index, key, content_key):
    key = bytes.fromhex(key)
    location = index.get(key[:9])
    if location is None:
        raise ValueError(f'metadata encoding {key.hex()} not in local IDX')
    archive, offset, size = location
    with (install / 'Data/data' / f'data.{archive:03d}').open('rb') as stream:
        stream.seek(offset)
        raw = stream.read(size)
    if len(raw) != size:
        raise ValueError('metadata archive is truncated')
    decoded = decode_blte(raw[30:])
    if hashlib.md5(decoded).hexdigest() != content_key:
        raise ValueError('metadata content key mismatch')
    return decoded


def installed_products(install):
    with (install / '.build.info').open() as stream:
        rows = csv.DictReader(stream, delimiter='|')
        for row in rows:
            fields = {k.split('!')[0]: v for k, v in row.items()}
            if fields['Active'] == '1':
                yield fields


def audit_product(install, product, wanted, index, archives):
    build = product['Build Key']
    path = install / 'Data/config' / build[:2] / build[2:4] / build
    text = path.read_text()
    if hashlib.md5(text.encode()).hexdigest() != build:
        raise ValueError('build config content key mismatch')
    config = dict(line.split(' = ', 1) for line in text.splitlines() if ' = ' in line)
    encoding_ck, encoding_ek = config['encoding'].split()
    encoding = read_metadata(install, index, encoding_ek, encoding_ck)
    root_ck = config['root']
    root_enc = encoding_keys(encoding, {bytes.fromhex(root_ck)}).get(bytes.fromhex(root_ck))
    if not root_enc:
        raise ValueError('root content key not in encoding metadata')
    # Select an explicitly indexed encoding, not the first encoding blindly.
    root_ek = next((key for key in root_enc[1] if key[:9] in index), None)
    if root_ek is None:
        raise ValueError('root metadata encoding not in local IDX')
    root = read_metadata(install, index, root_ek.hex(), root_ck)
    roots = root_keys(root, wanted)
    enc = encoding_keys(encoding, set(roots.values()))
    result = resolve_missing(wanted, roots, enc, index, archives)
    result['metadata'] = {'root_ck': root_ck, 'encoding_ck': encoding_ck, 'build_config': build,
                          'root_bytes': len(root), 'encoding_bytes': len(encoding)}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest', type=Path, required=True)
    parser.add_argument('--install', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text())
    missing = {row['fdid'] for row in manifest['assets'] if not row['present']}
    index, archives, sources = load_indices(args.install)
    products = {}
    for product in installed_products(args.install):
        name = product['Product']
        result = {'version': product['Version'], 'build': product['Build Key']}
        try:
            result.update(audit_product(args.install, product, missing, index, archives))
        except (ValueError, OSError, struct.error, zlib.error) as error:
            result['unresolved_metadata'] = str(error)
            result['counts'] = {'metadata_unresolved': len(missing)}
        products[name] = result
        print(json.dumps({'product': name, 'counts': result['counts']}, sort_keys=True), flush=True)
    output = {'method': 'enUS root last variant -> all encoding alternatives -> latest IDX bucket/update -> archive bounds. Only metadata decoded; no asset payloads read. IDX presence does not prove known TACT keys, complete/authentic assets or runtime compatibility.',
              'missing_unique_fdids': len(missing), 'missing_asset_identities': manifest['summary']['missing'],
              'manifest_sha256': hashlib.sha256(args.manifest.read_bytes()).hexdigest(),
              'tool_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'indices': sources, 'products': products}
    args.output.write_text(json.dumps(output, sort_keys=True, indent=2) + '\n')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
