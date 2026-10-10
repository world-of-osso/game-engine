"""WMO MLIQ joins matching core/wmo_liquid.rs and the native liquid loader.

Sources: core/asset/wmo_format/{parser,parser_types}.rs; wowdev.wiki/WMO
MOHD/MOGP/MLIQ; WebWowViewerCpp WmoGroupObject::setLiquidType cited by core.
MLIQ's material_id is a MOMT index, NOT a LiquidMaterial/Type identity.
"""

import struct


def inherit_root(graph, child, flags):
    if not child:
        return
    contexts = graph.wmo_root_flags.setdefault(child, set())
    flags &= 4
    if flags in contexts:
        return
    contexts.add(flags)
    key = (child, "wmo")
    if key in graph.assets and "present" in graph.assets[key]:
        graph.queue.append(key)


def liquid_type(root_flags, group_flags, liquid):
    if root_flags & 4:
        value = (
            basic_liquid((liquid - 1) & 0xFFFFFFFF, group_flags)
            if liquid < 21
            else liquid
        )
    elif liquid == 15:
        return None
    elif liquid < 20:
        value = basic_liquid(liquid, group_flags)
    else:
        value = liquid + 1
    return value if 0 < value <= 0xFFFF else None


def basic_liquid(value, flags):
    basic = value & 3
    if basic == 0:
        return 14 if flags & 0x80000 else 13
    return {1: 14, 2: 19, 3: 20}[basic]


def expand_liquid(graph, fdid, header, payload):
    if len(payload) < 30:
        raise ValueError("MLIQ header requires 30 bytes")
    xv, yv, xt, yt = struct.unpack_from("<4i", payload)
    if min(xv, yv, xt, yt) < 0 or 30 + xv * yv * 8 + xt * yt > len(payload):
        raise ValueError("MLIQ vertex/tile array out of bounds")
    missing = (
        "wmo_liquid_edges",
        "MLIQ needs owning MOHD root flags; no liquid identity guessed",
        fdid,
    )
    contexts = graph.wmo_root_flags.get(fdid)
    if not contexts:
        graph.unresolved.add(missing)
        return
    graph.unresolved.discard(missing)
    group_flags, authored = (
        struct.unpack_from("<I", header, 8)[0],
        struct.unpack_from("<I", header, 52)[0],
    )
    if graph.terrain is None:
        from closure_terrain import TerrainReferences

        graph.terrain = TerrainReferences(graph)
    for flags in sorted(contexts):
        identity = liquid_type(flags, group_flags, authored)
        if identity is None:
            graph.resolve(
                "wmo_liquid_edges",
                fdid,
                "not_needed",
                f"MOGP liquid={authored} root_flags={flags}: core group_liquid_type returns None before native material IO",
            )
            continue
        graph.terrain.liquid(
            identity, 0, fdid, source="MLIQ", issue_code="wmo_liquid_edges"
        )
        graph.resolve(
            "wmo_liquid_edges",
            fdid,
            "resolved",
            f"MOGP liquid={authored} root_flags={flags} group_flags={group_flags} -> LiquidType {identity}; native material request precedes tile visibility",
        )
