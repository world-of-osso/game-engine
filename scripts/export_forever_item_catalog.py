#!/usr/bin/env python3
"""Export the pinned Forever starter item catalog from local DB2s, never Retail/SQL."""

import argparse
import csv
import hashlib
import importlib
import io
import json
import os
from pathlib import Path
import shutil
import sqlite3
import struct
import sys
import tempfile

from import_forever_skyborne import decode_rows, parse_definition

BUILD = "1.60.1.70205"
LOADOUT_IDS = list(range(2373, 2379))
ITEM_TABLES = {
    "CharacterLoadout": "character_loadout",
    "CharacterLoadoutItem": "character_loadout_item",
    "Item": "item",
    "ItemSparse": "item_sparse",
    "ItemAppearance": "item_appearance",
    "ItemModifiedAppearance": "item_modified_appearance",
    "ItemEffect": "item_effect",
    "ItemXItemEffect": "item_x_item_effect",
}
# Local clear 70205 snapshots. Item/loadout pins remain owned by skyborne_data.
SCALING_SOURCES = {
    "ArmorLocation": (
        1284818,
        0xFB67352F,
        "565dee93a070e1470205e227a1fb236463f5daa8832abf0bf4043ce3bd681d2f",
    ),
    "ItemArmorQuality": (
        1283021,
        0x2935AA9D,
        "67628d34b90d977514d14ae70ed658ec3e19f356f882cc2f1e029c1db46c5c2b",
    ),
    "ItemArmorShield": (
        1277741,
        0x7E6C94F9,
        "b35098dd45d3367414dbe97dd19127a010a0ff8222a935f8faefb3aa29cc3def",
    ),
    "ItemArmorTotal": (
        1283022,
        0xEB155D51,
        "a644a2edbae2677c46b49d69b72c311e34904e44598f0a3fa42da74d6769d5b7",
    ),
    "ItemDamageOneHand": (
        1277743,
        0x56F30531,
        "35bbc193595e611eaac37ee72b6e4617697f6d7d687582acb5231814d28a4653",
    ),
    "ItemDamageOneHandCaster": (
        1277739,
        0x56F30531,
        "8c5d1481919488dd485b703bc19abced4ea81a8552eee8cd5410be4700403e68",
    ),
    "ItemDamageTwoHand": (
        1277738,
        0x56F30531,
        "90496b8efdb747df7293571c27925407a0dbc528ee5544d53733bfd83c08ee59",
    ),
    "ItemDamageTwoHandCaster": (
        1277742,
        0x56F30531,
        "ac0697a06745bcbe90ce231d42ac53cc0cbb1fcdff71a851fcb9001a71ffdae7",
    ),
    "ItemSubClass": (
        1261604,
        0x1E67DB87,
        "f5283e016ce14686725e8997f17f027cbd30faf012df0a1cdb2ebd9f0165286b",
    ),
    "RandPropPoints": (
        1310245,
        0x4FD22743,
        "a3edc16cbf61a3b0ea7ef297692fef5def352a882a6df5c30254caea28f79a3e",
    ),
}
SCHEMA_SHA256 = {
    "ArmorLocation": "5cc4ab95ae3849df0ba8fe01861ca041f577fa4a0afa184ef4b3fcebbcc06050",
    "CharacterLoadout": "8a041ad8469bc1f7204c97a21ab447de73626e542a205c0b3d4c1e679333192b",
    "CharacterLoadoutItem": "1a81c6883306836dd66dc38090f315517f1adccd9d04b706a670e57072551d80",
    "Item": "27cd6b58e6dd5166266f0f8a69f420f700c8480b584952c8f4260bf651ae3209",
    "ItemAppearance": "104e9a06ebee926c178cccd2815c25f186afc75d968db32af162ad64be10472b",
    "ItemArmorQuality": "3ba03d09b1f7f9b3302f9ffe626601467f3cac988781dc9e4b37450e35656715",
    "ItemArmorShield": "b635c6c26bb463ca616e3d8c9d64ad464dfa4d1d2bd7596b5d0ecc4b201e09bb",
    "ItemArmorTotal": "e74a751a870752e957f6108a0504bcc1ef31dd70deeb9d9e42151b948ec9284d",
    "ItemDamageOneHand": "3f6a5e14db1d6a469dba2b0ab2d8498712ae101a61668d19908cd5b101892289",
    "ItemDamageOneHandCaster": "3f6a5e14db1d6a469dba2b0ab2d8498712ae101a61668d19908cd5b101892289",
    "ItemDamageTwoHand": "3f6a5e14db1d6a469dba2b0ab2d8498712ae101a61668d19908cd5b101892289",
    "ItemDamageTwoHandCaster": "3f6a5e14db1d6a469dba2b0ab2d8498712ae101a61668d19908cd5b101892289",
    "ItemEffect": "da7d1b971a53e814b7c68b9ab13adc63d35377d1b09808a41edfc83ac2fa9b8b",
    "ItemModifiedAppearance": "e2c6bad3dfe256326da6baf5982feb519fc555cbdf95db2dd240fb1cd18e68e5",
    "ItemSparse": "ff87dafe875376c0ee5b6951ef4668aaae53a862d190cd9d3e6b708a9deafe9b",
    "ItemSubClass": "bd1739eb0de65ce3bf4398784f508916a7ea0e49a5b2100dd2ae7c7d7004c79d",
    "ItemXItemEffect": "ba49e31174184dbfd42c654576fc7819e148260fed0c7c7b8f5154a54aedc337",
    "RandPropPoints": "4070bfa66ecee3441dd149c4c9199bd933d00a7dee24cfc698bc30d5aa256c9c",
}
CONTRACT = {
    "Item": "ID ClassID SubclassID IconFileDataID SheatheType".split(),
    "ItemSparse": "ID Display_lang OverallQualityID Stackable SellPrice Bonding RequiredLevel InventoryType ItemLevel MaxCount Description_lang ContainerSlots ExpansionID ItemDelay DmgVariance Flags_1".split()
    + [f"StatModifier_bonusStat_{i}" for i in range(10)]
    + [f"StatPercentEditor_{i}" for i in range(10)],
    "ItemSubClass": "ClassID SubClassID DisplayName_lang".split(),
    "ItemAppearance": "ID DefaultIconFileDataID ItemDisplayInfoID".split(),
    "ItemModifiedAppearance": "ItemID ItemAppearanceID OrderIndex".split(),
    "ItemArmorQuality": ["ID"] + [f"Qualitymod_{i}" for i in range(7)],
    "ItemArmorTotal": "ItemLevel Cloth Leather Mail Plate".split(),
    "ArmorLocation": "ID Clothmodifier Leathermodifier Chainmodifier Platemodifier".split(),
    "RandPropPoints": ["ID"]
    + [f"{kind}F_{i}" for kind in ("Good", "Superior", "Epic") for i in range(5)],
}
for _name in (
    "ItemArmorShield",
    "ItemDamageOneHand",
    "ItemDamageOneHandCaster",
    "ItemDamageTwoHand",
    "ItemDamageTwoHandCaster",
):
    CONTRACT[_name] = ["ItemLevel"] + [f"Quality_{i}" for i in range(7)]


