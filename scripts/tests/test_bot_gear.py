import json
import sqlite3
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bot_gear
import bot_roster

FIXTURES = Path(__file__).parent / "fixtures/bot_harness"


class BotGearTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.database = Path(self.directory.name) / "world.db"
        rows = json.loads((FIXTURES / "gear-source-rows.json").read_text())
        with closing(sqlite3.connect(self.database)) as connection:
            for table, records in rows.items():
                columns = list(records[0])
                column_names = ",".join(f'"{key}"' for key in columns)
                placeholders = ",".join("?" for _ in columns)
                connection.execute(f'CREATE TABLE {table} ({column_names})')
                connection.executemany(f'INSERT INTO {table} VALUES ({placeholders})',
                                       [[record[column] for column in columns] for record in records])
            connection.commit()

    def test_retribution_has_source_two_hand_weapon_and_equal_budget_plate(self):
        plan = bot_gear.load_plan(self.database, FIXTURES / "db2", 70)
        items = {item["slot"]: item for item in plan["items"]}
        self.assertIn("Head", items, "source armor must replace unequal starter kits")
        self.assertEqual(items["Head"]["item_id"], 24930)
        self.assertEqual(items["Head"]["source"]["Item"]["SubclassID"], 4)
        item = items["MainHand"]
        self.assertEqual(item["item_id"], 25129)
        self.assertEqual(item["source"]["ItemSparse"]["Display_lang"], "Giant's Leg Bone")
        self.assertEqual((item["inventory_type"], item["item_level"], item["quality"]), (17, 30, 2))
        self.assertNotIn("OffHand", items)
        self.assertIn("Two-Handed", plan["source_specialization"]["Description_lang"])

    def test_fury_requires_both_source_two_hand_weapons_without_one_hand_fallback(self):
        plan = bot_gear.load_plan(self.database, FIXTURES / "db2", 72)
        weapons = [item for item in plan["items"] if item["slot"] in ("MainHand", "OffHand")]
        self.assertEqual([item["slot"] for item in weapons], ["MainHand", "OffHand"])
        self.assertTrue(all(item["inventory_type"] == 17 for item in weapons))
        starter = "slot=MainHand guid=116 item_id=25 name=\"Worn Shortsword\"\nslot=OffHand guid=115 item_id=2362 name=\"Worn Wooden Shield\"\n"
        expected = {24930: 1, 24932: 1, 24928: 1, 24926: 1, 24931: 1,
                    24927: 1, 24933: 1, 24929: 1, 25031: 1, 25129: 2}
        self.assertEqual(bot_gear.required_grants(plan, starter), expected)
        correct = "\n".join(f'slot={item["slot"]} guid={index} item_id={item["item_id"]}'
                            for index, item in enumerate(plan["items"], 1))
        self.assertEqual(bot_gear.required_grants(plan, correct), {})

    def test_every_installed_playable_spec_has_a_sourced_fixture(self):
        bots = bot_roster.build_roster(FIXTURES / "db2", 32)
        for bot in bots:
            plan = bot_gear.load_plan(self.database, FIXTURES / "db2", bot["spec"])
            expected = json.loads((FIXTURES / "gear-equal-budget-plans.json").read_text())["plans"][str(bot["spec"])]
            self.assertEqual(plan.get("fixture_budget"), {"item_level": 30, "quality": 2, "item_squish_era": 0})
            self.assertEqual({item["slot"]: item["item_id"] for item in plan["items"]}, expected["items"])
            self.assertEqual(plan["primary_stat"], expected["primary_stat"])
            self.assertEqual(plan["armor_subclass"], expected["armor_subclass"])
            self.assertTrue(all(item["item_level"] == 30 and item["quality"] == 2 for item in plan["items"]))
        with self.assertRaises(ValueError):
            bot_gear.load_plan(self.database, FIXTURES / "db2", 999999)

    def test_missing_plate_head_budget_blocks_instead_of_retaining_starter_armor(self):
        with closing(sqlite3.connect(self.database)) as connection:
            connection.execute('DELETE FROM item_sparse WHERE InventoryType=1 AND ID IN (SELECT ID FROM item WHERE ClassID=4 AND SubclassID=4)')
            connection.commit()
        with self.assertRaisesRegex(ValueError, "Head.*budget"):
            bot_gear.load_plan(self.database, FIXTURES / "db2", 70)

    def test_native_owner_snapshot_is_required_before_budget_receipt(self):
        plan = bot_gear.load_plan(self.database, FIXTURES / "db2", 70)
        native = {"equipment": [{"slot": item["slot"], "item_id": item["item_id"],
                                 "item_guid": index, "count": 1}
                                for index, item in enumerate(plan["items"], 100)]}
        receipt = bot_gear.equipment_receipt(plan, native, {"Strength:": "70"})
        self.assertEqual(receipt["spec"], 70)
        self.assertEqual(receipt["budget"], {"item_level": 30, "quality": 2, "item_squish_era": 0})
        self.assertEqual(receipt["items"][0]["item_level"], 30)
        native["equipment"][-1]["item_id"] = 25
        with self.assertRaisesRegex(ValueError, "MainHand"):
            bot_gear.equipment_receipt(plan, native, {"Strength:": "70"})

    def test_unbudgeted_ring_or_missing_primary_stat_blocks_comparison(self):
        plan = bot_gear.load_plan(self.database, FIXTURES / "db2", 70)
        native = {"equipment": [{"slot": item["slot"], "item_id": item["item_id"], "item_guid": index}
                                for index, item in enumerate(plan["items"], 100)]}
        with self.assertRaisesRegex(ValueError, "primary stat"):
            bot_gear.equipment_receipt(plan, native, {"Armor:": "31"})
        native["equipment"].append({"slot": "Finger1", "item_id": 50255, "item_guid": 999})
        with self.assertRaisesRegex(ValueError, "Unbudgeted.*Finger1"):
            bot_gear.equipment_receipt(plan, native, {"Strength:": "70"})

    def test_empty_jewelry_baseline_retains_four_native_storage_bags(self):
        plan = bot_gear.load_plan(self.database, FIXTURES / "db2", 577)
        self.assertEqual(plan.get("empty_slots"), ["Neck", "Finger1", "Finger2", "Trinket1", "Trinket2"])
        native = {"equipment": [{"slot": item["slot"], "item_id": item["item_id"], "item_guid": index}
                                for index, item in enumerate(plan["items"], 100)]}
        bags = [{"slot": f"Bag{index}", "item_id": 123958, "item_guid": 900 + index}
                for index in range(1, 5)]
        native["equipment"].extend(bags)
        receipt = bot_gear.equipment_receipt(plan, native, {"Agility:": "104"})
        self.assertEqual(receipt["retained_bags"], bags)
        for slot, item_id in [("Neck", 112461), ("Finger1", 112459), ("Finger2", 112460),
                              ("Trinket1", 123959), ("Trinket2", 123960)]:
            with self.subTest(slot=slot):
                leftover = {"slot": slot, "item_id": item_id, "item_guid": 999}
                native["equipment"].append(leftover)
                with self.assertRaisesRegex(ValueError, slot):
                    bot_gear.equipment_receipt(plan, native, {"Agility:": "104"})
                native["equipment"].pop()

    def test_holy_paladin_uses_plate_with_intellect_not_strength_only_armor(self):
        plan = bot_gear.load_plan(self.database, FIXTURES / "db2", 65)
        self.assertEqual(plan.get("primary_stat"), 5)
        self.assertEqual(plan.get("armor_subclass"), 4)
        for item in plan["items"]:
            sparse = item["source"]["ItemSparse"]
            self.assertTrue(any(sparse[f"StatModifier_bonusStat_{i}"] in {5, 71, 73, 74}
                                and sparse[f"StatPercentEditor_{i}"] > 0 for i in range(10)))


if __name__ == "__main__":
    unittest.main()
