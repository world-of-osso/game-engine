#!/usr/bin/env python3
"""Make a private attachment-key fixture over authentic solar spell geometry.

Only attachment tracks change. Source bytes and all referenced assets remain local;
no canonical data or tracked source asset is modified.
"""
import argparse
import struct
from pathlib import Path


def append_array(blob, values, fmt):
    offset = len(blob)
    for value in values:
        blob.extend(struct.pack(fmt, value))
    return len(values), offset


def write_track(blob, block, timelines):
    timestamps, values = [], []
    for times, keys in timelines:
        timestamps.append(append_array(blob, times, "<I"))
        values.append(append_array(blob, keys, "<B"))
    ts_offset = len(blob)
    for header in timestamps:
        blob.extend(struct.pack("<II", *header))
    key_offset = len(blob)
    for header in values:
        blob.extend(struct.pack("<II", *header))
    struct.pack_into("<HhIIII", blob, block, 0, -1, len(timelines), ts_offset,
                     len(timelines), key_offset)


def make_fixture(data, output):
    output.mkdir(parents=True, exist_ok=True)
    models = output / "models"
    models.mkdir(exist_ok=True)
    for name in ("casc", "tactkeys", "textures", "community-listfile.csv", "cache"):
        link = output / name
        if not link.exists():
            link.symlink_to((data / name).resolve())
    raw = (data / "models/1081566.m2").read_bytes()
    result, offset = bytearray(), 0
    while offset + 8 <= len(raw):
        tag = raw[offset:offset + 4]
        size = struct.unpack_from("<I", raw, offset + 4)[0]
        blob = bytearray(raw[offset + 8:offset + 8 + size])
        offset += 8 + size
        if tag == b"MD21":
            count, base = struct.unpack_from("<II", blob, 0xf0)
            assert count >= 3
            write_track(blob, base + 20, [([0, 100, 200], [1, 0, 1]), ([0], [1]), ([], [])])
            write_track(blob, base + 40 + 20, [])
            write_track(blob, base + 80 + 20, [([0], [1])] * 3)
        if tag in (b"SFID", b"SKID", b"AFID"):
            raise_if_missing_references(data, models, tag, blob)
        result.extend(tag + struct.pack("<I", len(blob)) + blob)
    path = models / "1081566.m2"
    path.write_bytes(result)
    print(path)


def raise_if_missing_references(data, models, tag, blob):
    if tag == b"AFID":
        return  # Source sequences are inline (flags161/224); no external keys needed.
    suffix = ".skin" if tag == b"SFID" else ".skel"
    for offset in range(0, len(blob), 4):
        fdid = struct.unpack_from("<I", blob, offset)[0]
        if not fdid:
            continue
        source = data / "models" / f"{fdid}{suffix}"
        if not source.exists():
            raise FileNotFoundError(source)
        link = models / source.name
        if not link.exists():
            link.symlink_to(source.resolve())


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("data", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    make_fixture(args.data, args.output)
