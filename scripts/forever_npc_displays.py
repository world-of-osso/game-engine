"""Forever-only NPC joins. Retail display IDs never enter the overlay."""

import csv
import sqlite3
import struct
from collections import defaultdict
from contextlib import closing

try:
    from scripts import import_npc_appearance
except ModuleNotFoundError:
    import import_npc_appearance


TABLES = {
    "CreatureDisplayInfoExtra": 1264997,
    "CreatureDisplayInfoOption": 3692043,
    "CreatureDisplayInfoGeosetData": 1720141,
    "NPCModelItemSlotDisplayInfo": 1340661,
    "ItemDisplayInfo": 1266429,
    "ItemDisplayInfoMaterialRes": 1280614,
    "ModelFileData": 1337833,
    "ComponentModelFileData": 1349053,
    "ComponentTextureFileData": 1278239,
    "HelmetGeosetData": 2821752,
}


def encrypted_record_ids(raw):
    """Read cleartext encrypted-ID metadata, not zero-filled record bodies.

    WDC5 stores one counted ID list per keyed section after palette/common data;
    see wowless tools/db2.lua (encpos). These IDs identify omissions only.
    """
    fields = struct.unpack_from("<I", raw, 176)[0]
    common, palette, sections = struct.unpack_from("<3I", raw, 192)
    position = 204 + sections * 40 + fields * 28 + common + palette
    size = struct.unpack_from("<I", raw, 144)[0]
    dropped = []
    for index in range(sections):
        key, start, count, *_ = struct.unpack_from("<Q8I", raw, 204 + index * 40)
        if not key:
            continue
        ids_count = struct.unpack_from("<I", raw, position)[0]
        position += 4
        ids = struct.unpack_from(f"<{ids_count}I", raw, position)
        position += ids_count * 4
        if count and not any(raw[start : start + count * size]):
            dropped.extend(ids)
    return sorted(set(dropped))


def index_rows(tables, name, column):
    return {int(row[column]): row for row in tables.get(name, [])}


def group_rows(tables, name, column):
    groups = defaultdict(list)
    for row in tables.get(name, []):
        groups[int(row[column])].append(row)
    return groups


def collect_material_assets(resources, texture_rows):
    missing = {
        resource
        for resource in resources - {0}
        if not any(int(row["FileDataID"]) for row in texture_rows.get(resource, []))
    }
    if missing:
        raise ValueError(
            f"missing TextureFileData material resources {sorted(missing)}"
        )
    return {
        (int(row["FileDataID"]), "blp")
        for resource in resources - {0}
        for row in texture_rows.get(resource, [])
        if int(row["FileDataID"])
    }


def collect_choice_assets(tables, choices, texture_rows):
    elements = [
        row
        for row in tables.get("ChrCustomizationElement", [])
        if int(row["ChrCustomizationChoiceID"]) in choices
    ]
    materials = {int(row["ChrCustomizationMaterialID"]) for row in elements} - {0}
    collections = {int(row["ChrCustomizationSkinnedModelID"]) for row in elements} - {0}
    resources = {
        int(row["MaterialResourcesID"])
        for row in tables.get("ChrCustomizationMaterial", [])
        if int(row["ID"]) in materials
    }
    assets = collect_material_assets(resources, texture_rows)
    assets.update(
        (int(row["CollectionsFileDataID"]), "m2")
        for row in tables.get("ChrCustomizationSkinnedModel", [])
        if int(row["ID"]) in collections and int(row["CollectionsFileDataID"])
    )
    return assets


def collect_item_assets(item_ids, tables, texture_rows):
    items = index_rows(tables, "ItemDisplayInfo", "ID")
    missing = item_ids - items.keys()
    if missing:
        raise ValueError(f"missing ItemDisplayInfo {sorted(missing)}")
    models, resources = set(), set()
    for item in item_ids:
        for column, value in items[item].items():
            if column.startswith("ModelResourcesID_"):
                models.add(int(value))
            elif column.startswith("ModelMaterialResourcesID_"):
                resources.add(int(value))
    resources.update(
        int(row["MaterialResourcesID"])
        for row in tables.get("ItemDisplayInfoMaterialRes", [])
        if int(row["ItemDisplayInfoID"]) in item_ids
    )
    model_rows = group_rows(tables, "ModelFileData", "ModelResourcesID")
    missing = {
        model
        for model in models - {0}
        if not any(int(row["FileDataID"]) for row in model_rows.get(model, []))
    }
    if missing:
        raise ValueError(f"missing ModelFileData model resources {sorted(missing)}")
    assets = collect_material_assets(resources, texture_rows)
    assets.update(
        (int(row["FileDataID"]), "m2")
        for model in models - {0}
        for row in model_rows[model]
        if int(row["FileDataID"])
    )
    return assets


