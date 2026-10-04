#!/usr/bin/env python3
"""Decode Forever 70205 DB2s and cache Skyborne assets from local CASC only."""

import argparse
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import sqlite3
import struct
import subprocess
import sys

try:
    from scripts import export_db2_csv as export
except ModuleNotFoundError:
    import export_db2_csv as export

BUILD = "1.60.1.70205"
BUILD_KEY = "842b2e5d11f8d6fe257a5b73bd5cf6c6"
CASC_LOCAL = Path(
    "/syncthing/Sync/Projects/world-of-osso/asset-resolver/target/debug/casc-local"
)
DEFINITIONS = Path("/home/osso/Repos/wowless/vendor/dbdefs/definitions")
CACHE = (
    Path.home() / ".cache/asset-resolver/casc/wow_classic_beta" / BUILD_KEY / "schema-2"
)
TABLES = dict(
    zip(
        (
            "ChrRaces CharBaseInfo ChrRaceXChrModel ChrModel CreatureDisplayInfo CreatureModelData "
            "ChrCustomizationOption ChrCustomizationChoice ChrCustomizationElement ChrCustomizationReq "
            "ChrCustomizationReqChoice ChrCustomizationMaterial ChrCustomizationSkinnedModel "
            "ChrModelTextureLayer ChrModelMaterial ChrCustomizationCategory ChrCustomizationGeoset "
            "CharHairGeosets CharComponentTextureLayouts CharComponentTextureSections TextureFileData "
            "ChrRacesCreateScreenIcon UiTextureAtlasElement UiTextureAtlasMember UiTextureAtlas Map"
        ).split(),
        (
            1305311,
            1343386,
            3490304,
            3384313,
            1108759,
            1365368,
            3384247,
            3450554,
            3512765,
            3450453,
            3580359,
            3459652,
            3460183,
            3548976,
            3566562,
            3526439,
            3456171,
            1256914,
            1360262,
            1360263,
            982459,
            4566929,
            1989276,
            897532,
            897470,
            1349477,
        ),
        strict=True,
    )
)
NEW_TABLES = {
    "CharBaseInfo",
    "ChrRacesCreateScreenIcon",
    "UiTextureAtlas",
    "UiTextureAtlasElement",
    "UiTextureAtlasMember",
    "Map",
}
RETAIL_TABLES = {"ChrRaces", "CreatureDisplayInfo", "CreatureModelData"}


def parse_definition(text, layout):
    blocks = re.split(r"\n\s*\n", text)
    types = {}
    for line in blocks[0].splitlines()[1:]:
        match = re.match(r"(int(?:<[^>]+>)?|float|locstring|string)\s+(\w+)\??", line)
        if match:
            types[match[2]] = match[1].split("<")[0]
    matches = [
        b
        for b in blocks[1:]
        if re.search(r"^LAYOUT .*\b" + f"{layout:08X}" + r"\b", b, re.M)
    ]
    if len(matches) != 1:
        raise ValueError(f"expected one DBD layout {layout:08X}, found {len(matches)}")
    block = matches[0]
    columns, index, id_field = [], 0, 0
    for line in block.splitlines():
        if line.startswith(("LAYOUT", "BUILD", "COMMENT")):
            continue
        match = re.fullmatch(
            r"(\$[^$]+\$)?(\w+)(?:<([u]?)(\d+)>)?(?:\[(\d+)\])?(?:\s*//.*)?", line
        )
        if not match:
            raise ValueError(f"unsupported DBD line: {line}")
        annotation, name, unsigned, width, count = match.groups()
        flags = (annotation or "").strip("$").split(",")
        if "noninline" in flags:
            if "id" in flags:
                source = "id"
            elif "relation" in flags:
                source = "parent"
            else:
                raise ValueError(f"unsupported DBD annotation: {line}")
            columns.append((name, source))
            continue
        if "id" in flags:
            id_field = index
        kind = types[name]
        count = int(count or 1)
        for element in range(count):
            csv_name = name if count == 1 else f"{name}_{element}"
            if "id" in flags:
                source = "id"
            elif kind in ("string", "locstring"):
                source = ("string", index)
            elif kind == "float":
                source = ("float", index, element)
            elif kind == "int" and width in ("8", "16", "32", "64"):
                source = (("u" if unsigned else "i") + width, index, element)
            else:
                raise ValueError(f"unsupported DBD field: {line}")
            columns.append((csv_name, source))
        index += 1
    return columns, id_field, BUILD in block


