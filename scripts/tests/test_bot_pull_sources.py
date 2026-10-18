"""Source aggro radius and hideable map 34 population from read-only world.db + DB2."""
import csv
import sqlite3
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_pull_sources import PullSources

SERVER_DATA = Path("/home/osso/Projects/world-of-osso/game-server/data")

EFFECT_COLUMNS = ["ID", "EffectAura", "DifficultyID", "EffectIndex", "Effect", "EffectTriggerSpell",
                  "EffectBasePointsF", "EffectMiscValue_0", "ImplicitTarget_0", "SpellID"]
FACTION_COLUMNS = ["ID", "Faction", "Flags", "FactionGroup", "FriendGroup", "EnemyGroup",
                   *[f"Enemies_{i}" for i in range(8)], *[f"Friend_{i}" for i in range(8)]]
# Installed 12.1.5.70448 FactionTemplate rows: Human player, Monster, Stormwind guard.
FACTIONS = [[1, 1, 72, 3, 2, 12] + [0] * 16,
            [16, 14, 1, 8, 0, 1] + [0] * 8 + [14] + [0] * 7,
            [35, 31, 0, 0, 1, 0] + [0] * 8 + [31] + [0] * 7]
APPLY_AURA, PROC_TRIGGER, PERIODIC_TRIGGER = 6, 42, 23
# (spell, effect, aura, trigger, base points, implicit target)
SPELLS = [
    (55964, APPLY_AURA, 18, 0, 9999, 1), (55964, APPLY_AURA, 12, 0, 1, 1),
    (55966, 252, 0, 0, 0, 1), (86726, APPLY_AURA, PERIODIC_TRIGGER, 86727, 0, 6),
    (86727, 31, 0, 0, 25, 6), (86729, 31, 0, 0, 110, 6),
    (7000, APPLY_AURA, PROC_TRIGGER, 7001, 1, 1), (7001, APPLY_AURA, 16, 0, 5, 1),
    (7100, APPLY_AURA, 18, 0, 1, 1), (7200, APPLY_AURA, PROC_TRIGGER, 18941, 0, 1),
    (7300, APPLY_AURA, 91, 0, 10, 1), (7400, APPLY_AURA, 152, 0, 10, 1),
    (7500, 35, 18, 0, 1, 1), (7600, 3, 0, 0, 0, 1), (7700, 28, 0, 0, 0, 1),
    (7800, APPLY_AURA, 70, 0, 25, 1),
]
TEMPLATE_COLUMNS = ("entry", "name", "faction", "ScriptName", "AIName", "minlevel", "maxlevel",
                    "ContentTuningID", "HealthScalingExpansion", "LevelScalingDeltaMin",
                    "LevelScalingDeltaMax", "flags_extra", "unit_flags", "type")
TEMPLATES = [
    (100, "Thug", 16, "", "", 10, 30, 215, -1, 0, 1, 0, 64, 7),
    (46383, "Randolph Moloch", 16, "boss_randolph_moloch", "", 10, 30, 215, -1, 2, 2, 0, 64, 7),
    (200, "Shifty Thief", 16, "", "", 10, 30, 215, -1, 0, 0, 0, 64, 7),
    (300, "Stockade Guard", 35, "", "", 10, 30, 215, -1, 0, 0, 0, 0, 7),
    (400, "Stalker", 16, "", "", 10, 30, 215, -1, 0, 0, 128, 0, 10),
    (500, "Smart Rogue", 16, "", "SmartAI", 10, 30, 215, -1, 0, 0, 0, 64, 7),
    (600, "Heroic Only", 16, "unported_heroic_script", "", 10, 30, 215, -1, 0, 0, 0, 64, 7),
    (700, "Untuned Brute", 16, "", "", 33, 33, 0, 0, 0, 0, 0, 64, 7),
    # Not spawned: boss_randolph_moloch summons it at death (the_stockade.rs just_died).
    (46482, "Mortimer Moloch", 35, "npc_mortimer_moloch", "", 10, 30, 215, -1, 2, 2, 0, 64, 7),
]
# entry -> (display, combat reach, display scale)
MODELS = {100: (500, 1.5, 1.0), 46383: (501, 2.0, 1.0), 200: (502, 1.5, 1.0), 300: (503, 1.5, 1.0),
          400: (504, 1.5, 1.0), 500: (505, 1.5, 1.0), 600: (506, 1.5, 1.0), 700: (507, 1.5, 1.0),
          46482: (508, 1.5, 1.0)}