def hash_bytes(content):
    return hashlib.sha256(content).hexdigest()


def read_pinned(path, expected):
    content = path.read_bytes()
    digest = hash_bytes(content)
    if digest != expected:
        raise ValueError(f"{path}: SHA256 {digest} != pinned {expected}")
    return content


def load_server_reader(root):
    scripts = root.resolve() / "scripts"
    if not (scripts / "skyborne_data.py").is_file():
        raise ValueError(f"{scripts}: maintained server reader missing")
    sys.path.insert(0, str(scripts))
    reader = importlib.import_module("skyborne_data")
    if Path(reader.__file__).resolve() != scripts / "skyborne_data.py":
        raise ValueError("skyborne_data loaded from a different server root")
    return reader


def validate_sources(source, definitions, reader):
    schema_provenance = json.loads((definitions / "schema-provenance.json").read_text())
    pins = dict(SCALING_SOURCES)
    for name, table in ITEM_TABLES.items():
        fdid, layout, _, digest = reader.SOURCES[table]
        pins[name] = (fdid, layout, digest)
    snapshots, manifest = {}, {}
    for name, (fdid, layout, digest) in sorted(pins.items()):
        raw = read_pinned(source / f"{fdid}.db2", digest)
        if reader.db2_casc.read_header(raw)["layout_hash"] != layout:
            raise ValueError(f"{name}: expected layout {layout:08X}")
        dbd = read_pinned(definitions / f"{name}.dbd", SCHEMA_SHA256[name])
        if schema_provenance[name]["sha256"] != SCHEMA_SHA256[name]:
            raise ValueError(f"{name}: schema-provenance SHA256 mismatch")
        columns, id_field, explicit = parse_definition(dbd.decode(), layout)
        snapshots[name] = (raw, layout, columns, id_field)
        manifest[name] = {
            "fdid": fdid,
            "layout": f"{layout:08X}",
            "raw_sha256": digest,
            "schema_sha256": SCHEMA_SHA256[name],
            "table_hash": f"{struct.unpack_from('<I', raw, 152)[0]:08X}",
            "schema_source": schema_provenance[name]["source"],
            "explicit_build": explicit,
        }
    return snapshots, manifest


