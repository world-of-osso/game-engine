#!/usr/bin/env python3
"""Provision Zephras' runtime closure from local wow_classic_beta CASC, never CDN.

Cache destinations mirror Godot: terrain/<FDID>.{wdt,adt}, models/<FDID>.{m2,wmo,anim},
models/<model>00.skin and <model>.skel, textures/<FDID>.blp. MAIN selects MAID root,
obj0 and tex0; obj1/LOD/map textures are not consumed by the runtime. Retail map
identity wins; this tool reads Forever's explicit build export for map 2991.
"""

import argparse
import csv
import json
import os
import shutil
import struct
import subprocess
from pathlib import Path

CASC_LOCAL = Path(
    "/syncthing/Sync/Projects/world-of-osso/asset-resolver/target/debug/casc-local"
)
BUILD = "1.60.1.70205"


def chunks(data):
    offset = 0
    while offset < len(data):
        if offset + 8 > len(data):
            raise ValueError(f"truncated chunk header at {offset}")
        tag, size = struct.unpack_from("<4sI", data, offset)
        end = offset + 8 + size
        if end > len(data):
            raise ValueError(f"truncated {tag!r} chunk at {offset}")
        yield tag, data[offset + 8 : end]
        offset = end


def u32s(data):
    if len(data) % 4:
        raise ValueError("unaligned u32 array")
    return [v[0] for v in struct.iter_unpack("<I", data)]


def terrain_references(data):
    parts = dict(chunks(data))
    main, maid = parts[b"NIAM"], parts[b"DIAM"]
    if len(main) != 4096 * 8 or len(maid) != 4096 * 32:
        raise ValueError("WDT MAIN/MAID must contain 4096 slots")
    refs = []
    for index in range(4096):
        if not struct.unpack_from("<I", main, index * 8)[0] & 1:
            continue
        ids = struct.unpack_from("<8I", maid, index * 32)
        for fdid in (ids[0], ids[1], ids[3]):
            if fdid:
                refs.append((fdid, f"terrain/{fdid}.adt"))
    return refs


def string_at(data, offset):
    if offset >= len(data):
        raise ValueError(f"string offset {offset} outside {len(data)} bytes")
    return data[offset:].split(b"\0", 1)[0].decode()


def resolve_path(path, paths):
    normalized = path.replace("\\", "/").lower()
    if normalized.endswith((".mdx", ".mdl")):
        normalized = normalized[:-4] + ".m2"
    if normalized not in paths:
        raise ValueError(f"asset path not in local listfile: {path}")
    return paths[normalized]


def placement_references(
    parts, tag, size, flag_offset, fdid_flag, names, offsets, ext, paths
):
    refs = []
    payload = parts.get(tag, b"")
    if len(payload) % size:
        raise ValueError(f"{tag!r} records truncated")
    table = u32s(parts.get(offsets, b""))
    for offset in range(0, len(payload), size):
        name = struct.unpack_from("<I", payload, offset)[0]
        flags = struct.unpack_from("<H", payload, offset + flag_offset)[0]
        fdid = (
            name
            if flags & fdid_flag
            else resolve_path(string_at(parts[names], table[name]), paths)
        )
        refs.append((fdid, f"models/{fdid}.{ext}"))
    return refs


def model_references(parts, fdid, destination, paths):
    refs = []
    for skin in u32s(parts.get(b"SFID", b""))[:1]:
        refs.append((skin, f"models/{fdid}00.skin"))
    for texture in u32s(parts.get(b"TXID", b"")):
        if texture:
            refs.append((texture, f"textures/{texture}.blp"))
    for _, _, animation in struct.iter_unpack("<HHI", parts.get(b"AFID", b"")):
        if animation:
            refs.append((animation, f"models/{animation}.anim"))
    for skeleton in u32s(parts.get(b"SKID", b"")):
        if skeleton:
            refs.append((skeleton, f"models/{fdid}.skel"))
    # Legacy type-0 textures store a path in the MD20 header texture array.
    md20 = parts.get(b"MD21")
    if md20 and len(md20) >= 88:
        count, offset = struct.unpack_from("<II", md20, 80)
        txids = u32s(parts.get(b"TXID", b""))
        for index in range(count):
            kind, _, length, name = struct.unpack_from("<4I", md20, offset + index * 16)
            if kind == 0 and length and (index >= len(txids) or txids[index] == 0):
                texture = resolve_path(string_at(md20, name), paths)
                refs.append((texture, f"textures/{texture}.blp"))
    return refs


def wmo_references(parts, fdid, names, paths):
    refs = []
    groups = struct.unpack_from("<I", parts[b"DHOM"], 4)[0]
    group_ids = u32s(parts.get(b"DIFG", b""))[:groups]
    if len(group_ids) < groups:
        base = names[fdid][:-4]
        group_ids = [
            resolve_path(f"{base}_{index:03}.wmo", paths) for index in range(groups)
        ]
    refs.extend((group, f"models/{group}.wmo") for group in group_ids)
    materials = parts.get(b"TMOM", b"")
    for offset in range(0, len(materials), 64):
        shader = struct.unpack_from("<I", materials, offset + 4)[0]
        fields = [12, 24, 36] + (
            [40, 44, 48]
            if shader == 19
            else list(range(40, 64, 4))
            if shader == 20
            else []
        )
        for field in fields:
            texture = struct.unpack_from("<I", materials, offset + field)[0]
            if texture:
                refs.append((texture, f"textures/{texture}.blp"))
    models = u32s(parts.get(b"IDOM", b""))
    refs.extend((model, f"models/{model}.m2") for model in models if model)
    if not models:
        for record in range(0, len(parts.get(b"DDOM", b"")), 40):
            name = struct.unpack_from("<I", parts[b"DDOM"], record)[0] & 0xFFFFFF
            model = resolve_path(string_at(parts[b"NDOM"], name), paths)
            refs.append((model, f"models/{model}.m2"))
    return refs


