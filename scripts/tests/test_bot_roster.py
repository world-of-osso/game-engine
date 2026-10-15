import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bot_roster

DATA = Path(__file__).parent / "fixtures/bot_harness/db2"


class BotRosterTests(unittest.TestCase):
    def test_one_character_per_actual_specialization_and_valid_race(self):
        bots = bot_roster.build_roster(DATA, 32)
        self.assertEqual(len(bots), 40)
        self.assertEqual(len({bot["spec"] for bot in bots}), 40)
        self.assertEqual(len({bot["name"] for bot in bots}), 40)
        self.assertNotIn(1449, {bot["spec"] for bot in bots})
        self.assertIn(1480, {bot["spec"] for bot in bots})
        mage = next(bot for bot in bots if bot["spec"] == 62)
        self.assertEqual((mage["class"], mage["race"], mage["level"], mage["role"]), (8, 1, 32, 2))
        shaman = next(bot for bot in bots if bot["spec"] == 262)
        self.assertEqual(shaman["race"], 11, "explicit source-valid Draenei fixture, not a failed-Dwarf retry")
        self.assertTrue(all(bot["account"].startswith("fb_bot_harness_") for bot in bots))
        self.assertTrue(all(bot["name"].isalpha() and len(bot["name"]) <= 12 for bot in bots))

    def test_authored_shaman_race_cannot_fall_back_to_another_source_pair(self):
        pairs = [{"RaceID": "3", "ClassID": "7"}]
        with self.assertRaisesRegex(ValueError, "11.*source"):
            bot_roster.select_alliance_race(7, pairs, {3, 11})


if __name__ == "__main__":
    unittest.main()
