#!/usr/bin/env python3
"""Retail's M2 effect-name hash: textbook MurmurHash3 x64_128, seed 0.

Retail hashes an effect name such as ``Vertex 52(2), Pixel 34(8)`` and looks it up in an
in-memory registry (`0x140287210` in 12.1.5.70688). Decompilation first reported the second
half's round addend as ``0x0b41def1``; an instruction-level emulator matched on 196 strings with
0 mismatches then showed the image computes ``h2 = (rot + h1 + 0x0b41def1) * 5``, and
``0x0b41def1 * 5 == 0x38495ab5``, so the algorithm is the published one:

    c1 = 0x87c37b91114253d5, c2 = 0x4cf5ad432745937f
    k1 rotr 31, k2 rotr 33, h1 rotr 27, h2 rotr 31
    h1 = h1 * 5 + 0x52dce729, h2 = h2 * 5 + 0x38495ab5
    h1 ^= len, h2 ^= len, h1 += h2, h2 += h1, fmix64 both, return h1 + h2

The hash only indexes that in-memory map: it is never written to an asset, so a bundle's
permutation keys can never match it (see docs/wiki/reference/bls-shaders.md).

Usage:
    python3 scripts/bls_effect_hash.py values
    python3 scripts/bls_effect_hash.py search --keys <dir-with-bundles>
"""

import argparse
import itertools
import struct
import sys

MASK = (1 << 64) - 1
C1 = 0x87C37B91114253D5
C2 = 0x4CF5AD432745937F
# The image's per-round addend, which becomes the published constant once multiplied by five.
ADDEND = 0x38495AB5


def rotl(value, bits):
    return ((value << bits) | (value >> (64 - bits))) & MASK


def fmix64(value):
    value ^= value >> 33
    value = (value * 0xFF51AFD7ED558CCD) & MASK
    value ^= value >> 33
    value = (value * 0xC4CEB9FE1A85EC53) & MASK
    value ^= value >> 33
    return value


def murmur3_x64_128(data, seed=0, addend=ADDEND):
    """MurmurHash3 x64_128 as the client implements it; returns the 64-bit (h1 + h2) sum."""
    h1 = seed & MASK
    h2 = seed & MASK
    blocks = len(data) // 16
    for index in range(blocks):
        k1, k2 = struct.unpack_from("<QQ", data, index * 16)
        k1 = (k1 * C1) & MASK
        k1 = rotl(k1, 31)
        k1 = (k1 * C2) & MASK
        h1 ^= k1
        h1 = rotl(h1, 27)
        h1 = (h1 + h2) & MASK
        h1 = (h1 * 5 + 0x52DCE729) & MASK
        k2 = (k2 * C2) & MASK
        k2 = rotl(k2, 33)
        k2 = (k2 * C1) & MASK
        h2 ^= k2
        h2 = rotl(h2, 31)
        h2 = (h2 + h1) & MASK
        h2 = (h2 * 5 + addend) & MASK
    tail = data[blocks * 16:]
    k1 = 0
    k2 = 0
    for slot in range(len(tail) - 1, 7, -1):
        k2 ^= tail[slot] << ((slot - 8) * 8)
    if tail[8:]:
        k2 = (k2 * C2) & MASK
        k2 = rotl(k2, 33)
        k2 = (k2 * C1) & MASK
        h2 ^= k2
    for slot in range(min(len(tail), 8) - 1, -1, -1):
        k1 ^= tail[slot] << (slot * 8)
    if tail[:8]:
        k1 = (k1 * C1) & MASK
        k1 = rotl(k1, 31)
        k1 = (k1 * C2) & MASK
        h1 ^= k1
    h1 ^= len(data)
    h2 ^= len(data)
    h1 = (h1 + h2) & MASK
    h2 = (h2 + h1) & MASK
    h1 = fmix64(h1)
    h2 = fmix64(h2)
    return (h1 + h2) & MASK


# Decompilation's final values, from the instruction-level emulator (196 strings, 0 mismatches).
KNOWN_VALUES = [
    ("M2Effect 21", 0x022770DBFEB758E6),
    ("Vertex 52(2), Pixel 34(8)", 0xF76E30B48C110473),
    ("Vertex 52(2), Pixel 35(7)", 0xFA5E2515CAFF5512),
]
# Names an effect's shader ids together: the search below hashes every variant of these.
SWAP_NAMES = [
    "M2Effect 21",
    "Vertex 52(2), Pixel 34(8)",
    "Vertex 52(2), Pixel 35(7)",
]
SEEDS = [0, 1, 0x9747B28C, 0xDEADBEEF, 0xFFFFFFFF]


def check_values():
    failures = 0
    for name, full64 in KNOWN_VALUES:
        got = murmur3_x64_128(name.encode())
        ok = got == full64
        failures += 0 if ok else 1
        print(f"{'ok  ' if ok else 'FAIL'} {name!r}: {got:#018x} (want {full64:#018x})")
    return failures


def variants(name):
    """Spellings retail might hash: as authored, lowercased, and NUL-terminated."""
    for text in (name, name.lower()):
        encoded = text.encode()
        yield f"{text!r}", encoded
        yield f"{text!r}+NUL", encoded + b"\0"


def bundle_keys(directory):
    """Every 4- and 8-byte window of every permutation key in the GXSH bundles of `directory`."""
    import glob
    import os

    keys = set()
    for path in sorted(glob.glob(os.path.join(directory, "*.bls"))):
        with open(path, "rb") as handle:
            header = handle.read(0x40)
            if header[:4] != b"HSXG":
                continue
            count = struct.unpack_from("<I", header, 0x10)[0]
            table = struct.unpack_from("<I", header, 0x0C)[0]
            handle.seek(table)
            raw = handle.read(min(count, 8192) * 24)
        for index in range(len(raw) // 24):
            key = raw[index * 24 + 8:index * 24 + 24]
            keys.add((os.path.basename(path), key))
    return keys


def search(directory):
    keys = bundle_keys(directory)
    print(f"{len(keys)} permutation keys in {directory}")
    windows = set()
    for path, key in keys:
        for offset in range(0, 13):
            windows.add((struct.unpack_from("<I", key, offset)[0], 32))
        for offset in range(0, 9):
            windows.add((struct.unpack_from("<Q", key, offset)[0], 64))
    print(f"{len(windows)} distinct 4- and 8-byte windows")
    hits = 0
    for name, text, encoded in itertools.chain.from_iterable(
        variants(name) for name in SWAP_NAMES
    ):
        for seed in SEEDS:
            value = murmur3_x64_128(encoded, seed=seed)
            wanted = {(value & 0xFFFFFFFF, 32), (value, 64)}
            matched = wanted & windows
            if matched:
                hits += 1
                print(f"HIT {name} {text} seed={seed:#x}: {value:#018x}")
    return hits


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["values", "search"])
    parser.add_argument("--keys", default="data/bls-research/scan")
    args = parser.parse_args()
    failures = check_values()
    if args.command == "values":
        sys.exit(1 if failures else 0)
    hits = search(args.keys)
    print(f"{hits} hit(s)")
    sys.exit(0 if hits else 1)


if __name__ == "__main__":
    main()
