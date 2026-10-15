import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run_bot_roster


class NativeReadinessTests(unittest.TestCase):
    def test_cast_target_is_verified_explicit_native_selection_not_stale_echo(self):
        source = Path(__file__).parent / "fixtures/bot_harness/native-target-pending.json"
        state = json.loads(source.read_text())
        self.assertEqual(state["target"]["target_name"], "Training Dummy")
        self.assertNotEqual(state["target"]["target"], state["target"]["server_target"])
        self.assertFalse(run_bot_roster.native_ready(state, 62, True))
        state["spells"]["catalog_ready"] = True
        self.assertFalse(run_bot_roster.native_ready(state, 62, True))
        state["talent_status"] = "observed"
        self.assertFalse(run_bot_roster.native_ready(state, 62, True))
        state["gear_status"] = "observed"
        self.assertTrue(run_bot_roster.native_ready(state, 62, True))
        verified = "Training Dummy (entity 4294959692) level 30 health 13137/13137 difficulty 0"
        self.assertEqual(run_bot_roster.verified_dummy_target(state, verified), 4294959692)
        with self.assertRaises(ValueError):
            run_bot_roster.verified_dummy_target(state, "Training Dummy (entity 999) level 30")
        state["spells"]["catalog_ready"] = False
        self.assertFalse(run_bot_roster.native_ready(state, 62, True))


if __name__ == "__main__":
    unittest.main()
