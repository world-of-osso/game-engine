import json
import sys
import tempfile
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run_bot_roster
import run_dummy_bots


class NativeReadinessTests(unittest.TestCase):
    def test_cast_target_is_verified_explicit_native_selection_not_stale_echo(self):
        source = Path(__file__).parent / "fixtures/bot_harness/native-target-pending.json"
        state = json.loads(source.read_text())
        state["account"]["local_player_health"] = 100
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

    def test_actual_native_dead_player_is_not_ready_despite_allowed_input(self):
        source = Path(__file__).parent / "fixtures/bot_harness/native-dead-player.json"
        state = json.loads(source.read_text())
        self.assertTrue(state["account"]["gameplay_input_allowed"])
        self.assertEqual(state["account"]["local_player_health"], 0)
        self.assertFalse(run_bot_roster.native_ready(state, 70, True))
        state["at_ms"] = int(time.time() * 1000)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "native-state.json"
            path.write_text(json.dumps(state))
            with self.assertRaisesRegex(RuntimeError, "dead|health"):
                run_dummy_bots.read_state(path)
            state["account"]["local_player_health"] = 8208
            path.write_text(json.dumps(state))
            self.assertTrue(run_bot_roster.native_ready(state, 70, True))
            self.assertEqual(run_dummy_bots.read_state(path)["account"]["local_player_health"], 8208)

    def test_absent_health_observation_cannot_qualify_a_live_measurement(self):
        source = Path(__file__).parent / "fixtures/bot_harness/native-dead-player.json"
        state = json.loads(source.read_text())
        del state["account"]["local_player_health"]
        state["at_ms"] = int(time.time() * 1000)
        self.assertFalse(run_bot_roster.native_ready(state, 70, True))
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "native-state.json"
            path.write_text(json.dumps(state))
            with self.assertRaisesRegex(RuntimeError, "health.*observed"):
                run_dummy_bots.read_state(path)


if __name__ == "__main__":
    unittest.main()
