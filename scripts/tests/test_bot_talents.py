import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bot_talents

DATA = Path(__file__).parent / "fixtures/bot_harness/db2"


class BotTalentPlanTests(unittest.TestCase):
    def test_installed_arcane_loadout_preserves_order_and_cumulative_ranks(self):
        plan = bot_talents.load_plan(DATA, 62)
        self.assertEqual(plan["loadout"], 991)
        self.assertEqual(len(plan["steps"]), 68)
        self.assertEqual([(step["node"], step["rank"]) for step in plan["steps"][:4]],
                         [(62122, 1), (62123, 1), (62123, 2), (62124, 1)])
        self.assertEqual(plan["steps"][6]["entry"], 134182)
        self.assertTrue(all(step["frame"].endswith("Button") for step in plan["steps"]))

    def test_multiple_sourced_loadouts_require_an_explicit_choice(self):
        with self.assertRaisesRegex(ValueError, "explicit loadout"):
            bot_talents.load_plan(DATA, 63)
        with self.assertRaisesRegex(ValueError, "specialization"):
            bot_talents.load_plan(DATA, 62, 930)


if __name__ == "__main__":
    unittest.main()
