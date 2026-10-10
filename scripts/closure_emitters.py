"""M2 particle/ribbon asset arrays, using the current core parser's offsets.

Source: godot/core/src/asset/m2_format/{m2_particle,m2_ribbon}.rs;
https://wowdev.wiki/M2#Particle_emitters and #Ribbon_emitters.
272/274 use the legacy filename header and 0x1ec stride, not the alternate
Legion header. Recursion filenames resolve as M2 models on the graph work queue.
"""

import struct


def array(data, header, stride, label):
    count, offset = struct.unpack_from("<II", data, header)
    end = offset + count * stride
    if count and end > len(data):
        raise ValueError(f"{label} array outside MD20: {count} at {offset}")
    return range(offset, end, stride) if count else ()


def filename(data, header):
    count, offset = struct.unpack_from("<II", data, header)
    if not count:
        return None
    if offset + count > len(data):
        raise ValueError("emitter filename array outside MD20")
    raw = data[offset : offset + count]
    end = raw.find(b"\0")
    if end < 0:
        raise ValueError("unterminated emitter filename")
    name = raw[:end].decode("utf-8").strip()
    return name or None


def expand_emitters(graph, fdid, data, texture_fdids):
    # All named/TXID texture slots are already enumerated by expand_model.
    texture_count, _ = struct.unpack_from("<II", data, 0x50)
    version = struct.unpack_from("<I", data, 4)[0]
    stride = 0x1EC if version >= 272 else 0x178
    for number, base in enumerate(array(data, 0x128, stride, "particle")):
        for header, label in [(0x18, "geometry"), (0x20, "recursive model")]:
            name = filename(data, base + header)
            if name:
                graph.named(name, "m2", f"M2 particle {number} {label}", fdid)
        packed = struct.unpack_from("<H", data, base + 0x16)[0]
        flags = struct.unpack_from("<I", data, base + 4)[0]
        indices = (
            [(packed >> (5 * i)) & 31 for i in range(3)]
            if flags & 0x10000000
            else [packed]
        )
        for index in indices:
            bind_texture(
                graph, fdid, texture_fdids, texture_count, index, f"particle {number}"
            )
        graph.resolve(
            "emitter_auxiliary_edges",
            fdid,
            "resolved",
            f"particle {number}: core filename arrays, {stride:#x} stride, texture indices {indices}",
        )
    for number, base in enumerate(array(data, 0x120, 176, "ribbon")):
        for at in array(data, base + 0x14, 2, "ribbon texture indices"):
            index = struct.unpack_from("<H", data, at)[0]
            bind_texture(
                graph, fdid, texture_fdids, texture_count, index, f"ribbon {number}"
            )
        graph.resolve(
            "emitter_auxiliary_edges",
            fdid,
            "resolved",
            f"ribbon {number}: 176-byte core layout, u16 indices into already enumerated M2 textures; materials carry no file identity",
        )


def bind_texture(graph, fdid, texture_fdids, texture_count, index, label):
    if texture_fdids is not None and index < len(texture_fdids):
        graph.add(
            texture_fdids[index], "blp", f"M2 {label} texture index {index}", fdid
        )
    elif index >= texture_count:
        graph.resolve(
            "emitter_texture",
            fdid,
            "not_needed",
            f"{label} index {index} outside {texture_count} textures: native lookup returns None before texture IO",
        )
    else:
        graph.resolve(
            "emitter_texture",
            fdid,
            "resolved",
            f"{label} index {index}: named or replaceable model slot already traversed/context-seeded; no alternative FDID guessed",
        )