def decode_rows(data, layout, columns, id_field):
    rows, dropped, fields, _ = export.read_wdc5(data, layout, id_field)
    output = []
    for row_id, (values, parent, offset) in sorted(rows.items()):
        row = {}
        for name, source in columns:
            if source == "id":
                value = row_id
            elif source == "parent":
                if parent is None:
                    raise ValueError(f"record {row_id}: missing relationship {name}")
                value = parent
            else:
                kind, index, *elements = source
                value = values[index]
                if kind == "string":
                    value = export.read_string(data, offset, fields[index], value)
                else:
                    width = 32 if kind == "float" else int(kind[1:])
                    element = elements[0]
                    bits = (
                        value[element]
                        if isinstance(value, tuple)
                        else value >> (width * element)
                    )
                    bits &= (1 << width) - 1
                    if kind == "float":
                        value = format(
                            struct.unpack("<f", struct.pack("<I", bits))[0], ".9g"
                        )
                    else:
                        value = (
                            bits - (1 << width)
                            if kind[0] == "i" and bits >> (width - 1)
                            else bits
                        )
            row[name] = value
        output.append(row)
    return output, dropped


def encode_csv(header, columns, rows):
    names = (
        next(csv.reader(io.StringIO(header.decode("utf-8-sig"))))
        if header
        else [c[0] for c in columns]
    )
    known = {name for name, _ in columns}
    missing = [name for name in names if name not in known]
    stream = io.StringIO(newline="")
    writer = csv.writer(stream)
    if not header:
        writer.writerow(names)
    for row in rows:
        writer.writerow([row.get(name, "") for name in names])
    return (header or b"") + stream.getvalue().encode(), missing


def find_definition(data, table):
    candidates = [
        data / f"forever-{BUILD}" / "definitions" / f"{table}.dbd",
        Path("/tmp/skyborn-dbd") / f"{table}.dbd",
        DEFINITIONS / f"{table}.dbd",
    ]
    return next((path for path in candidates if path.is_file()), candidates[-1])


def extract_missing(staging, fdids):
    missing = [
        fdid for fdid in sorted(set(fdids)) if not list(staging.glob(f"{fdid}.*"))
    ]
    if not missing:
        return
    env = dict(os.environ, WOW_PRODUCT="wow_classic_beta")
    result = subprocess.run(
        [str(CASC_LOCAL), *map(str, missing), "-o", str(staging)], env=env, check=False
    )
    if result.returncode:
        print(
            f"casc-local exited {result.returncode}; checking each requested FDID",
            flush=True,
        )


def extracted_path(staging, fdid):
    paths = sorted(staging.glob(f"{fdid}.*"))
    if len(paths) != 1:
        raise ValueError(f"FDID {fdid}: expected one extracted file, found {paths}")
    return paths[0]


def import_tables(data, staging):
    out = data / "db2" / BUILD
    out.mkdir(parents=True, exist_ok=True)
    extract_missing(staging, TABLES.values())
    tables, provenance, failures = {}, {}, []
    connection = sqlite3.connect(
        f"file:{CACHE / 'resolution.sqlite'}?mode=ro", uri=True
    )
    manifest = json.loads((DEFINITIONS.parent / "manifest.json").read_text())
    for table, fdid in TABLES.items():
        try:
            raw = extracted_path(staging, fdid).read_bytes()
            if raw[:4] != b"WDC5":
                raise ValueError("not WDC5")
            table_hash, layout = struct.unpack_from("<II", raw, 152)
            entry = next(e for e in manifest if e["tableName"] == table)
            if (
                int(entry["tableHash"], 16) != table_hash
                or entry["db2FileDataID"] != fdid
            ):
                raise ValueError("FDID/TableHash differs from WoWDBDefs manifest")
            definition = find_definition(data, table)
            columns, id_field, explicit = parse_definition(
                definition.read_text(), layout
            )
            rows, dropped = decode_rows(raw, layout, columns, id_field)
            header_path = (
                data / "db2/12.1.0.69933" / f"{table}.csv"
                if table in RETAIL_TABLES
                else data / f"{table}.csv"
            )
            header = b"" if table in NEW_TABLES else header_path.open("rb").readline()
            encoded, missing = encode_csv(header, columns, rows)
            (out / f"{table}.csv").write_bytes(encoded)
            tables[table] = rows
            record = connection.execute(
                "select content_key,encoding_key from resolution where fdid=?", (fdid,)
            ).fetchone()
            if not record:
                raise ValueError("missing CASC resolution provenance")
            content_key, encoding_key = (bytes(v).hex() for v in record)
            digest = hashlib.md5(raw).hexdigest()
            if content_key != digest and not dropped:
                raise ValueError(f"content-key MD5 mismatch {content_key} != {digest}")
            sections = struct.unpack_from("<I", raw, 200)[0]
            unknown = []
            size = struct.unpack_from("<I", raw, 144)[0]
            for i in range(sections):
                key, start, count, *_ = struct.unpack_from("<Q8I", raw, 204 + i * 40)
                if key and count and not any(raw[start : start + count * size]):
                    unknown.append(f"{key:016X}")
            provenance[table] = {
                "fdid": fdid,
                "content_key": content_key,
                "encoding_key": encoding_key,
                "md5": digest,
                "content_key_matches_extracted_md5": content_key == digest,
                "content_key_mismatch_reason": (
                    "local CASC zero-filled unknown encrypted chunks"
                    if content_key != digest
                    else None
                ),
                "sha256": hashlib.sha256(raw).hexdigest(),
                "TableHash": f"{table_hash:08X}",
                "LayoutHash": f"{layout:08X}",
                "dbd_file": str(definition),
                "dbd_sha256": hashlib.sha256(definition.read_bytes()).hexdigest(),
                "explicit_build": explicit,
                "layout_match": "explicit-build" if explicit else "hash-match",
                "rows_decoded": len(rows),
                "encrypted_rows_dropped": dropped,
                "unknown_tact_key_ids": unknown,
                "absent_forever_columns": missing,
            }
            print(
                f"{table}: {len(rows)} rows, {dropped} encrypted dropped; absent columns: {missing}",
                flush=True,
            )
        except (ValueError, OSError, KeyError, StopIteration, struct.error) as error:
            failures.append(f"{table} ({fdid}): {error}")
            print(f"FAILED {failures[-1]}", flush=True)
    connection.close()
    (out / "provenance.json").write_text(
        json.dumps(
            {
                "product": "wow_classic_beta",
                "build": BUILD,
                "build_key": BUILD_KEY,
                "tables": provenance,
                "failures": failures,
            },
            indent=2,
        )
        + "\n"
    )
    return tables, failures