def build_world(path, templates=TEMPLATES, spawns=None, template_addons=None, smart=None):
    spawns = spawns or [(1, 100, "1", 0), (2, 46383, "1", 0), (3, 200, "1", 0), (4, 300, "1", 0),
                        (5, 400, "1", 0), (6, 500, "1", 0), (7, 600, "2", 0), (8, 100, "1", 0)]
    template_addons = template_addons if template_addons is not None else {200: "7000", 300: "7100"}
    smart = smart if smart is not None else [(500, 0, 11, 7100), (500, 0, 25, 1)]
    with closing(sqlite3.connect(path)) as db, db:
        db.executescript("""
            CREATE TABLE content_creature(guid, id1, map, spawnDifficulties, modelid, ScriptName);
            CREATE TABLE content_creature_template(entry, name, faction, ScriptName, AIName, minlevel,
                maxlevel, ContentTuningID, HealthScalingExpansion, LevelScalingDeltaMin,
                LevelScalingDeltaMax, flags_extra, unit_flags, type);
            CREATE TABLE content_creature_template_difficulty(entry, DifficultyID, minlevel, maxlevel,
                ContentTuningID, HealthScalingExpansion, LevelScalingDeltaMin, LevelScalingDeltaMax);
            CREATE TABLE content_creature_template_model(CreatureID, Idx, CreatureDisplayID,
                DisplayScale, Probability);
            CREATE TABLE encounter_creature_model_info(DisplayID, CombatReach);
            CREATE TABLE content_creature_addon(guid, auras);
            CREATE TABLE content_creature_template_addon(entry, auras);
            CREATE TABLE tdb_creature_template_spell(CreatureID, "Index", Spell);
            CREATE TABLE tdb_smart_scripts(entryorguid, source_type, action_type, action_param1);
            CREATE TABLE tdb_instance_template(map, script);
            CREATE TABLE spell_name(ID);
            CREATE TABLE tdb_serverside_spell(Id);
            CREATE TABLE tdb_serverside_spell_effect(SpellID, DifficultyID, Effect, EffectAura,
                EffectTriggerSpell, EffectBasePoints, EffectMiscValue1, ImplicitTarget1);
        """)
        db.execute("INSERT INTO tdb_instance_template VALUES (34, 'instance_the_stockade')")
        db.executemany(f"INSERT INTO content_creature_template({','.join(TEMPLATE_COLUMNS)}) "
                       f"VALUES ({','.join('?' * len(TEMPLATE_COLUMNS))})", templates)
        db.executemany("INSERT INTO content_creature VALUES (?, ?, 34, ?, ?, '')", spawns)
        for entry, (display, reach, scale) in MODELS.items():
            db.execute("INSERT INTO content_creature_template_model VALUES (?, 0, ?, ?, 1)",
                       (entry, display, scale))
            db.execute("INSERT INTO encounter_creature_model_info VALUES (?, ?)", (display, reach))
        db.executemany("INSERT INTO content_creature_template_addon VALUES (?, ?)",
                       template_addons.items())
        db.executemany("INSERT INTO tdb_smart_scripts VALUES (?, ?, ?, ?)", smart)


