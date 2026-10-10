"""Configurable content roots, separate from the fixed-point byte traversal.

Unresolved joins remain audit errors. Tables and legacy cache files are read only;
metadata source/build and authenticated byte provenance are distinct concepts.
"""
import csv
import hashlib
import json
import re
import sqlite3
from collections import defaultdict


def number(row, key):
    value = row.get(key)
    return int(value) if value not in (None, "") else 0


def readonly(path):
    conn = sqlite3.connect(path.resolve().as_uri() + "?mode=ro", uri=True)
    conn.row_factory = sqlite3.Row
    conn.execute("BEGIN")
    return conn


class Catalogs:
    def __init__(self, graph):
        self.graph = graph
        self.tables = {}

    def rows(self, name):
        if name not in self.tables:
            overrides = self.graph.seeds.get("table_paths", {})
            if name in overrides:
                path = self.graph.data / overrides[name]
                self.graph.issue("unverified_metadata_build", f"{name}: explicit legacy source {overrides[name]}")
            else:
                path = self.graph.data / "db2" / self.graph.metadata_build / (name + ".csv")
            if not path.is_file():
                self.graph.issue("missing_metadata_file", str(path.relative_to(self.graph.data)))
                self.tables[name] = []
            else:
                self.graph.input_file(path)
                with path.open(encoding="utf-8-sig", newline="") as stream:
                    self.tables[name] = list(csv.DictReader(stream))
        return self.tables[name]

    def index(self, name, key="ID"):
        return {number(row, key): row for row in self.rows(name)}

    def required(self, name, key):
        row = self.index(name).get(key)
        if row is None:
            self.graph.issue("missing_metadata_row", f"{name} ID={key}")
        return row


def seed_displays(graph, catalogs, displays):
    models = catalogs.index("CreatureModelData")
    info = catalogs.index("CreatureDisplayInfo")
    for display in sorted(set(displays)):
        row = info.get(display)
        if row is None:
            graph.issue("missing_metadata_row", f"CreatureDisplayInfo ID={display}")
            continue
        model_id = number(row, "ModelID")
        model = models.get(model_id)
        if not model or not number(model, "FileDataID"):
            graph.issue("missing_metadata_row", f"CreatureModelData ID={model_id} display={display}")
        else:
            graph.add(number(model, "FileDataID"), "m2", f"display {display} -> CreatureModelData {model_id}")
        for key, value in row.items():
            if key.startswith("TextureVariationFileDataID_") and int(value):
                graph.add(int(value), "blp", f"display {display} {key}")
        if number(row, "ExtendedDisplayInfoID"):
            graph.issue("display_extended_appearance", f"display {display} extended={row['ExtendedDisplayInfoID']}; cache coverage must be authenticated")


def seed_items(graph, catalogs, db, item_ids):
    displays = set()
    for item in sorted(set(item_ids)):
        row = db.execute("SELECT DisplayInfoID FROM content_item WHERE ID=?", (item,)).fetchone()
        if row is None:
            graph.issue("missing_metadata_row", f"content_item ID={item}")
        elif row[0]:
            displays.add(int(row[0]))
    info = catalogs.index("ItemDisplayInfo")
    model_rows = defaultdict(list)
    texture_rows = defaultdict(list)
    material_rows = defaultdict(set)
    for row in catalogs.rows("ModelFileData"):
        model_rows[number(row, "ModelResourcesID")].append(row)
    for row in catalogs.rows("TextureFileData"):
        texture_rows[number(row, "MaterialResourcesID")].append(row)
    for row in catalogs.rows("ItemDisplayInfoMaterialRes"):
        material_rows[number(row, "ItemDisplayInfoID")].add(number(row, "MaterialResourcesID"))
    for display in sorted(displays):
        row = info.get(display)
        if row is None:
            graph.issue("missing_metadata_row", f"ItemDisplayInfo ID={display}")
            continue
        models = {number(row, key) for key in row if key.startswith("ModelResourcesID_")} - {0}
        materials = {number(row, key) for key in row if key.startswith("ModelMaterialResourcesID_")} - {0}
        materials.update(material_rows[display])
        for resource in sorted(models):
            matches = model_rows[resource]
            if not matches:
                graph.issue("missing_metadata_row", f"item display {display} ModelResourcesID={resource}")
            for match in matches:
                graph.add(number(match, "FileDataID"), "m2", f"item display {display} ModelResourcesID {resource} (all component variants)")
        for resource in sorted(materials - {0}):
            matches = texture_rows[resource]
            if not matches:
                graph.issue("missing_metadata_row", f"item display {display} MaterialResourcesID={resource}")
            for match in matches:
                graph.add(number(match, "FileDataID"), "blp", f"item display {display} MaterialResourcesID {resource}")