def asset_references(raw):
    result = {}
    offset = 0
    kinds = {
        b"SFID": "skin",
        b"AFID": "anim",
        b"BFID": "bone",
        b"TXID": "blp",
        b"SKID": "skel",
    }
    while offset < len(raw):
        if offset + 8 > len(raw):
            raise ValueError("truncated model chunk header")
        tag, size = struct.unpack_from("<4sI", raw, offset)
        offset += 8
        payload = raw[offset : offset + size]
        if len(payload) != size:
            raise ValueError(f"truncated {tag!r} chunk")
        if tag in kinds:
            fmt = "<HHI" if tag == b"AFID" else "<I"
            result.setdefault(kinds[tag], []).extend(
                v[-1] for v in struct.iter_unpack(fmt, payload) if v[-1]
            )
        offset += size
    return result


def customization_assets(tables):
    options = {
        int(r["ID"])
        for r in tables["ChrCustomizationOption"]
        if int(r["ChrModelID"]) in (218, 219)
    }
    choices = {
        int(r["ID"])
        for r in tables["ChrCustomizationChoice"]
        if int(r["ChrCustomizationOptionID"]) in options
    }
    elements = [
        r
        for r in tables["ChrCustomizationElement"]
        if int(r["ChrCustomizationChoiceID"]) in choices
    ]
    materials = {int(r["ChrCustomizationMaterialID"]) for r in elements}
    skinned = {int(r["ChrCustomizationSkinnedModelID"]) for r in elements}
    resources = {
        int(r["MaterialResourcesID"])
        for r in tables["ChrCustomizationMaterial"]
        if int(r["ID"]) in materials
    }
    textures = {
        int(r["FileDataID"])
        for r in tables["TextureFileData"]
        if int(r["MaterialResourcesID"]) in resources
    }
    models = {
        int(r["CollectionsFileDataID"])
        for r in tables["ChrCustomizationSkinnedModel"]
        if int(r["ID"]) in skinned
    }
    return textures - {0}, models - {0}


def validate_magic(raw, extension, content_key=None):
    # Forever BFID payloads start with version 1, then BIDA/BOMT chunks.
    if extension == "bone" and raw[:8] == b"\x01\0\0\0BIDA":
        asset_references(raw[4:])  # Validate every chunk's bounds.
        return
    # Observed Skyborne AFID files are raw timestamp/keyframe streams: no magic.
    # Their extension comes from AFID, and exact CASC content identity is required.
    if extension == "anim" and raw[:4] not in (b"AFM2", b"AFSA", b"AFSB"):
        if not raw or content_key != hashlib.md5(raw).hexdigest():
            raise ValueError("raw AFID animation requires matching CASC content key")
        return
    magics = {
        "m2": (b"MD21", b"MD20"),
        "skin": (b"SKIN",),
        "blp": (b"BLP2",),
        "anim": (b"AFM2", b"AFSA", b"AFSB"),
        "bone": (b"BONE",),
        "skel": (b"SKL1",),
    }
    if raw[:4] not in magics[extension]:
        raise ValueError(f"expected {extension} magic, got {raw[:4]!r}")