def asset_references(data, fdid, destination, names, paths):
    if destination.endswith((".blp", ".skin", ".anim")):
        return []
    parts = dict(chunks(data))
    if destination.endswith(".wdt"):
        return terrain_references(data)
    if destination.endswith(".adt"):
        refs = [
            (texture, f"textures/{texture}.blp")
            for tag in (b"DIDM", b"DIHM")
            for texture in u32s(parts.get(tag, b""))
            if texture
        ]
        refs += placement_references(
            parts, b"FDDM", 36, 34, 0x40, b"XDMM", b"DIMM", "m2", paths
        )
        refs += placement_references(
            parts, b"FDOM", 64, 56, 0x8, b"OMWM", b"DIWM", "wmo", paths
        )
        return refs
    if destination.endswith((".m2", ".skel")):
        return model_references(parts, fdid, destination, paths)
    if destination.endswith(".wmo") and b"DHOM" in parts:
        return wmo_references(parts, fdid, names, paths)
    return []


def validate_asset(data, destination):
    ext = Path(destination).suffix
    allowed = {
        ".m2": (b"MD21",),
        ".skin": (b"SKIN",),
        ".blp": (b"BLP2", b"BLP1"),
        ".wdt": (b"REVM",),
        ".adt": (b"REVM",),
        ".wmo": (b"REVM",),
        ".skel": (b"SKL1", b"SKS1"),
    }
    if not data or (ext in allowed and data[:4] not in allowed[ext]):
        raise ValueError(f"{destination}: unexpected magic {data[:4]!r}")
    if ext in (".wdt", ".adt", ".wmo"):
        parts = dict(chunks(data))
        required = {
            ".wdt": b"DIAM",
            ".adt": b"REVM",
            ".wmo": b"DHOM" if b"DHOM" in parts else b"PGOM",
        }[ext]
        if required not in parts:
            raise ValueError(f"{destination}: missing {required!r}")


def read_listfile(data):
    names, paths = {}, {}
    with (data / "community-listfile.csv").open() as handle:
        for line in handle:
            number, name = line.rstrip().split(";", 1)
            fdid = int(number)
            names[fdid] = name
            paths[name.replace("\\", "/").lower()] = fdid
    return names, paths


def import_closure(data):
    with (data / "db2" / BUILD / "Map.csv").open(newline="") as handle:
        row = next(row for row in csv.DictReader(handle) if row["ID"] == "2991")
    wdt = int(row["WdtFileDataID"])
    names, paths = read_listfile(data)
    staging = data / "cache/forever-zephras-extract"
    staging.mkdir(parents=True, exist_ok=True)
    pending = {(wdt, f"terrain/{wdt}.wdt")}
    seen, files, failures = set(), [], []
    extracted = set()
    while pending:
        batch = sorted(pending - seen)
        pending.clear()
        if not batch:
            break
        missing = sorted(
            {
                fdid
                for fdid, dest in batch
                if not (data / dest).is_file()
                and not list(staging.glob(f"{fdid}.*"))
                and fdid not in extracted
            }
        )
        for start in range(0, len(missing), 100):
            ids = missing[start : start + 100]
            result = subprocess.run(
                [str(CASC_LOCAL), *map(str, ids), "-o", str(staging)],
                env=dict(os.environ, WOW_PRODUCT="wow_classic_beta"),
                check=False,
            )
            extracted.update(ids)
            if result.returncode:
                print(
                    f"casc-local exit {result.returncode}; inspecting requested files",
                    flush=True,
                )
        for fdid, destination in batch:
            seen.add((fdid, destination))
            try:
                target = data / destination
                candidates = sorted(staging.glob(f"{fdid}.*"))
                source = (
                    target
                    if target.is_file()
                    else candidates[0]
                    if len(candidates) == 1
                    else None
                )
                if source is None:
                    raise ValueError(
                        f"FDID {fdid}: no unique extracted file for {destination}"
                    )
                raw = source.read_bytes()
                validate_asset(raw, destination)
                refs = asset_references(raw, fdid, destination, names, paths)
                if source != target:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(source, target)
                files.append({"fdid": fdid, "path": destination, "bytes": len(raw)})
                pending.update(refs)
            except (ValueError, KeyError, IndexError, OSError, struct.error) as error:
                failures.append(
                    {"fdid": fdid, "path": destination, "error": str(error)}
                )
                print(f"FAIL {fdid} {destination}: {error}", flush=True)
        print(
            f"closure: {len(files)} files, {sum(file['bytes'] for file in files)} bytes, {len(failures)} failures, {len(pending - seen)} pending",
            flush=True,
        )
    report = {
        "map": 2991,
        "product": "wow_classic_beta",
        "build": BUILD,
        "files": files,
        "bytes": sum(file["bytes"] for file in files),
        "failures": failures,
    }
    report_path = data / f"forever-{BUILD}/metadata/Zephras-closure.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--data", type=Path, default=Path(__file__).resolve().parents[1] / "data"
    )
    args = parser.parse_args()
    report = import_closure(args.data)
    print(
        f"FINAL files={len(report['files'])} bytes={report['bytes']} failures={len(report['failures'])}",
        flush=True,
    )
    return bool(report["failures"])


if __name__ == "__main__":
    raise SystemExit(main())
