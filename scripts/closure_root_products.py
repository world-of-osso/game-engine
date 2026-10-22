"""Offline roots follow native metadata ownership, never a process-wide product.

Sources: core/map_catalog.rs, creature_display_data.rs and
outfit_catalog{,_db}.rs. Cache source rows select IO ownership; they do not
constitute authenticated cache-import provenance.
"""

from contextlib import contextmanager

from asset_closure import ascii_lower
from closure_seeds import Catalogs, number, readonly, seed_terrain, seed_skyboxes

SOURCES = {"wow": "db2/active", "wow_classic_beta": "db2/1.60.1.70205"}
ITEM_SOURCES = {"Retail": "wow", "Forever70205": "wow_classic_beta"}


@contextmanager
def source_catalog(graph, product, *, owned_items=False):
    previous = graph.product, graph.metadata_build, graph.seeds
    relative = SOURCES[product]
    graph.product = product
    graph.metadata_build = relative.removeprefix("db2/")
    overrides = {}
    if owned_items and product == "wow_classic_beta":
        overrides = {
            name: f"{relative}/items/{name}.csv"
            for name in ["ItemModifiedAppearance", "ItemAppearance"]
        }
    graph.seeds = {**graph.seeds, "table_paths": overrides}
    try:
        yield Catalogs(graph)
    finally:
        graph.product, graph.metadata_build, graph.seeds = previous


def require_product(graph, product, reason, identity):
    if product in SOURCES:
        return True
    graph.issue(
        "missing_product_source",
        f"{reason}: invalid or absent source_product {product!r}",
        identity,
    )
    return False


@contextmanager
def source_cache(graph, name):
    path = graph.data / "cache" / name
    if not path.is_file():
        graph.issue(
            "missing_metadata_file",
            f"cache/{name}: native source ownership unavailable",
        )
        yield None
        return
    graph.input_file(path)
    graph.issue(
        "unverified_cache_build",
        f"{name}: source rows select ownership, not authenticated import provenance",
    )
    with readonly(path) as db:
        yield db


def native_maps(graph):
    maps, ids, directories = [], set(), set()
    for product in SOURCES:
        with source_catalog(graph, product) as catalogs:
            for row in catalogs.rows("Map"):
                item = {
                    "id": number(row, "ID"),
                    "directory": row["Directory"],
                    "tiles": "all",
                    "wdt_fdid": number(row, "WdtFileDataID"),
                    "product": product,
                }
                directory = ascii_lower(item["directory"])
                if product == "wow_classic_beta" and (
                    item["id"] in ids or directory in directories
                ):
                    continue
                maps.append(item)
                ids.add(item["id"])
                directories.add(directory)
    return maps


def seed_map_roots(graph, config):
    requested = config["maps"]
    if not requested:
        return set()
    maps = native_maps(graph)
    selected = maps if requested == "all" else []
    if requested != "all":
        for spec in requested:
            match = next(
                (
                    row
                    for row in maps
                    if row["id"] == spec["id"]
                    and ascii_lower(row["directory"]) == ascii_lower(spec["directory"])
                ),
                None,
            )
            if match is None:
                graph.issue(
                    "missing_metadata_row",
                    f"native MapCatalog selection {spec['id']} / {spec['directory']}",
                )
                continue
            selected.append({**match, "tiles": spec["tiles"]})
    graph.seeds["map_sources"] = selected
    tiles = set()
    for product in SOURCES:
        owned = [row for row in selected if row["product"] == product]
        if not owned:
            continue
        with source_catalog(graph, product) as catalogs:
            tiles.update(seed_terrain(graph, {"maps": owned}))
            seed_skyboxes(graph, catalogs, {row["id"] for row in owned})
    return tiles


def seed_display_appearance(graph, product, display, model_fdid):
    with source_catalog(graph, product) as catalogs:
        row = catalogs.index("CreatureDisplayInfo").get(display)
        if row is None:
            graph.issue(
                "missing_metadata_row",
                f"{product} CreatureDisplayInfo ID={display}: source-local appearance join unavailable",
            )
            return
        extra = number(row, "ExtendedDisplayInfoID")
        if not extra:
            return
        from closure_appearance import DisplayAppearanceReferences

        appearances = graph.__dict__.setdefault("source_appearances", {})
        if product not in appearances:
            appearances[product] = DisplayAppearanceReferences(graph, catalogs)
        appearances[product].expand(display, extra, model_fdid)
        appearances[product].flush_items()