def load_kit(source, reader):
    # Reuse first-party importer order/type/array mappings, without its SQL/import path.
    with sqlite3.connect(":memory:") as schema:
        schema.executescript((reader.SCRIPTS / "db2_schema.sql").read_text())
        kit = {
            table: reader.decode_source(
                source, table, reader.source_columns(schema, table)
            )
            for table in ("character_loadout", "character_loadout_item")
        }
        if sorted(r["ID"] for r in kit["character_loadout"]) != LOADOUT_IDS:
            raise ValueError("expected the six pinned loadouts 2373..2378")
        for row in kit["character_loadout"]:
            reader.validate_loadout_selector(row)
        reader.load_item_rows(source, schema, kit)
    reader.validate_item_rows(kit)
    ids = sorted({r["ItemID"] for r in kit["character_loadout_item"]})
    if len(ids) != 30:
        raise ValueError(f"expected 30 distinct kit items, found {len(ids)}")
    return kit, ids


def encode_contract(name, rows):
    columns = CONTRACT[name]
    stream = io.StringIO(newline="")
    writer = csv.writer(stream, lineterminator="\n")
    writer.writerow(columns)
    for row in sorted(rows, key=lambda r: r["ID"]):
        missing = [
            column for column in columns if column not in row or row[column] is None
        ]
        if missing:
            raise ValueError(
                f"{name} row {row['ID']}: missing critical fields {missing}"
            )
        writer.writerow([row[column] for column in columns])
    return stream.getvalue().encode()