def seed_icons(graph, catalogs, spells):
    wanted = set(spells)
    found = set()
    for row in catalogs.rows("SpellMisc"):
        spell = number(row, "SpellID")
        if spell not in wanted or number(row, "DifficultyID") != 0:
            continue
        found.add(spell)
        for key in ["SpellIconFileDataID", "ActiveIconFileDataID"]:
            graph.add(number(row, key), "blp", f"spell {spell} SpellMisc {row['ID']} {key}")
    for spell in sorted(wanted - found):
        graph.issue("missing_metadata_row", f"SpellMisc SpellID={spell} DifficultyID=0")


def seed_terrain(graph, config):
    selected = set()
    tiles_by_directory = defaultdict(set)
    pattern = re.compile(r"world/maps/([^/]+)/([^/]+)_(\d+)_(\d+)\.adt$")
    for path in graph.paths.values():
        match = pattern.fullmatch(path)
        if match and match[1] == match[2]:
            tiles_by_directory[match[1]].add((int(match[3]), int(match[4])))
    for map_spec in config["maps"]:
        directory = map_spec["directory"].lower()
        tiles = map_spec["tiles"]
        if tiles == "all":
            tiles = sorted(tiles_by_directory[directory])
            for kind in ["wdt", "wdl"]:
                logical = f"world/maps/{directory}/{directory}.{kind}"
                if kind == "wdt" and "wdt_fdid" in map_spec:
                    graph.add(map_spec["wdt_fdid"], "wdt", f"Map {map_spec['id']} WdtFileDataID")
                    if not map_spec["wdt_fdid"]:
                        graph.resolve("map_declared_wdt", None, "not_needed", f"Map {map_spec['id']} WdtFileDataID=0; native read_fdid_file rejects zero before file IO, not a supported-map certification")
                elif logical in graph.by_path:
                    graph.named(logical, kind, f"map {map_spec['id']} {kind}", None)
        for x, y in tiles:
            selected.add((map_spec["id"], x, y))
            for suffix in ["", "_tex0", "_obj0", "_obj1"]:
                logical = f"world/maps/{directory}/{directory}_{x}_{y}{suffix}.adt"
                # obj1 is not present in every supported map's format. Enumerate it
                # only when authored; root/tex0/obj0 are required for this slice.
                if suffix == "_obj1" and logical not in graph.by_path:
                    continue
                graph.named(logical, "adt", f"map {map_spec['id']} tile {x},{y}{suffix}", None)
    return selected


def npc_displays(db, tiles):
    spawns = []
    for map_id, x, y in sorted(tiles):
        spawns.extend(dict(row) for row in db.execute(
            "SELECT guid,id1,id2,id3,modelid,position_x,position_y FROM content_creature "
            "WHERE map=? AND position_x>? AND position_x<=? AND position_y>? AND position_y<=? ORDER BY guid",
            (map_id, (31 - y) * (1600 / 3), (32 - y) * (1600 / 3),
             (31 - x) * (1600 / 3), (32 - x) * (1600 / 3))))
    displays = set()
    for spawn in spawns:
        if spawn["modelid"]:
            displays.add(spawn["modelid"])
        for key in ["id1", "id2", "id3"]:
            if spawn[key]:
                displays.update(row[0] for row in db.execute(
                    "SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=? ORDER BY Idx", (spawn[key],)))
    return spawns, sorted(displays - {0})