def build_db2(directory, spells=SPELLS, missing_names=(18941,)):
    names = sorted({spell for spell, *_ in spells} | {12466} - set(missing_names))
    with open(directory / "SpellName.csv", "w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(["ID", "Name_lang"])
        writer.writerows([spell, f"Spell {spell}"] for spell in names)
    with open(directory / "SpellEffect.csv", "w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(EFFECT_COLUMNS)
        for row_id, (spell, effect, aura, trigger, points, target) in enumerate(spells, 1):
            writer.writerow([row_id, aura, 0, 0, effect, trigger, points, 0, target, spell])
    with open(directory / "FactionTemplate.csv", "w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(FACTION_COLUMNS)
        writer.writerows(FACTIONS)


TANK = {"id": 1, "entry": 0, "level": 20, "scale": 1.0, "faction": 1, "auras": [],
        "position": [0.0, 0.0, 0.0], "health": 100.0, "combat": False}


def creature(entry=100, display=500, level=32, scaling=None, scale=1.0, auras=()):
    return {"id": 9, "entry": entry, "display": display, "scale": scale, "level": level,
            "scaling": scaling, "auras": list(auras), "faction": 16, "position": [5.0, 0.0, 0.0],
            "health": 100.0, "combat": False}


class SourceFixture(unittest.TestCase):
    def sources(self, **world):
        directory = Path(self.enterContext(tempfile.TemporaryDirectory()))
        db2 = directory / "db2"
        db2.mkdir()
        build_world(directory / "world.db", **{k: v for k, v in world.items() if k != "spells"})
        build_db2(db2, **({"spells": world["spells"]} if "spells" in world else {}))
        return PullSources(directory / "world.db", db2)


class RadiusTests(SourceFixture):
    def test_center_radius_follows_server_attack_distance(self):
        sources = self.sources()
        tank32 = {**TANK, "level": 32}
        self.assertEqual(sources.radius(creature(level=32), tank32), 16.5)
        self.assertEqual(sources.radius(creature(level=30), tank32), 14.5)
        self.assertEqual(sources.radius(creature(entry=46383, display=501, scale=1.5), tank32), 16.5)
        self.assertEqual(sources.radius(creature(level=90), {**TANK, "level": 1}), 48.0)

    def test_level_scaling_uses_viewer_level_with_delta(self):
        scaling = {"min_level": 10, "max_level": 30, "delta": 2, "content_tuning_id": 215}
        self.assertEqual(self.sources().radius(creature(level=32, scaling=scaling), TANK), 18.5)

    def test_untuned_difficulty_expansion_caps_above_expansion_level(self):
        tank32 = {**TANK, "level": 32}
        self.assertEqual(self.sources().radius(creature(entry=700, display=507, level=33), tank32), 14.5)

    def test_tank_scale_widens_target_reach(self):
        self.assertEqual(self.sources().radius(creature(level=32), {**TANK, "level": 32, "scale": 2.0}), 18.0)

    def test_unknown_inputs_and_detection_range_auras_fail_closed(self):
        sources = self.sources()
        for bad_creature, tank in [
            (creature(display=999), TANK),
            (creature(entry=999), TANK),
            (creature(auras=[{"spell_id": 424242, "stacks": 1}]), TANK),
            (creature(auras=[{"spell_id": 7300, "stacks": 1}]), TANK),
            (creature(), {**TANK, "auras": [{"spell_id": 7400, "stacks": 1}]}),
        ]:
            with self.subTest(creature=bad_creature, tank=tank), self.assertRaises(ValueError):
                sources.radius(bad_creature, tank)

    def test_irrelevant_known_auras_do_not_change_radius(self):
        tank32 = {**TANK, "level": 32, "auras": [{"spell_id": 7800, "stacks": 2}]}
        enraged = creature(auras=[{"spell_id": 7800, "stacks": 1}])
        self.assertEqual(self.sources().radius(enraged, tank32), 16.5)


class PopulationTests(SourceFixture):
    def test_population_lists_hostile_aggressive_entries_and_hide_sources(self):
        population = self.sources().population(TANK)
        self.assertEqual(population["entries"], [100, 200, 500, 46383])
        self.assertEqual(population["hideable_entries"], [200, 500, 46383])
        self.assertEqual(population["sources"][46383]["hide_spells"], [55964])
        self.assertEqual(population["sources"][200]["hide_spells"], [7001])
        self.assertEqual(population["sources"][500]["hide_spells"], [7100])
        self.assertEqual(population["sources"][100]["hide_spells"], [])
        self.assertEqual(population["max_radius"], 18.5)
        self.assertEqual(population["sources"][100]["max_radius"], 17.5)
        self.assertEqual(population["map"], 34)
        self.assertEqual(population["difficulty"], 1)

    def test_spawn_display_override_is_unscaled_and_needs_source_reach(self):
        spawns = [(1, 100, "1", 777)]
        with self.assertRaisesRegex(ValueError, "777"):
            self.sources(spawns=spawns).population(TANK)

    def test_trigger_spell_absent_from_every_server_store_is_recorded_not_hidden(self):
        population = self.sources(template_addons={200: "7200"}).population(TANK)
        self.assertNotIn(200, population["hideable_entries"])
        self.assertEqual(population["absent_spells"], [18941])

    def test_addon_spell_absent_from_every_server_store_cannot_hide(self):
        population = self.sources(template_addons={200: "424242"}).population(TANK)
        self.assertNotIn(200, population["hideable_entries"])
        self.assertEqual(population["absent_spells"], [424242])

    def test_script_summons_join_the_candidate_population(self):
        templates = [row if row[0] != 46482 else (46482, "Mortimer Moloch", 16, "npc_mortimer_moloch", "",
                                                  10, 30, 215, -1, 2, 2, 0, 64, 7) for row in TEMPLATES]
        population = self.sources(templates=templates).population(TANK)
        self.assertIn(46482, population["entries"])
        self.assertNotIn(46482, population["hideable_entries"])

    def test_unsupported_source_semantics_fail_closed(self):
        def thug(script, ai):
            return [row if row[0] != 100 else (100, "Thug", 16, script, ai, *row[5:]) for row in TEMPLATES]
        unknown_script, other_ai = thug("boss_unknown", ""), thug("", "CombatAI")
        cases = {
            "unknown script": {"templates": unknown_script},
            "unknown AIName": {"templates": other_ai},
            "unknown smart action": {"smart": [(500, 0, 75, 7100)]},
            "smart summon": {"smart": [(500, 0, 12, 100)]},
            "area hide aura": {"template_addons": {200: "7500"}},
            "dummy without source authority": {"template_addons": {200: "7600"}},
            "summon effect": {"template_addons": {200: "7700"}},
            "creature detect range aura": {"template_addons": {200: "7300"}},
        }
        for label, world in cases.items():
            with self.subTest(label), self.assertRaises(ValueError):
                self.sources(**world).population(TANK)

    def test_unknown_tank_faction_fails_closed(self):
        with self.assertRaises(ValueError):
            self.sources().population({**TANK, "faction": 999999})

    def test_stockade_instance_script_is_required_for_ported_boss_scripts(self):
        directory = Path(self.enterContext(tempfile.TemporaryDirectory()))
        build_world(directory / "world.db")
        with closing(sqlite3.connect(directory / "world.db")) as db, db:
            db.execute("UPDATE tdb_instance_template SET script = ''")
        (directory / "db2").mkdir()
        build_db2(directory / "db2")
        with self.assertRaisesRegex(ValueError, "instance_the_stockade"):
            PullSources(directory / "world.db", directory / "db2").population(TANK)

    def test_world_db_is_opened_read_only(self):
        sources = self.sources()
        with self.assertRaises(sqlite3.OperationalError):
            sources.db.execute("CREATE TABLE written(x)")


@unittest.skipUnless((SERVER_DATA / "world.db").exists() and (SERVER_DATA / "db2/active").exists(),
                     "installed game-server source data")
class InstalledStockadeTests(unittest.TestCase):
    def test_installed_stockade_population_hides_only_moloch_vanish(self):
        sources = PullSources(SERVER_DATA / "world.db", SERVER_DATA / "db2/active")
        population = sources.population(TANK)
        self.assertEqual(population["hideable_entries"], [46383])
        self.assertEqual(population["sources"][46383]["hide_spells"], [55964])
        self.assertIn(18941, population["absent_spells"])
        self.assertIn(46263, population["entries"])
        self.assertLessEqual(population["max_radius"], 48.0 + 1.5)


if __name__ == "__main__":
    unittest.main()