def collect_baked_asset(extra, model_path, textures):
    values = tuple(
        int(extra[name])
        for name in (
            "ID",
            "DisplayRaceID",
            "DisplaySexID",
            "DisplayClassID",
            "Flags",
            "BakeMaterialResourcesID",
            "HDBakeMaterialResourcesID",
        )
    )
    material = import_npc_appearance.select_material(values, model_path)
    if not material:
        return set()
    fdids = {
        int(row["FileDataID"])
        for row in textures.get(material, [])
        if int(row["UsageType"]) == 0
    }
    if len(fdids) != 1 or not all(fdids):
        raise ValueError(
            f"unresolved/ambiguous baked material {material}: {sorted(fdids)}"
        )
    return {(next(iter(fdids)), "blp")}


def npc_asset_roots(tables, requested, retail_ids, model_paths):
    displays = index_rows(tables, "CreatureDisplayInfo", "ID")
    models = index_rows(tables, "CreatureModelData", "ID")
    extras = index_rows(tables, "CreatureDisplayInfoExtra", "ID")
    options = group_rows(
        tables, "CreatureDisplayInfoOption", "CreatureDisplayInfoExtraID"
    )
    items = group_rows(tables, "NPCModelItemSlotDisplayInfo", "NpcModelID")
    textures = group_rows(tables, "TextureFileData", "MaterialResourcesID")
    assets, failures = {}, {}
    for display_id in sorted(requested - retail_ids):
        roots, errors = set(), []
        row = displays.get(display_id)
        if row is None:
            failures[display_id] = ["missing readable CreatureDisplayInfo"]
            continue
        model_id = int(row["ModelID"])
        model = models.get(model_id)
        if model is None or not int(model["FileDataID"]):
            failures[display_id] = [f"missing CreatureModelData ModelID {model_id}"]
            continue
        fdid = int(model["FileDataID"])
        roots.add((fdid, "m2"))
        roots.update(
            (int(value), "blp")
            for key, value in row.items()
            if key.startswith("TextureVariationFileDataID_") and int(value)
        )
        extra_id = int(row["ExtendedDisplayInfoID"])
        if extra_id:
            extra = extras.get(extra_id)
            if extra is None:
                errors.append(f"missing CreatureDisplayInfoExtra {extra_id}")
            else:
                try:
                    roots.update(
                        collect_baked_asset(extra, model_paths[fdid], textures)
                    )
                except (ValueError, KeyError) as error:
                    errors.append(str(error))
            choices = {
                int(option["ChrCustomizationChoiceID"])
                for option in options.get(extra_id, [])
            }
            try:
                roots.update(collect_choice_assets(tables, choices, textures))
            except ValueError as error:
                errors.append(str(error))
            item_ids = {
                int(item["ItemDisplayInfoID"])
                for item in items.get(extra_id, [])
                if int(item["ItemSlot"]) != 11
            }
            for item_id in sorted(item_ids):
                try:
                    roots.update(collect_item_assets({item_id}, tables, textures))
                except ValueError as error:
                    errors.append(str(error))
            for table in (
                "CreatureDisplayInfoOption",
                "CreatureDisplayInfoGeosetData",
                "NPCModelItemSlotDisplayInfo",
            ):
                if table not in tables:
                    errors.append(f"missing {table}")
        assets[display_id] = roots
        if errors:
            failures[display_id] = errors
    return assets, failures


def read_npc_import_inputs(data, requested, tables):
    with (data / "db2/12.1.0.69933/CreatureDisplayInfo.csv").open(newline="") as handle:
        retail_ids = {int(row["ID"]) for row in csv.DictReader(handle)}
    models = index_rows(tables, "CreatureModelData", "ID")
    needed = {int(row["FileDataID"]) for row in models.values()}
    with (data / "community-listfile.csv").open(newline="") as handle:
        paths = {
            int(fdid): path
            for fdid, path in csv.reader(handle, delimiter=";")
            if int(fdid) in needed
        }
    roots, errors = npc_asset_roots(tables, requested, retail_ids, paths)
    return roots, errors, retail_ids, paths