def starting_items(db, character):
    race_bit = 1 << (character["race"] - 1)
    items = [row[0] for row in db.execute(
        "SELECT li.ItemID FROM character_loadout l JOIN character_loadout_item li ON li.CharacterLoadoutID=l.ID "
        "JOIN content_item i ON i.ID=li.ItemID WHERE l.Purpose=9 AND l.ChrClassID=? "
        "AND ((l.RaceMasks_0 & ?)!=0) ORDER BY l.ID,li.ID", (character["class"], race_bit))]
    for row in db.execute(
        "SELECT itemid,amount FROM tdb_playercreateinfo_item WHERE race IN (0,?) AND class IN (0,?) "
        "AND itemid IN (SELECT ID FROM content_item) ORDER BY race,class,itemid", (character["race"], character["class"])):
        if row[1] > 0:
            items.append(row[0])
        elif row[1] < 0:
            items = [item for item in items if item != row[0]]
    return sorted(set(items))


def human_warrior_spellbook(db, level):
    # Exact bounded class/race: server class_data/class_progression and client's
    # future-spell listing. Include future Warrior spells because the book shows them.
    defaults = list(db.execute(
        "SELECT r.SkillID,r.Flags,s.CategoryID,t.Value1 FROM skill_race_class_info r "
        "JOIN skill_line s ON s.ID=r.SkillID LEFT JOIN tdb_skill_tiers t ON t.ID=r.SkillTierID "
        "WHERE r.Availability=1 AND r.SkillID!=840 AND (r.ClassMask=0 OR (r.ClassMask & 1)!=0) "
        "AND r.MinLevel<=? AND ((r.RaceMasks_0=0 AND r.RaceMasks_1=0) OR (r.RaceMasks_0 & 1)!=0) "
        "ORDER BY r.SkillID,r.ID", (level,)))
    skill_values = {}
    for line, flags, category, maximum in defaults:
        if line in skill_values:
            continue
        if category == 10:
            value = 300
        elif line == 960 or category == 8:
            value = 1
        elif flags & 0x10:
            value = maximum if maximum is not None else 5 * level
        else:
            value = 1
        skill_values[line] = value
    wanted = set()
    for row in db.execute(
        "SELECT a.Spell,a.SkillLine,a.AcquireMethod,a.MinSkillLineRank,COALESCE(l.SpellLevel,0), "
        "COALESCE(m.Attributes_0,0),COALESCE(m.Attributes_4,0),COALESCE(m.Attributes_7,0),COALESCE(m.Attributes_8,0) "
        "FROM skill_line_ability a JOIN spell_name n ON n.ID=a.Spell "
        "LEFT JOIN spell_levels l ON l.SpellID=a.Spell AND l.DifficultyID=0 "
        "LEFT JOIN spell_misc m ON m.SpellID=a.Spell AND m.DifficultyID=0 "
        "WHERE a.AcquireMethod IN (1,2,4) AND (a.ClassMask=0 OR (a.ClassMask & 1)!=0) "
        "AND ((a.RaceMasks_0=0 AND a.RaceMasks_1=0) OR (a.RaceMasks_0 & 1)!=0) "
        "AND (COALESCE(m.Attributes_13,0) & 65536)=0 ORDER BY a.Spell"):
        spell, line, method, rank, required, a0, a4, a7, a8 = row
        known = required <= level and (line == 840 or (line in skill_values and (method != 1 or skill_values[line] >= rank)))
        future = line == 840 and required > level
        listed = not (a0 & 0x80 or a4 & 0x8000 or (known and a7 & 0x10000) or (future and a8 & 0x2000))
        if (known or future) and listed:
            wanted.add(spell)
    for row in db.execute(
        "SELECT s.SpellID FROM specialization_spells s JOIN chr_specialization c ON c.ID=s.SpecID "
        "LEFT JOIN spell_levels l ON l.SpellID=s.SpellID AND l.DifficultyID=0 "
        "WHERE c.ClassID=1 AND c.OrderIndex=4 AND COALESCE(l.SpellLevel,0)<=? ORDER BY s.ID", (level,)):
        wanted.add(row[0])
    return sorted(wanted)


