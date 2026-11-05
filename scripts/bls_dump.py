#!/usr/bin/env python3
"""Offline GXSH 0x1000e reader. See docs/wiki/reference/bls-shaders.md.

Uses only Python's standard library; SM5 assembly uses scripts/bls-disasm.
Never loads shader bundles into the engine or contacts the Blizzard CDN.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import zlib

ROOT = Path(__file__).resolve().parents[1]
BLOCK_SIZE = 16384


def require(condition, message):
    if not condition:
        raise ValueError(message)


def u32(data, offset):
    require(0 <= offset <= len(data) - 4, f"truncated u32 at {offset:#x}")
    return struct.unpack_from("<I", data, offset)[0]


def slice_bytes(data, offset, size):
    require(0 <= offset <= len(data) and 0 <= size <= len(data) - offset,
            f"range {offset:#x}+{size:#x} exceeds {len(data):#x}")
    return data[offset:offset + size]


def parse_bundle(data):
    require(data[:4] == b"HSXG", "not an on-disk GXSH bundle (HSXG)")
    require(u32(data, 4) == 0x1000e, "unsupported GXSH version")
    table, count, block_table, blocks, body, word32, word36 = struct.unpack(
        "<7I", slice_bytes(data, 12, 28))
    require(table == 40, "unsupported header size/permutation-table offset")
    require(block_table == table + count * 24 + 4, "invalid permutation table extent")
    require(body == block_table + (blocks + 1) * 4, "invalid block table extent")
    require(blocks > 0, "empty compressed block table")
    entries = []
    for index in range(count):
        entry = slice_bytes(data, table + index * 24, 24)
        offset, size = struct.unpack_from("<2I", entry)
        entries.append(dict(index=index, offset=offset, size=size,
                            opaque_key=entry[8:].hex()))
    raw_size = u32(data, block_table - 4)
    offsets = [u32(data, block_table + i * 4) for i in range(blocks + 1)]
    require(offsets[0] == 0 and body + offsets[-1] == len(data), "invalid body extent")
    inflated = inflate_blocks(data, body, offsets, raw_size)
    end = 0
    for entry in entries:
        require(entry['offset'] == end, "non-contiguous permutation records")
        end += entry['size']
    require(end == raw_size, "permutation sizes do not cover inflated body")
    header = dict(version=0x1000e, profile=data[8:12][::-1].decode('ascii'),
                  permutation_table_offset=table, permutation_count=count,
                  block_table_offset=block_table, block_count=blocks,
                  body_offset=body, unknown_32=word32, unknown_36=word36,
                  inflated_size=raw_size, compressed_block_offsets=offsets)
    return header, entries, inflated


def inflate_blocks(data, body, offsets, raw_size):
    require(len(offsets) - 1 == (raw_size + BLOCK_SIZE - 1) // BLOCK_SIZE,
            "block count differs from inflated size")
    parts = []
    for index, (start, end) in enumerate(zip(offsets, offsets[1:])):
        require(end > start, "unordered compressed block offsets")
        decoder = zlib.decompressobj()
        block = decoder.decompress(slice_bytes(data, body + start, end - start), BLOCK_SIZE + 1)
        require(decoder.eof and not decoder.unused_data and not decoder.unconsumed_tail,
                f"incomplete/overlong zlib block {index}")
        expected = min(BLOCK_SIZE, raw_size - index * BLOCK_SIZE)
        require(len(block) == expected, f"wrong inflated length in block {index}")
        parts.append(block)
    return b''.join(parts)


def split_program(record):
    # The retail record has an opaque prefix, not a bare DXBC container.
    offset = record.find(b'DXBC')
    require(offset >= 0, "permutation has no DXBC container")
    size = u32(record, offset + 24)
    program = slice_bytes(record, offset, size)
    require(program[:4] == b'DXBC' and size >= 32, "invalid DXBC extent")
    require(record.find(b'DXBC', offset + size) < 0, "multiple containers in permutation")
    return program, dict(dxbc_offset=offset, prefix_hex=record[:offset].hex(),
                         suffix_hex=record[offset + size:].hex())


def parse_chunks(program):
    require(u32(program, 24) == len(program), "DXBC size differs from record")
    count = u32(program, 28)
    table_end = 32 + 4 * count
    slice_bytes(program, 32, 4 * count)
    chunks = {}
    ranges = []
    for index in range(count):
        offset = u32(program, 32 + 4 * index)
        require(offset >= table_end, "chunk overlaps DXBC directory")
        tag = slice_bytes(program, offset, 4).decode('ascii')
        size = u32(program, offset + 4)
        require(tag not in chunks, f"duplicate chunk {tag}")
        chunks[tag] = slice_bytes(program, offset + 8, size)
        ranges.append((offset, offset + 8 + size))
    ranges.sort()
    require(all(a[1] <= b[0] for a, b in zip(ranges, ranges[1:])), "overlapping chunks")
    return chunks


def parse_signature(data, tag):
    count = u32(data, 0)
    extended = tag.endswith('1')
    stride = 32 if extended else 24
    result = []
    slice_bytes(data, 8, stride * count)
    for index in range(count):
        offset = 8 + stride * index
        stream = u32(data, offset) if extended else 0
        base = offset + (4 if extended else 0)
        name_offset, semantic_index, system, component, register = struct.unpack_from('<5I', data, base)
        require(name_offset < len(data), "signature name outside chunk")
        end = data.find(b'\0', name_offset)
        require(end >= name_offset, "unterminated semantic name")
        result.append(dict(semantic=data[name_offset:end].decode('ascii'),
                           index=semantic_index, system_value=system,
                           component_type=component, register=register, stream=stream,
                           mask=data[base + 20], read_write_mask=data[base + 21],
                           minimum_precision=u32(data, offset + 28) if extended else None))
    return result


def shader_declarations(data):
    require(len(data) % 4 == 0, "unaligned shader tokens")
    words = struct.unpack('<' + str(len(data) // 4) + 'I', data)
    require(len(words) >= 2 and words[1] == len(words), "invalid shader token count")
    slots = {'textures': [], 'samplers': [], 'constant_buffers': []}
    opcodes = set()
    offset = 2
    while offset < len(words):
        token = words[offset]
        opcode = token & 0x7ff
        length = words[offset + 1] if opcode == 53 else (token >> 24) & 0x7f
        require(length > 0 and offset + length <= len(words), f"invalid instruction at {offset}")
        opcodes.add(opcode)
        if opcode in (88, 89, 90):
            # dcl_resource/cb/sampler: one immediate register index, cb also has vector count.
            require(length >= 3 and not token >> 31, "unsupported extended declaration")
            operand = words[offset + 1]
            expected_type = {88: 7, 89: 8, 90: 6}[opcode]
            dimensions = 2 if opcode == 89 else 1
            require((operand >> 12) & 0xff == expected_type, "unexpected declaration operand")
            require((operand >> 20) & 3 == dimensions and operand >> 22 == 0,
                    "unsupported indexed declaration")
            key = {88: 'textures', 89: 'constant_buffers', 90: 'samplers'}[opcode]
            slot = words[offset + 2]
            slots[key].append(dict(slot=slot, vectors=words[offset + 3]) if opcode == 89 else slot)
        offset += length
    slots['textures'].sort()
    slots['samplers'].sort()
    slots['constant_buffers'].sort(key=lambda item: item['slot'])
    return slots, sorted(opcodes)


def describe_program(program):
    chunks = parse_chunks(program)
    result = dict(chunks=list(chunks), inputs=[], outputs=[], kind='DXIL' if 'DXIL' in chunks else 'SM5')
    for tag, data in chunks.items():
        if tag in ('ISGN', 'ISG1'):
            result['inputs'] = parse_signature(data, tag)
        elif tag in ('OSGN', 'OSG1', 'OSG5'):
            require(tag != 'OSG5', 'OSG5 signatures are not supported')
            result['outputs'] = parse_signature(data, tag)
    shader = chunks.get('SHEX', chunks.get('SHDR'))
    if shader is not None:
        result['slots'], result['opcodes'] = shader_declarations(shader)
    else:
        require('DXIL' in chunks, 'no SHEX/SHDR/DXIL program')
        result['slots'] = None  # DXIL resource reflection is deliberately not inferred.
    return result, shader


def read_source(source, cache, casc_local):
    if not source.isdecimal():
        return Path(source)
    cache.mkdir(parents=True, exist_ok=True)
    path = cache / (source + '.bls')
    if not path.exists():
        require(casc_local is not None, 'FDID not cached; provide --casc-local <binary> for local CASC extraction')
        subprocess.run([casc_local, source, '-o', str(cache)], check=True)
    require(path.is_file(), f'CASC extraction did not produce {path}')
    return path


def write_dump(path, output, disassembler):
    data = path.read_bytes()
    header, entries, raw = parse_bundle(data)
    output.mkdir(parents=True, exist_ok=True)
    programs = []
    shex_paths = []
    for entry in entries:
        record = slice_bytes(raw, entry['offset'], entry['size'])
        program, wrapper = split_program(record)
        description, shader = describe_program(program)
        stem = output / f"permutation-{entry['index']:04d}"
        stem.with_suffix('.dxbc').write_bytes(program)
        if shader is not None:
            shex_path = stem.with_suffix('.shex')
            shex_path.write_bytes(shader)
            shex_paths.append(str(shex_path))
        programs.append(dict(**entry, **wrapper, **description,
                             sha256=hashlib.sha256(program).hexdigest()))
    if shex_paths:
        require(disassembler.is_file(), f'build disassembler first; missing {disassembler}')
        subprocess.run([str(disassembler), *shex_paths], check=True)
    report = dict(source=str(path), sha256=hashlib.sha256(data).hexdigest(), header=header, programs=programs)
    (output / 'manifest.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', help='local .bls path or FileDataID')
    parser.add_argument('-o', '--output', type=Path, required=True)
    parser.add_argument('--cache', type=Path, default=ROOT / 'data/bls-research')
    parser.add_argument('--casc-local', default=os.environ.get('CASC_LOCAL'))
    parser.add_argument('--disassembler', type=Path,
                        default=ROOT / 'target/bls-disasm/debug/bls-disasm')
    args = parser.parse_args()
    try:
        path = read_source(args.source, args.cache, args.casc_local)
        report = write_dump(path, args.output, args.disassembler)
        print(json.dumps(dict(source=str(path), header=report['header'],
                              programs=[dict(index=p['index'], slots=p['slots']) for p in report['programs']]), indent=2))
    except (ValueError, OSError, zlib.error, struct.error, subprocess.CalledProcessError) as error:
        parser.exit(1, f'bls_dump: {error}\n')


if __name__ == '__main__':
    main()
