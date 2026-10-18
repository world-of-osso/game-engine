"""Synthetic received area/route controls; not a fresh native escape receipt."""
import json
import sys
import tempfile
import time
import unittest
from contextlib import contextmanager
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_controller import PlayerController
from run_stockade_bot import StockadeRun


class HazardControllerTests(unittest.TestCase):
    @contextmanager
    def controller(self):
        with tempfile.TemporaryDirectory() as directory:
            run = StockadeRun.__new__(StockadeRun)
            run.output = Path(directory)
            run.args = SimpleNamespace(spec=72)
            run.opening_target = 99
            run.blocked_steps = 0
            run.tank = "Captain Garrick"
            run.run = {"clear_elapsed_ms": None}
            run.poll_combat = lambda: None
            run.record_credit = lambda: None
            run.party = lambda: [{"health": 100, "max_health": 100}]
            run.account = lambda: {"screen": "InWorld", "local_player_health": 100,
                                   "local_player_position": "(1,0,0)"}
            run.snapshot = lambda target: self.fail("Target work must not precede area escape")
            commands, queries = [], []

            def route(start, goal, label):
                queries.append((start, goal, label))
                return [start, goal]

            run.query_route = route
            run.player = PlayerController(lambda *argv: commands.append(argv), run.snapshot,
                lambda *args: run.safe_route(*args),
                SimpleNamespace(request=lambda *args: self.fail("Assist/Tab during escape")))
            state = {"at_ms": int(time.time() * 1000), "actor": 7,
                     "assisted": {"in_combat": True},
                     "spells": {"cast_failure_seq": 0, "cast_failure_events": [], "cast_success_events": []},
                     "ground_areas": {"state_json": json.dumps({"areas": [{
                         "id": 14, "caster": 22, "spell": 86636, "center": [0, 0, 0],
                         "radius": 5, "hostile": True}]})}}
            def publish():
                (run.output / "native-rotation-state.json").write_text(json.dumps(state))
            publish()
            with patch("run_stockade_bot.time.sleep"):
                yield run, state, commands, queries, publish

    def test_escape_preempts_target_work_with_one_bounded_native_movement(self):
        with self.controller() as (run, state, commands, queries, publish):
            self.assertFalse(run.step())
            self.assertEqual(run.opening_target, 99)
            self.assertEqual(len(commands), 1)
            self.assertEqual(commands[0][:2], ("movement", "forward"))
            self.assertLessEqual(float(commands[0][3]), .55)
            self.assertEqual(len(run.player.outcomes), 0)
            self.assertGreaterEqual(len(queries), 8)

    def test_pending_cast_blocks_escape_without_reset_or_retry(self):
        with self.controller() as (run, state, commands, queries, publish):
            run.player.submit_cast(state, 99, 1464)
            pending = run.player.pending
            commands.clear()
            self.assertFalse(run.step())
            self.assertIs(run.player.pending, pending)
            self.assertEqual((commands, queries), ([], []))
            self.assertEqual(run.opening_target, 99)

    def test_outside_route_cannot_cross_actual_sphere_between_clear_endpoints(self):
        with self.controller() as (run, state, commands, queries, publish):
            with self.assertRaisesRegex(ValueError, "Rain.of.Fire|hazard"):
                run.safe_route([6, 0, 0], [-6, 0, 0], "native-melee-approach")
            self.assertEqual(commands, [])

    def test_unavailable_or_stale_area_observation_is_not_safe(self):
        for context in ["error", "missing", "stale"]:
            with self.subTest(context=context), self.controller() as (run, state, commands, queries, publish):
                if context == "error":
                    state["ground_areas"] = {"error": "caster faction unavailable"}
                elif context == "missing":
                    del state["ground_areas"]
                else:
                    state["at_ms"] -= 10001
                publish()
                with self.assertRaises((KeyError, ValueError, RuntimeError)):
                    run.step()
                self.assertEqual((commands, queries), ([], []))

    def test_explicit_empty_area_list_does_not_request_escape(self):
        with self.controller() as (run, state, commands, queries, publish):
            state["ground_areas"] = {"state_json": '{"areas":[]}'}
            publish()
            self.assertFalse(run.escape_hazard())
            self.assertEqual((commands, queries), ([], []))


if __name__ == "__main__":
    unittest.main()
