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

    def test_retribution_has_authentic_two_hand_item_not_starter_shield(self):
        plan = bot_gear.load_plan(self.database, FIXTURES / "db2", 70)
        self.assertEqual([(item["slot"], item["item_id"]) for item in plan["items"]], [("MainHand", 2361)])
        item = plan["items"][0]
        self.assertEqual(item["source"]["ItemSparse"]["Display_lang"], "Battleworn Bludgeon")
        self.assertEqual((item["inventory_type"], item["item_level"], item["quality"]), (17, 1, 1))
        self.assertIn("Two-Handed", plan["source_specialization"]["Description_lang"])

    def test_fury_requires_both_source_two_hand_weapons_without_one_hand_fallback(self):
        plan = bot_gear.load_plan(self.database, FIXTURES / "db2", 72)
        self.assertEqual([item["slot"] for item in plan["items"]], ["MainHand", "OffHand"])
        self.assertTrue(all(item["inventory_type"] == 17 for item in plan["items"]))
        starter = "slot=MainHand guid=116 item_id=25 name=\"Worn Shortsword\"\nslot=OffHand guid=115 item_id=2362 name=\"Worn Wooden Shield\"\n"
        self.assertEqual(bot_gear.required_grants(plan, starter), {2361: 2})
        correct = "slot=MainHand guid=10 item_id=2361\nslot=OffHand guid=11 item_id=2361\n"
        self.assertEqual(bot_gear.required_grants(plan, correct), {})

    def test_every_installed_playable_spec_has_a_sourced_fixture(self):
        bots = bot_roster.build_roster(FIXTURES / "db2", 32)
        for bot in bots:
            plan = bot_gear.load_plan(self.database, FIXTURES / "db2", bot["spec"])
            self.assertTrue(plan["items"], bot["spec"])
        with self.assertRaises(ValueError):
            bot_gear.load_plan(self.database, FIXTURES / "db2", 999999)


if __name__ == "__main__":
    unittest.main()