def requirement_allows(graph, requirements, requirement_id, race, class_id):
    if requirement_id == 0:
        return True
    row = requirements.get(requirement_id)
    if row is None:
        graph.issue("missing_customization_requirement", f"requirement {requirement_id}")
        return False
    class_mask = number(row, "ClassMask")
    race_masks = [number(row, "RaceMasks_0"), number(row, "RaceMasks_1")]
    # Bounded Human race bit follows customization_catalog.rs/support. Full
    # catalog mode enumerates all choices, so it does not prune by this mask.
    race_bit = race - 1
    race_allowed = race_masks == [0, 0] or (race_bit < 64 and race_masks[race_bit // 32] & (1 << (race_bit % 32)))
    unlocked = not any(number(row, key) for key in ["ReqAchievementID", "ReqQuestID", "ReqItemModifiedAppearanceID", "RegionGroupMask"])
    class_allowed = class_mask == 0 or class_mask & (1 << (class_id - 1))
    return bool(number(row, "ReqType") & 1 and unlocked and class_allowed and race_allowed)


def seed_customizations(graph, chr_model, selector, race=1, class_id=1):
    path = graph.data / "cache/customization.sqlite"
    if not path.is_file():
        graph.issue("missing_metadata_file", "cache/customization.sqlite")
        return []
    graph.input_file(path)
    graph.issue("unverified_cache_build", "customization.sqlite: source mtimes do not authenticate build")
    requirements = Catalogs(graph).index("ChrCustomizationReq") if selector != "all" else {}
    with readonly(path) as db:
        options = list(db.execute("SELECT id,requirement_id FROM options WHERE chr_model_id=? ORDER BY order_index,id", (chr_model,)))
        choices = []
        for option, requirement in options:
            if selector != "all" and not requirement_allows(graph, requirements, requirement, race, class_id):
                continue
            rows = list(db.execute("SELECT id,requirement_id,visibility_requirement_id FROM choices WHERE option_id=? ORDER BY order_index,id", (option,)))
            if selector == "all":
                choices.extend(row[0] for row in rows)
            elif selector == "default":
                eligible = [row[0] for row in rows if requirement_allows(graph, requirements, row[1], race, class_id) and row[2] == 0]
                if eligible:
                    choices.append(eligible[0])
                elif rows:
                    graph.issue("customization_requirement", f"model {chr_model} option {option}: no eligible player choice")
            else:
                choices.extend(row[0] for row in rows if row[0] in selector)
        apply_choices(graph, db, choices, "all" if selector == "all" else "selected")
    return choices


def apply_choices(graph, db, choices, mode):
    selected = set(choices)
    for choice in sorted(selected):
        for row in db.execute("SELECT related_choice_id,material_id,skinned_model_id,has_unsupported_effects FROM elements WHERE choice_id=?", (choice,)):
            related, material, model, unsupported = row
            if related and related not in selected and mode != "all":
                continue
            for (fdid,) in db.execute("SELECT t.file_data_id FROM materials m JOIN texture_fdids t ON t.material_resources_id=m.material_resources_id WHERE m.id=?", (material,)):
                graph.add(fdid, "blp", f"customization choice {choice} material {material}")
            for (fdid,) in db.execute("SELECT collection_fdid FROM skinned_models WHERE id=?", (model,)):
                graph.add(fdid, "m2", f"customization choice {choice} skinned model {model}")
            if unsupported:
                graph.issue("unsupported_customization_effect", f"choice {choice}")


def seed_npc_appearance(graph, displays):
    path = graph.data / "cache/npc_appearance.sqlite"
    if not path.is_file():
        graph.issue("missing_metadata_file", "cache/npc_appearance.sqlite")
        return
    graph.input_file(path)
    graph.issue("unverified_cache_build", "npc_appearance.sqlite: no authenticated source receipt")
    customization = graph.data / "cache/customization.sqlite"
    with readonly(path) as db:
        choice_ids = []
        for display in displays:
            for (texture,) in db.execute("SELECT baked_texture_fdid FROM appearances WHERE display_id=?", (display,)):
                graph.add(texture, "blp", f"NPC display {display} baked appearance")
            choice_ids.extend(row[0] for row in db.execute("SELECT choice_id FROM choices WHERE display_id=?", (display,)))
        if choice_ids and customization.is_file():
            with readonly(customization) as choices:
                apply_choices(graph, choices, choice_ids, "selected")
        elif choice_ids:
            graph.issue("missing_metadata_file", "NPC choices need cache/customization.sqlite")


def seed_catalogs(graph, world_path, config):
    catalogs = Catalogs(graph)
    graph.seeds = dict(config)
    maps = config["maps"]
    if maps == "all":
        maps = [{"id": number(row, "ID"), "directory": row["Directory"], "tiles": "all", "wdt_fdid": number(row, "WdtFileDataID")}
                for row in catalogs.rows("Map") if row.get("Directory")]
    tiles = seed_terrain(graph, {"maps": maps})
    race_models = catalogs.rows("ChrRaceXChrModel")
    characters = config["characters"]
    if characters == "all":
        graph.issue("playable_character_reachability", "all modeled race/sex pairs include nonplayable placeholders; supported-pair policy required")
        characters = [{"race": number(row, "ChrRacesID"), "sex": number(row, "Sex"), "choices": "all"}
                      for row in race_models]
    player_displays = set()
    chr_models = catalogs.index("ChrModel")
    for character in characters:
        rows = [r for r in race_models if number(r, "ChrRacesID") == character["race"] and number(r, "Sex") == character["sex"]]
        if not rows:
            graph.issue("missing_metadata_row", f"ChrRaceXChrModel race={character['race']} sex={character['sex']}")
        for row in rows:
            model_id = number(row, "ChrModelID")
            model = chr_models.get(model_id)
            if not model:
                graph.issue("missing_metadata_row", f"ChrModel ID={model_id}")
                continue
            player_displays.add(number(model, "DisplayID"))
            graph.add(number(model, "SkeletonFileDataID"), "skel", f"ChrModel {model_id} SkeletonFileDataID")
            character["selected_choices"] = seed_customizations(graph, model_id, character["choices"], character["race"], character.get("class", 1))
    with readonly(world_path) as db:
        if config["npc_displays"] == "all":
            spawns = [dict(row) for row in db.execute(
                "SELECT guid,id1,id2,id3,modelid,position_x,position_y FROM content_creature ORDER BY guid")]
            npc_ids = sorted(({row[0] for row in db.execute("SELECT CreatureDisplayID FROM content_creature_template_model")} | {row["modelid"] for row in spawns}) - {0})
        else:
            spawns, npc_ids = npc_displays(db, tiles)
        items = config["items"]
        if items == "all":
            items = [row[0] for row in db.execute("SELECT ID FROM content_item ORDER BY ID")]
        elif items == "starting":
            items = sorted({item for character in characters for item in starting_items(db, character)})
            graph.issue("world_loadout_source_identity", "world.db lacks/does not validate per-row product receipts; loadout Purpose=9 union audited, not authenticated")
        spells = config["spells"]
        if spells == "all":
            spells = [row[0] for row in db.execute("SELECT ID FROM spell_name ORDER BY ID")]
            graph.issue("spell_reachability_superset", "all spell_name rows, not a pruned server-reachable set")
        elif spells == "spellbook":
            if any((c["race"], c["class"]) != (1, 1) for c in characters):
                raise ValueError("bounded spellbook selector is Human Warrior only; use explicit spells or all")
            spells = sorted({spell for character in characters for spell in human_warrior_spellbook(db, character["level"])})
        seed_items(graph, catalogs, db, items)
        selected = {"spawns": spawns, "displays": npc_ids, "items": items, "spells": spells}
        graph.seeds["world_selection"] = selected
        graph.seeds["world_selection_sha256"] = hashlib.sha256(json.dumps(selected, sort_keys=True).encode()).hexdigest()
    seed_displays(graph, catalogs, sorted(player_displays | set(npc_ids)))
    seed_npc_appearance(graph, npc_ids)
    seed_icons(graph, catalogs, spells)
    if config["spells"] == "all":
        from closure_spell_seeds import seed_spell_visuals
        seed_spell_visuals(graph, catalogs, spells)
