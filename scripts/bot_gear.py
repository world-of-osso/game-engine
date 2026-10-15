"""Source-backed normal equipment fixtures; never modify item/stat definitions."""
import shlex
import sqlite3
from collections import Counter
from contextlib import closing
from pathlib import Path

from bot_rotation import read_rows

# Source item references identify ChrSpecialization's preferred weapon kinds,
# not the items equipped. Every equipped slot uses the same authored budget.
FIXTURE_BUDGET = {"item_level": 30, "quality": 2, "item_squish_era": 0}
ARMOR_SLOTS = {"Head": (1,), "Shoulder": (3,), "Chest": (5, 20), "Waist": (6,),
               "Legs": (7,), "Feet": (8,), "Wrist": (9,), "Hands": (10,), "Back": (16,)}
PRIMARY_MODIFIERS = {3: {3, 71, 72, 73}, 4: {4, 71, 72, 74}, 5: {5, 71, 73, 74}}
PRIMARY_LABELS = {3: "Agility:", 4: "Strength:", 5: "Intellect:"}
WEAPON_REFERENCES = {
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
    budget = (sparse["ItemLevel"], sparse["OverallQualityID"], sparse["ItemSquishEraID"])
    if budget != (30, 2, 0):
        raise ValueError(f"Source gear item {item_id} is outside fixture budget {FIXTURE_BUDGET}")
    return {"slot": slot, "item_id": item_id, "name": sparse["Display_lang"],
            "inventory_type": sparse["InventoryType"], "item_level": sparse["ItemLevel"],
            "quality": sparse["OverallQualityID"], "delay_ms": sparse["ItemDelay"],
            "source": {"Item": dict(item), "ItemSparse": dict(sparse)}}


def read_spec_context(db2, spec):
    if spec not in WEAPON_REFERENCES:
        raise ValueError(f"No sourced gear fixture for specialization {spec}")
    specialization = next((row for row in read_rows(db2, "ChrSpecialization") if int(row["ID"]) == spec), None)
    if specialization is None:
        raise ValueError(f"No source specialization {spec}")
    source_class = next(row for row in read_rows(db2, "ChrClasses") if row["ID"] == specialization["ClassID"])
    # TC Player::GetPrimaryStat, also used by native SpecTabInfo::primary_stat.
    priority = int(specialization["PrimaryStatPriority"])
    primary = 4 if priority >= 4 else 3 if priority >= 2 else 5
    armor = max(kind for kind in range(1, 5) if int(source_class["ArmorTypeMask"]) & (1 << kind))
    return specialization, source_class, primary, armor


def matches_primary(row, primary):
    return any(row[f"StatModifier_bonusStat_{index}"] in PRIMARY_MODIFIERS[primary]
               and row[f"StatPercentEditor_{index}"] > 0 for index in range(10))


def read_budget_candidates(connection, level, class_id, primary):
    rows = connection.execute(
        'SELECT s.*, i.ClassID AS item_class, i.SubclassID AS item_subclass '
        'FROM item_sparse s JOIN item i ON i.ID=s.ID '
        'WHERE s.ItemLevel=? AND s.OverallQualityID=? AND s.ItemSquishEraID=? '
        'AND s.RequiredLevel<=? AND s.PlayerLevelToItemLevelCurveID=0 '
        'AND s.ItemLevelOffsetCurveID=0 AND s.ItemSet=0 ORDER BY s.ID',
        (FIXTURE_BUDGET["item_level"], FIXTURE_BUDGET["quality"], FIXTURE_BUDGET["item_squish_era"], level))
    allowed_class = 1 << (class_id - 1)
    return [row for row in rows
            if (row["AllowableClass"] < 0 or row["AllowableClass"] & allowed_class)
            and row["AllowableRaces_0"] == -1 and matches_primary(row, primary)]


def select_budget_item(candidates, slot, item_class, subclass, inventories):
    for row in candidates:
        if (row["item_class"], row["item_subclass"]) == (item_class, subclass) and row["InventoryType"] in inventories:
            return row["ID"]
    raise ValueError(f"{slot}: no matching source budget {FIXTURE_BUDGET} for class {item_class}/subclass {subclass}")


def read_weapon_kind(connection, reference, slot):
    row = connection.execute('SELECT i.ClassID, i.SubclassID, s.InventoryType FROM item i '
                             'JOIN item_sparse s ON s.ID=i.ID WHERE i.ID=?', (reference,)).fetchone()
    if row is None:
        raise ValueError(f"Missing source weapon-kind reference {reference}")
    inventories = (row["InventoryType"],)
    if row["ClassID"] == 2 and row["InventoryType"] in (13, 21, 22):
        # Native inventory types: generic one-hand plus the requested hand-only type.
        inventories = (13, 21) if slot == "MainHand" else (13, 22)
    return row["ClassID"], row["SubclassID"], inventories


def read_fixture_items(connection, candidates, spec, armor):
    armor_ids = [(slot, select_budget_item(candidates, slot, 4, 1 if slot == "Back" else armor, inventories))
                 for slot, inventories in ARMOR_SLOTS.items()]
    weapon_ids = []
    for slot, reference in zip(("MainHand", "OffHand"), WEAPON_REFERENCES[spec]):
        kind = read_weapon_kind(connection, reference, slot)
        weapon_ids.append((slot, select_budget_item(candidates, slot, *kind)))
    return [load_item(connection, identity, slot) for slot, identity in armor_ids + weapon_ids]


def load_plan(database, db2, spec, level=32):
    specialization, source_class, primary, armor = read_spec_context(db2, spec)
    with closing(sqlite3.connect(Path(database).resolve().as_uri() + "?mode=ro", uri=True)) as connection:
        connection.row_factory = sqlite3.Row
        candidates = read_budget_candidates(connection, level, int(source_class["ID"]), primary)
        items = read_fixture_items(connection, candidates, spec, armor)
    return {"spec": spec, "items": items, "source_specialization": specialization,
            "source_class": source_class, "primary_stat": primary, "armor_subclass": armor,
            "fixture_budget": FIXTURE_BUDGET.copy(),
            "policy": "lowest eligible source ID per preferred kind; same quality/item-level/squish-era; native class stats unchanged",
            "known_boundary": "Fury dual two-hand equipment requires unmodelled Titans Grip; do not substitute one-hand weapons" if spec == 72 else None}


def equipment_receipt(plan, native, sheet_stats):
    label = PRIMARY_LABELS[plan["primary_stat"]]
    if not sheet_stats.get(label):
        raise ValueError(f"Native primary stat {label} was not observed")
    expected = {item["slot"]: item for item in plan["items"]}
    equipped = {item["slot"]: item for item in native["equipment"]}
    extra = set(equipped) - set(expected) - {"Shirt", "Tabard"}
    if extra:
        raise ValueError(f"Unbudgeted equipment: {', '.join(sorted(extra))}")
    for slot, item in expected.items():
        actual = equipped.get(slot)
        if actual is None or actual["item_id"] != item["item_id"]:
            raise ValueError(f"Native {slot} does not match source budget item {item['item_id']}")
    items = [{"slot": slot, "item_id": item["item_id"], "item_level": item["item_level"],
              "quality": item["quality"], "item_guid": equipped[slot]["item_guid"]}
             for slot, item in expected.items()]
    return {"spec": plan["spec"], "budget": plan["fixture_budget"], "items": items,
            "primary_stat": plan["primary_stat"], "armor_subclass": plan["armor_subclass"],
            "native_stats": sheet_stats}


def required_grants(plan, equipment):
    equipped = {}
    for line in equipment.splitlines():
        if not line.startswith("slot="):
            continue
        fields = dict(part.split("=", 1) for part in shlex.split(line))
        equipped[fields["slot"]] = int(fields["item_id"])
    return dict(Counter(item["item_id"] for item in plan["items"]
                        if equipped.get(item["slot"]) != item["item_id"]))
