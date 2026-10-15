import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run_bot_roster


class NativeReadinessTests(unittest.TestCase):
    def test_real_predicted_dummy_with_other_server_target_is_not_ready(self):
        source = Path(__file__).parent / "fixtures/bot_harness/native-target-pending.json"
        state = json.loads(source.read_text())
        self.assertEqual(state["target"]["target_name"], "Training Dummy")
        self.assertNotEqual(state["target"]["target"], state["target"]["server_target"])
        self.assertFalse(run_bot_roster.native_ready(state, 62, True))
        state["target"]["server_target"] = state["target"]["target"]
        self.assertFalse(run_bot_roster.native_ready(state, 62, True))
        state["spells"]["catalog_ready"] = True
        self.assertTrue(run_bot_roster.native_ready(state, 62, True))
        state["spells"]["catalog_ready"] = False
        self.assertFalse(run_bot_roster.native_ready(state, 62, True))


if __name__ == "__main__":
    unittest.main()
