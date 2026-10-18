import json
import sys
import tempfile
import unittest
from types import SimpleNamespace
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bot_talents
from run_bot_roster import prepare_native_talents

DATA = Path(__file__).parent / "fixtures/bot_harness/db2"
FURY = DATA.parent / "talent_tree72"


class BotTalentPlanTests(unittest.TestCase):
    def test_installed_arcane_loadout_preserves_order_and_cumulative_ranks(self):
        plan = bot_talents.load_plan(DATA, 62, native_tree=658)
        self.assertEqual(plan["loadout"], 991)
        self.assertEqual(len(plan["steps"]), 68)
        self.assertEqual(
            [(step["node"], step["rank"]) for step in plan["steps"][:4]],
            [(62122, 1), (62123, 1), (62123, 2), (62124, 1)],
        )
        self.assertEqual(plan["steps"][6]["entry"], 134182)
        self.assertTrue(all(step["frame"].endswith("Button") for step in plan["steps"]))

    def test_received_fury_tree_selects_matching_installed_loadout(self):
        received = json.loads((FURY / "provenance.json").read_text())["native_trait_config"]
        self.assertEqual(
            int(
                bot_talents.select_loadout(FURY, 72, None, native_tree=int(received["tree_id"]))[
                    "TraitTreeID"
                ]
            ),
            int(received["tree_id"]),
        )
        plan = bot_talents.load_plan(FURY, 72, native_tree=int(received["tree_id"]))
        self.assertEqual((plan["loadout"], plan["tree"]), (915, 850))
        self.assertTrue(any(step["spell"] == 23881 for step in plan["steps"]))
        other = bot_talents.load_plan(FURY, 72, native_tree=880)
        self.assertEqual((other["loadout"], other["tree"]), (954, 880))

    def test_missing_and_unmatched_native_tree_stop_instead_of_selecting_last(self):
        for tree in (None, 0, 999999):
            with self.subTest(tree=tree), self.assertRaisesRegex(ValueError, "tree"):
                bot_talents.load_plan(FURY, 72, native_tree=tree)

    def test_explicit_loadout_cannot_bypass_received_tree(self):
        with self.assertRaisesRegex(ValueError, "tree"):
            bot_talents.load_plan(FURY, 72, 954, native_tree=850)

    def test_roster_publishes_plan_from_received_owner_tree(self):
        config = json.loads((FURY / "provenance.json").read_text())["native_trait_config"]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "data/db2").mkdir(parents=True)
            (root / "data/db2/active").symlink_to(FURY.resolve())
            output = root / "bot"
            output.mkdir()
            (output / "native-state.json").write_text(json.dumps({"trait_config": config}))
            prepare_native_talents(SimpleNamespace(engine=root), output, {"spec": 72})
            plan = json.loads((output / "talent-plan.json").read_text())
            self.assertEqual((plan["loadout"], plan["tree"]), (915, 850))
            (output / "talent-plan.json").unlink()
            config.pop("tree_id")
            (output / "native-state.json").write_text(json.dumps({"trait_config": config}))
            with self.assertRaisesRegex(ValueError, "tree"):
                prepare_native_talents(SimpleNamespace(engine=root), output, {"spec": 72})
            self.assertFalse((output / "talent-plan.json").exists())

    def test_pinned_tc_last_encountered_loadout_and_explicit_spec_validation(self):
        self.assertEqual(bot_talents.select_loadout(DATA, 63, None, native_tree=658)["ID"], "931")
        with self.assertRaisesRegex(ValueError, "specialization"):
            bot_talents.load_plan(DATA, 62, 930, native_tree=658)


if __name__ == "__main__":
    unittest.main()
