"""Forever liquid table identities and MH2O texture dependency closure."""

import csv
import struct

TABLES = {
    "LiquidType": 1371380,
    "LiquidMaterial": 1132538,
    "LiquidObject": 1308058,
    "LiquidTypeXTexture": 2261065,
}


def read_tables(data, build):
    tables = {}
    for name in TABLES:
        with (data / "db2" / build / f"{name}.csv").open(newline="") as handle:
            tables[name] = list(csv.DictReader(handle))
    return tables


def layer_keys(payload):
    if len(payload) < 256 * 12:
        raise ValueError("truncated MH2O chunk headers")
    keys = set()
    for chunk in range(256):
        offset, count = struct.unpack_from("<II", payload, chunk * 12)
        if offset + count * 24 > len(payload):
            raise ValueError(f"MH2O chunk {chunk}: truncated instances")
        for index in range(count):
            keys.add(struct.unpack_from("<HH", payload, offset + index * 24))
    return keys


def texture_references(keys, tables):
    types = {int(row["ID"]): row for row in tables["LiquidType"]}
    objects = {int(row["ID"]): row for row in tables["LiquidObject"]}
    materials = {int(row["ID"]): row for row in tables["LiquidMaterial"]}
    selected = set()
    for liquid_type, liquid_object in keys:
        if liquid_object >= 42:
            if liquid_object not in objects:
                raise ValueError(f"Forever LiquidObject {liquid_object} has no DB2 row")
            liquid_type = int(objects[liquid_object]["LiquidTypeID"])
        if liquid_type not in types:
            raise ValueError(f"Forever LiquidType {liquid_type} has no DB2 row")
        material = int(types[liquid_type]["MaterialID"])
        if material not in materials:
            raise ValueError(f"Forever LiquidMaterial {material} has no DB2 row")
        selected.add(liquid_type)
    textures = {
        int(row["FileDataID"])
        for row in tables["LiquidTypeXTexture"]
        if int(row["LiquidTypeID"]) in selected and int(row["FileDataID"])
    }
    mapped = {int(row["LiquidTypeID"]) for row in tables["LiquidTypeXTexture"]}
    if selected - mapped:
        raise ValueError(
            f"Forever LiquidType textures missing: {sorted(selected - mapped)}"
        )
    refs = {(fdid, f"textures/{fdid}.blp") for fdid in textures}
    material_ids = {int(types[type_id]["MaterialID"]) for type_id in selected}
    if material_ids & {2, 4}:
        refs.add((768431, "textures/768431.blob"))
    if 18 in material_ids:
        refs.update((fdid, f"textures/{fdid}.blp") for fdid in (1797551, 1844666))
    return sorted(refs)


def asset_references(raw, tables):
    # Import here to keep the existing importer's standalone entry point usable.
    try:
        from scripts.import_forever_zephras import chunks
    except ModuleNotFoundError:
        from import_forever_zephras import chunks
    payload = dict(chunks(raw)).get(b"O2HM")
    return [] if payload is None else texture_references(layer_keys(payload), tables)
