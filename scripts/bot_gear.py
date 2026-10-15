"""Source-backed normal equipment fixtures; never modify item/stat definitions."""
import shlex
import sqlite3
from collections import Counter
from contextlib import closing
from pathlib import Path

from bot_rotation import read_rows

# ChrSpecialization preferred weapon descriptions; all selected ItemSparse rows
# are authentic quality1/item-level1 starter items, not synthetic stat budgets.
KITS = {
    62: (35,), 63: (35,), 64: (35,),
    65: (36, 2362), 66: (36, 2362), 70: (2361,),
    71: (2361,), 72: (2361, 2361), 73: (20977, 2362),
    102: (35,), 103: (35,), 104: (35,), 105: (35,),
    250: (23346,), 251: (20977, 20977), 252: (23346,),
    253: (2504,), 254: (2504,), 255: (57243,),
    256: (35,), 257: (35,), 258: (35,),
    259: (2092, 2092), 260: (20977, 20977), 261: (2092, 2092),
    262: (36, 2362), 263: (20981, 20981), 264: (36, 2362),
    265: (35,), 266: (35,), 267: (35,),
    268: (35,), 269: (20981, 20981), 270: (35,),
    577: (20977, 20977), 581: (20977, 20977),
    1467: (35,), 1468: (35,), 1473: (35,), 1480: (20977, 20977),
}


def load_item(connection, item_id, slot):
    item = connection.execute("SELECT * FROM item WHERE ID=?", (item_id,)).fetchone()
    sparse = connection.execute("SELECT * FROM item_sparse WHERE ID=?", (item_id,)).fetchone()
    if item is None or sparse is None:
        raise ValueError(f"Source gear item {item_id} is absent from Item/ItemSparse")
    if sparse["OverallQualityID"] != 1 or sparse["ItemLevel"] != 1:
        raise ValueError(f"Source gear item {item_id} is outside the common starter fixture")
    return {"slot": slot, "item_id": item_id, "name": sparse["Display_lang"],
            "inventory_type": sparse["InventoryType"], "item_level": sparse["ItemLevel"],
            "quality": sparse["OverallQualityID"], "delay_ms": sparse["ItemDelay"],
            "source": {"Item": dict(item), "ItemSparse": dict(sparse)}}


def load_plan(database, db2, spec):
    if spec not in KITS:
        raise ValueError(f"No sourced gear fixture for specialization {spec}")
    specialization = next((row for row in read_rows(db2, "ChrSpecialization") if int(row["ID"]) == spec), None)
    if specialization is None:
        raise ValueError(f"No source specialization {spec}")
    with closing(sqlite3.connect(Path(database).resolve().as_uri() + "?mode=ro", uri=True)) as connection:
        connection.row_factory = sqlite3.Row
        items = [load_item(connection, item_id, slot)
                 for slot, item_id in zip(("MainHand", "OffHand"), KITS[spec])]
    return {"spec": spec, "items": items, "source_specialization": specialization,
            "policy": "source-preferred weapon kinds; common quality1/item-level1 starter budget; no stat tuning",
            "known_boundary": "Fury dual two-hand equipment requires unmodelled Titans Grip; do not substitute one-hand weapons" if spec == 72 else None}


def required_grants(plan, equipment):
    equipped = {}
    for line in equipment.splitlines():
        if not line.startswith("slot="):
            continue
        fields = dict(part.split("=", 1) for part in shlex.split(line))
        equipped[fields["slot"]] = int(fields["item_id"])
    return dict(Counter(item["item_id"] for item in plan["items"]
                        if equipped.get(item["slot"]) != item["item_id"]))
