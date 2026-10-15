"""Generic rule, exercised against retained actual installed DB2 rows."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bot_rotation

DATA = Path(__file__).parent / "fixtures/bot_harness/db2"


class BotRotationTests(unittest.TestCase):
    def test_only_known_active_direct_enemy_damage_is_selected(self):
        spells = bot_rotation.load_spells(DATA, [30451, 44425, 137018])
        self.assertEqual({spell["id"] for spell in spells}, {30451, 44425})
        self.assertEqual(bot_rotation.choose(spells, {}, 0)["id"], 30451)

    def test_highest_coefficient_available_not_fabricated_priority(self):
        spells = bot_rotation.load_spells(DATA, [30451, 44425])
        self.assertEqual(bot_rotation.choose(spells, {30451: 3000}, 1000)["id"], 44425)
        self.assertIsNone(bot_rotation.choose(spells, {30451: 3000, 44425: 3000}, 1000))

    def test_db2_cast_time_and_gcd_are_preserved(self):
        spells = bot_rotation.load_spells(DATA, [30451, 44425])
        blast = next(s for s in spells if s["id"] == 30451)
        self.assertEqual(blast["cast_ms"], 2250)
        self.assertEqual(blast["gcd_ms"], 1500)
        self.assertGreater(blast["coefficient"], 1)
        self.assertEqual(blast["source"]["SpellEffect"][0]["SpellID"], "30451")


if __name__ == "__main__":
    unittest.main()
