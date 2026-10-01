"""Read-only, independent cached MD21/SKIN batch-zero translation evidence.

Usage: python3 godot/tests/skybox_track_reference.py <cached-skybox.m2>
No engine imports, extraction, renderer, native clock calls or generated expectations.
"""

import hashlib
import json
import struct
import sys
from pathlib import Path


def read_array(data, offset, format_code):
    count, start = struct.unpack_from("<II", data, offset)
    width = struct.calcsize("<" + format_code)
    if start + count * width > len(data):
        raise ValueError(f"Array at {offset:#x} exceeds cached file extent")
    return [struct.unpack_from("<" + format_code, data, start + i * width) for i in range(count)]


def read_md21(raw):
    offset = 0
    while offset + 8 <= len(raw):
        size = struct.unpack_from("<I", raw, offset + 4)[0]
        end = offset + 8 + size
        if end > len(raw):
            raise ValueError("Truncated M2 chunk")
        if raw[offset : offset + 4] == b"MD21":
            md20 = raw[offset + 8 : end]
            if md20[:4] != b"MD20":
                raise ValueError("MD21 does not contain an MD20 header")
            return md20
        offset = end
    raise ValueError("Cached model has no MD21 chunk")


def read_track_reference(model_path):
    raw = model_path.read_bytes()
    md20 = read_md21(raw)
    skin_path = model_path.with_name(model_path.stem + "00.skin")
    skin = skin_path.read_bytes()
    if skin[:4] != b"SKIN":
        raise ValueError("Cached primary skin has no SKIN header")
    units = read_array(skin, 0x24, "Bb11H")
    if not units:
        raise ValueError("Cached primary skin has no material units")
    lookup = read_array(md20, 0x98, "h")
    lookup_index = units[0][-1]
    track_index = lookup[lookup_index][0]
    count, offset = struct.unpack_from("<II", md20, 0x60)
    if not 0 <= track_index < count:
        raise ValueError("Batch-zero first-stage UV lookup has no translation track")
    block = offset + track_index * 60
    interpolation, global_index = struct.unpack_from("<Hh", md20, block)
    timestamps = read_array(md20, block + 4, "II")
    values = read_array(md20, block + 12, "II")
    if len(timestamps) != 1 or len(values) != 1:
        raise ValueError("Bounded oracle requires one embedded track sequence")
    times = [value[0] for value in read_array(md20, struct.unpack_from("<I", md20, block + 8)[0], "I")]
    keys = read_array(md20, struct.unpack_from("<I", md20, block + 16)[0], "3f")
    durations = [value[0] for value in read_array(md20, 0x14, "I")]
    if len(times) != len(keys) or not times:
        raise ValueError("Translation timestamp/key count differs or is empty")
    if not 0 <= global_index < len(durations):
        raise ValueError("Bounded oracle requires an authored global sequence")
    return {
        "model": str(model_path),
        "model_sha256": hashlib.sha256(raw).hexdigest(),
        "skin_sha256": hashlib.sha256(skin).hexdigest(),
        "source_unit": 0,
        "shader_id": hex(units[0][2]),
        "first_stage_uv_lookup": lookup_index,
        "translation_track": track_index,
        "interpolation": interpolation,
        "global_sequence": global_index,
        "global_duration_ms": durations[global_index],
        "timestamps_ms": times,
        "translation_keys": keys,
        "limit": "Source evidence only, not runtime or rendered animation proof",
    }


def main():
    if len(sys.argv) != 2:
        raise ValueError("Requires exactly one cached skybox M2 path")
    result = read_track_reference(Path(sys.argv[1]))
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, IndexError, struct.error) as error:
        print(f"SKYBOX TRACK REFERENCE: {error}", file=sys.stderr)
        sys.exit(1)