def seed_displays(graph, displays):
    displays = sorted(set(displays) - {0})
    if not displays:
        return
    with source_cache(graph, "creature_display.sqlite") as db:
        if db is None:
            return
        for display in displays:
            row = db.execute(
                "SELECT model_fdid,skin_fdid_0,skin_fdid_1,skin_fdid_2,skin_fdid_3,source_product FROM creature_displays WHERE display_id=?",
                (display,),
            ).fetchone()
            if row is None:
                graph.issue(
                    "missing_metadata_row", f"creature_display.sqlite display {display}"
                )
                continue
            product = row["source_product"]
            if not require_product(graph, product, f"display {display}", display):
                continue
            with source_catalog(graph, product):
                graph.add(row["model_fdid"], "m2", f"source display {display} model")
                for slot in range(4):
                    fdid = row[f"skin_fdid_{slot}"]
                    if fdid:
                        graph.add_model_texture(
                            row["model_fdid"],
                            fdid,
                            f"source display {display} texture variation {slot}",
                        )
            seed_display_appearance(graph, product, display, row["model_fdid"])


def seed_resource(graph, db, table, resource, fdid_column, reason):
    if not resource:
        return
    source = db.execute(
        "SELECT source_product FROM asset_resource_sources WHERE table_name=? AND resource_id=?",
        (table, resource),
    ).fetchone()
    product = source[0] if source else None
    if not require_product(graph, product, f"{table} resource {resource}", resource):
        return
    identity_column = (
        "model_resource_id" if table == "model_to_fdid" else "material_resource_id"
    )
    rows = db.execute(
        f"SELECT {fdid_column} FROM {table} WHERE {identity_column}=?", (resource,)
    ).fetchall()
    if not rows:
        graph.issue(
            "missing_metadata_row",
            f"{table} resource {resource}: declared source {product}, no files",
        )
    with source_catalog(graph, product):
        for row in rows:
            graph.add(row[0], "m2" if table == "model_to_fdid" else "blp", reason)


def seed_item_displays(graph, displays):
    displays = sorted(set(displays) - {0})
    if not displays:
        return
    with source_cache(graph, "outfit_links-v5.sqlite") as db:
        if db is None:
            return
        for display in displays:
            row = db.execute(
                "SELECT model_res_0,model_res_1,model_mat_res_0,model_mat_res_1 FROM display_info WHERE id=?",
                (display,),
            ).fetchone()
            if row is None:
                graph.issue(
                    "missing_metadata_row", f"outfit_links-v5.sqlite display {display}"
                )
                continue
            for resource in sorted(set(row[:2]) - {0}):
                seed_resource(
                    graph,
                    db,
                    "model_to_fdid",
                    resource,
                    "file_data_id",
                    f"source item display {display} model resource {resource}",
                )
            materials = set(row[2:]) | {
                value[0]
                for value in db.execute(
                    "SELECT material_resource_id FROM display_materials WHERE display_info_id=?",
                    (display,),
                )
            }
            for resource in sorted(materials - {0}):
                seed_resource(
                    graph,
                    db,
                    "material_textures",
                    resource,
                    "texture_fdid",
                    f"source item display {display} material resource {resource}",
                )


def owned_display(catalogs, item):
    first = {}
    for row in catalogs.rows("ItemModifiedAppearance"):
        identity, appearance = number(row, "ItemID"), number(row, "ItemAppearanceID")
        if identity and appearance:
            first.setdefault(identity, appearance)
    appearance = first.get(item)
    row = catalogs.index("ItemAppearance").get(appearance)
    if row is None:
        catalogs.graph.issue(
            "missing_metadata_row",
            f"{catalogs.graph.product} owned item {item}: source-local appearance unavailable",
        )
        return None
    return number(row, "ItemDisplayInfoID")


def seed_items(graph, items):
    groups = {source: set() for source in ITEM_SOURCES}
    for value in items:
        item = value["id"] if isinstance(value, dict) else value
        source = value.get("definition_source") if isinstance(value, dict) else None
        if source not in groups:
            graph.issue(
                "missing_item_definition_source",
                f"owned item {item}: explicit ItemDefinitionSource required, got {source!r}",
                item,
            )
            continue
        groups[source].add(item)
    from closure_seeds import seed_item_display_rows

    for source, wanted in groups.items():
        if not wanted:
            continue
        with source_catalog(graph, ITEM_SOURCES[source], owned_items=True) as catalogs:
            displays = {owned_display(catalogs, item) for item in sorted(wanted)} - {
                None,
                0,
            }
            seed_item_display_rows(graph, catalogs, displays)