def publish_display_rows(data, tables, requested, retail_ids):
    """Add Forever-only rows to the native catalog; existing Retail rows stay byte-for-byte."""
    displays = index_rows(tables, "CreatureDisplayInfo", "ID")
    models = index_rows(tables, "CreatureModelData", "ID")
    entries = []
    for display in sorted(requested - retail_ids):
        row = displays.get(display)
        model = models.get(int(row["ModelID"])) if row else None
        if model is None or not int(model["FileDataID"]):
            continue
        scale = round(
            float(row["CreatureModelScale"]) * float(model["ModelScale"]) * 1000
        )
        skins = [int(row.get(f"TextureVariationFileDataID_{i}", 0)) for i in range(3)]
        entries.append((display, int(model["FileDataID"]), *skins, scale))
    path = data / "cache/creature_display.sqlite"
    with (
        closing(
            sqlite3.connect(path.resolve().as_uri() + "?mode=rw", uri=True)
        ) as connection,
        connection,
    ):
        connection.executemany(
            "INSERT OR IGNORE INTO creature_displays VALUES (?, ?, ?, ?, ?, ?)", entries
        )
    return len(entries)


def npc_appearance_rows(tables, selected, model_paths):
    displays = index_rows(tables, "CreatureDisplayInfo", "ID")
    models = index_rows(tables, "CreatureModelData", "ID")
    extras = {
        int(row["ID"]): (
            tuple(
                int(row[name])
                for name in (
                    "ID",
                    "DisplayRaceID",
                    "DisplaySexID",
                    "DisplayClassID",
                    "Flags",
                    "BakeMaterialResourcesID",
                    "HDBakeMaterialResourcesID",
                )
            ),
            None,
        )
        for row in tables.get("CreatureDisplayInfoExtra", [])
    }
    options = {
        index: (
            (
                int(row["ChrCustomizationOptionID"]),
                int(row["ChrCustomizationChoiceID"]),
            ),
            int(row["CreatureDisplayInfoExtraID"]),
        )
        for index, row in enumerate(tables.get("CreatureDisplayInfoOption", []))
    }
    geosets = {
        index: (
            (int(row["GeosetIndex"]), int(row["GeosetValue"])),
            int(row["CreatureDisplayInfoID"]),
        )
        for index, row in enumerate(tables.get("CreatureDisplayInfoGeosetData", []))
    }
    chosen = {
        display: int(displays[display]["ExtendedDisplayInfoID"]) for display in selected
    }
    paths = {
        display: model_paths[
            int(models[int(displays[display]["ModelID"])]["FileDataID"])
        ]
        for display in selected
        if chosen[display]
    }
    materials = {
        import_npc_appearance.select_material(extras[extra][0], paths[display])
        for display, extra in chosen.items()
        if extra
    }
    textures = {}
    for row in tables.get("TextureFileData", []):
        material, fdid = int(row["MaterialResourcesID"]), int(row["FileDataID"])
        if material not in materials or int(row["UsageType"]) != 0:
            continue
        if material in textures and textures[material] != fdid:
            raise ValueError(f"ambiguous baked material {material}")
        textures[material] = fdid
    return import_npc_appearance.join_appearances(
        chosen, extras, options, geosets, paths, textures
    )


def publish_appearance_rows(data, rows):
    path = data / "cache/npc_appearance.sqlite"
    with (
        closing(
            sqlite3.connect(path.resolve().as_uri() + "?mode=rw", uri=True)
        ) as connection,
        connection,
    ):
        existing = {
            row[0]
            for row in connection.execute("SELECT display_id FROM display_coverage")
        }
        for table, values in zip(
            ("appearances", "choices", "geosets", "display_coverage"), rows, strict=True
        ):
            new_rows = [row for row in values if row[0] not in existing]
            if new_rows:
                placeholders = ",".join("?" for _ in new_rows[0])
                connection.executemany(
                    f"INSERT INTO {table} VALUES ({placeholders})", new_rows
                )


def summarize_asset_closures(roots_by_display, graph, visited, failed):
    report = {}
    for display, roots in roots_by_display.items():
        closure, pending = set(), set(roots)
        while pending:
            asset = pending.pop()
            if asset in closure:
                continue
            closure.add(asset)
            pending.update(graph.get(asset, set()) - closure)
        missing = (closure - visited) | (closure & failed)
        report[display] = {
            "resolved": len(closure - missing),
            "failed": len(missing),
            "assets": sorted(f"{fdid}.{ext}" for fdid, ext in closure),
            "missing": sorted(f"{fdid}.{ext}" for fdid, ext in missing),
        }
    return report