def build_catalog(source, definitions, reader):
    snapshots, sources = validate_sources(source, definitions, reader)
    kit, ids = load_kit(source, reader)
    tables = {
        name: kit[ITEM_TABLES[name]]
        for name in ("Item", "ItemSparse", "ItemAppearance", "ItemModifiedAppearance")
    }
    for name in SCALING_SOURCES:
        raw, layout, columns, id_field = snapshots[name]
        rows, dropped = decode_rows(raw, layout, columns, id_field)
        if dropped:
            raise ValueError(
                f"{name}: {dropped} unreadable rows; complete clear source required"
            )
        tables[name] = rows
    class_keys = {(r["ClassID"], r["SubclassID"]) for r in tables["Item"]}
    tables["ItemSubClass"] = [
        r
        for r in tables["ItemSubClass"]
        if (r["ClassID"], r["SubClassID"]) in class_keys
    ]
    found = {(r["ClassID"], r["SubClassID"]) for r in tables["ItemSubClass"]}
    if found != class_keys:
        raise ValueError(f"missing critical class labels: {sorted(class_keys - found)}")
    # Requested starter equipment scaling domain; full source rows are exported.
    levels = [1, 2]
    item_levels = sorted({r["ItemLevel"] for r in tables["ItemSparse"]})
    for name in SCALING_SOURCES.keys() - {"ArmorLocation", "ItemSubClass"}:
        key = "ItemLevel" if "ItemLevel" in CONTRACT[name] else "ID"
        missing = set(levels) - {r[key] for r in tables[name]}
        if missing:
            raise ValueError(
                f"{name}: missing critical scaling levels {sorted(missing)}"
            )
    # Only cloth/leather/mail/plate use ArmorLocation; robes read the chest row.
    armor_ids = {
        r["ID"]
        for r in tables["Item"]
        if r["ClassID"] == 4 and 1 <= r["SubclassID"] <= 4
    }
    locations = {
        5 if r["InventoryType"] == 20 else r["InventoryType"]
        for r in tables["ItemSparse"]
        if r["ID"] in armor_ids
    }
    missing = locations - {r["ID"] for r in tables["ArmorLocation"]}
    if missing:
        raise ValueError(
            f"ArmorLocation: missing critical inventory types {sorted(missing)}"
        )
    files, outputs = {}, {}
    for name, rows in sorted(tables.items()):
        encoded = encode_contract(name, rows)
        files[f"{name}.csv"] = encoded
        outputs[f"{name}.csv"] = {
            "rows": len(rows),
            "row_ids": sorted(r["ID"] for r in rows),
            "columns": CONTRACT[name],
            "sha256": hash_bytes(encoded),
        }
    manifest = {
        "build": BUILD,
        "product": "wow_classic_beta",
        "selected_loadout_ids": LOADOUT_IDS,
        "selected_item_ids": ids,
        "selected_levels": levels,
        "authored_item_levels": item_levels,
        "sources": sources,
        "outputs": outputs,
        "missing_critical_fields": [],
        "readers": {
            name: hash_bytes((reader.SCRIPTS / name).read_bytes())
            for name in ("skyborne_data.py", "db2_casc.py", "db2_schema.sql")
        },
    }
    files["manifest.json"] = (
        json.dumps(manifest, sort_keys=True, indent=2) + "\n"
    ).encode()
    return files


def publish_catalog(output, files):
    if output.exists():
        raise ValueError(
            f"{output}: destination exists; use a fresh build/items directory"
        )
    output.parent.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=".items-", dir=output.parent))
    try:
        for name, content in files.items():
            (stage / name).write_bytes(content)
        os.rename(stage, output)
    finally:
        if stage.exists():
            shutil.rmtree(stage)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--server-root",
        required=True,
        type=Path,
        help="first-party server checkout; scripts only, no SQL database",
    )
    parser.add_argument(
        "--source-dir",
        required=True,
        type=Path,
        help="local 70205 starter-kit-probe DB2s",
    )
    parser.add_argument(
        "--definitions",
        type=Path,
        help="pinned official DBDs and schema-provenance.json",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "data/db2" / BUILD / "items",
    )
    args = parser.parse_args()
    try:
        if args.output.parts[-2:] != (BUILD, "items"):
            raise ValueError(
                f"output must end in {BUILD}/items; Retail paths forbidden"
            )
        if args.output.exists():
            raise ValueError(f"{args.output}: destination exists")
        reader = load_server_reader(args.server_root)
        files = build_catalog(
            args.source_dir,
            args.definitions or args.source_dir.parent / "definitions",
            reader,
        )
        publish_catalog(args.output, files)
    except (
        OSError,
        ValueError,
        KeyError,
        struct.error,
        ImportError,
        sqlite3.Error,
    ) as error:
        print(f"Forever item export failed: {error}", file=sys.stderr)
        return 1
    print(f"Forever {BUILD}: 30 items, 14 CSVs and manifest in {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