def import_assets(data, staging, tables):
    textures, collections = customization_assets(tables)
    pending = {(fdid, "blp") for fdid in textures | {8200220, 8199012}}
    pending |= {(fdid, "m2") for fdid in collections | {7478487, 7478494}}
    connection = sqlite3.connect(
        f"file:{CACHE / 'resolution.sqlite'}?mode=ro", uri=True
    )
    aliases = []
    visited, failures, counts = set(), [], {"present": 0, "extracted": 0}
    while pending:
        batch = sorted(pending - visited)
        pending.clear()
        if not batch:
            break
        destinations = {
            (fdid, ext): data
            / ("textures" if ext == "blp" else "models")
            / f"{fdid}.{ext}"
            for fdid, ext in batch
        }
        extract_missing(
            staging,
            [fdid for (fdid, ext), path in destinations.items() if not path.is_file()],
        )
        for fdid, ext in batch:
            visited.add((fdid, ext))
            path = destinations[fdid, ext]
            try:
                present = path.is_file()
                source = path if present else extracted_path(staging, fdid)
                raw = source.read_bytes()
                content_key = None
                if ext == "anim":
                    record = connection.execute(
                        "select content_key from resolution where fdid=?", (fdid,)
                    ).fetchone()
                    if record:
                        content_key = bytes(record[0]).hex()
                validate_magic(raw, ext, content_key)
                if not present:
                    path.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(source, path)
                counts["present" if present else "extracted"] += 1
                if ext in ("m2", "skel"):
                    refs = asset_references(raw)
                    pending.update(
                        (child, kind)
                        for kind, children in refs.items()
                        for child in children
                    )
                    if ext == "m2":
                        for index, child in enumerate(refs.get("skin", [])):
                            alias = data / "models" / f"{fdid}{index:02}.skin"
                            # Consumer reads model00.skin, not SFID.skin.
                            pending.add((child, "skin"))
                            aliases.append((child, "skin", alias))
                        for child in refs.get("skel", []):
                            aliases.append(
                                (child, "skel", data / "models" / f"{fdid}.skel")
                            )
            except (ValueError, OSError, struct.error) as error:
                failures.append(f"asset {fdid}.{ext}: {error}")
                print(f"FAILED {failures[-1]}", flush=True)
    connection.close()
    for fdid, ext, alias in aliases:
        source = data / "models" / f"{fdid}.{ext}"
        if source.is_file() and not alias.is_file():
            shutil.copyfile(source, alias)
    counts.update(total=len(visited), failures=len(failures))
    print(f"assets: {json.dumps(counts)}", flush=True)
    (data / "db2" / BUILD / "assets.json").write_text(
        json.dumps(
            {
                "counts": counts,
                "assets": sorted(f"{fdid}.{ext}" for fdid, ext in visited),
                "failures": failures,
            },
            indent=2,
        )
        + "\n"
    )
    return failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--data", type=Path, default=Path(__file__).resolve().parent.parent / "data"
    )
    args = parser.parse_args()
    staging = args.data / "cache/forever-skyborne-extract"
    staging.mkdir(parents=True, exist_ok=True)
    tables, failures = import_tables(args.data, staging)
    required = {
        "ChrCustomizationOption",
        "ChrCustomizationChoice",
        "ChrCustomizationElement",
        "ChrCustomizationMaterial",
        "TextureFileData",
        "ChrCustomizationSkinnedModel",
    }
    if required <= tables.keys():
        failures.extend(import_assets(args.data, staging, tables))
    else:
        failures.append(
            f"asset closure blocked by tables: {sorted(required - tables.keys())}"
        )
    for model in (218, 219):
        options = {
            r["ID"]
            for r in tables.get("ChrCustomizationOption", [])
            if int(r["ChrModelID"]) == model
        }
        choices = [
            r
            for r in tables.get("ChrCustomizationChoice", [])
            if r["ChrCustomizationOptionID"] in options
        ]
        print(f"ChrModel {model}: {len(options)} options, {len(choices)} choices")
    print(
        "Skyborne races:",
        [r["ID"] for r in tables.get("ChrRaces", []) if int(r["ID"]) in (95, 96)],
    )
    if failures:
        print("Failures:\n" + "\n".join(failures), flush=True)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
