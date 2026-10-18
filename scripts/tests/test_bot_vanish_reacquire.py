"""Run23 native loss/reappearance frames; post-assist responses are synthetic controls."""
import copy
import json
import sys
import tempfile
import unittest
from contextlib import contextmanager
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_controller import PlayerController
from run_stockade_bot import StockadeRun

FIXTURE = Path(__file__).parent / "fixtures/bot_harness/native-stockade-vanish.json"


class VanishReacquisitionTests(unittest.TestCase):
    @contextmanager
    def controller(self, frame_name="absent"):
        fixture = json.loads(FIXTURE.read_text())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            native = SimpleNamespace(frame=None, clock=0.0, wall_ms=0, commands=[], inputs=[], after_assist=None)

            def publish_frame(frame):
                native.frame = copy.deepcopy(frame)
                native.wall_ms = native.frame["at_ms"]
                (root / "native-rotation-state.json").write_text(json.dumps(native.frame))

            def sleep(seconds):
                # Explicit synthetic fresh publications for deadline controls.
                native.clock += seconds
                native.wall_ms += seconds * 1000
                native.frame["at_ms"] = round(native.wall_ms)
                (root / "native-rotation-state.json").write_text(json.dumps(native.frame))

            def invoke(*words):
                native.commands.append(words)
                return "native request submitted"

            def request(kind, payload):
                native.inputs.append((kind, payload))
                self.assertEqual(kind, "assist", "no Tab cycling or alternate input")
                self.assertEqual(payload, {"leader_name": fixture["tank_name"]})
                if native.after_assist is not None:
                    # No actual post-Vanish assist exists in run23; this is synthetic.
                    frame = copy.deepcopy(native.after_assist)
                    frame["at_ms"] = round(native.wall_ms + 1)
                    publish_frame(frame)
                return {"before": {"target_name": fixture["tank_name"], "target": fixture["tank"]},
                        "after": native.frame["target_state"]}

            account = root / "native-account-state.json"
            account.write_text(json.dumps(fixture["account"]))
            run = StockadeRun.__new__(StockadeRun)
            run.args = SimpleNamespace(spec=72)
            run.output = root
            run.opening_target = fixture["boss"]
            run.tank = fixture["tank_name"]
            run.run = {"clear_elapsed_ms": None, "openings": []}
            run.poll_combat = lambda: None
            run.record_credit = lambda: None
            run.rainfire_areas = lambda: []  # Run23 never observed ground objects; synthetic control.
            run.party = lambda: fixture["party_full_health_control"]
            run.invoke = invoke
            run.player = PlayerController(invoke, run.snapshot, lambda *args: self.fail("No movement expected"),
                                          SimpleNamespace(request=request))
            native.publish = publish_frame
            native.account_path = account
            publish_frame(fixture["frames"][frame_name])
            with (patch("run_stockade_bot.time.time", side_effect=lambda: native.wall_ms / 1000),
                  patch("run_stockade_bot.time.monotonic", side_effect=lambda: native.clock),
                  patch("run_stockade_bot.time.sleep", side_effect=sleep)):
                yield run, native, fixture

    def assert_no_extra_input(self, native):
        self.assertEqual(native.commands, [])
        self.assertEqual(native.inputs, [])

    def test_actual_no_selected_target_ends_step_without_waiting_or_input(self):
        with self.controller() as (run, native, fixture):
            self.assertIsNone(native.frame["target"])
            self.assertIsNone(native.frame["facts"])
            self.assertFalse(run.step())
            self.assertIsNone(run.opening_target)
            self.assertEqual(native.clock, 0)
            self.assert_no_extra_input(native)

    def test_actual_reappeared_node_does_not_restore_selection_and_next_cycle_only_assists(self):
        with self.controller("returned_unselected") as (run, native, fixture):
            self.assertIsNotNone(native.frame["recorded_boss_origin"])
            self.assertIsNone(native.frame["target"])
            self.assertFalse(run.step())
            self.assert_no_extra_input(native)
            self.assertFalse(run.step())
            self.assertEqual(native.inputs, [("assist", {"leader_name": fixture["tank_name"]})])
            self.assertEqual(native.commands, [("quest", "interact", "--npc", fixture["tank_name"])])
            self.assertEqual(run.run["openings"], [])

    def test_synthetic_native_assist_requires_new_bound_usable_facts_before_one_cast(self):
        with self.controller("returned_unselected") as (run, native, fixture):
            run.step()
            native.after_assist = fixture["frames"]["cast_ready"]
            self.assertFalse(run.step())
            spell = int(native.after_assist["assisted"]["next_spell"])
            self.assertNotEqual(spell, 57755)
            self.assertEqual(native.commands, [("quest", "interact", "--npc", fixture["tank_name"]),
                ("spell", "cast", "--spell", str(spell), "--target", str(fixture["boss"]))])
            before = list(native.commands)
            run.step()  # No new ACK: cannot cast or re-assist again.
            self.assertEqual(native.commands, before)
            self.assertEqual(len(native.inputs), 1)
            self.assertEqual(run.run["openings"], [])

    def test_loss_during_precast_reobservation_ends_step_without_cast(self):
        with self.controller("cast_ready") as (run, native, fixture):
            original = run.player.snapshot

            def lose_selection(target):
                native.publish(fixture["frames"]["absent"])
                return original(target)

            run.player.snapshot = lose_selection
            self.assertFalse(run.step())
            self.assertIsNone(run.opening_target)
            self.assertIsNone(run.player.pending)
            self.assert_no_extra_input(native)

    def test_reacquired_but_unusable_readiness_cost_or_range_never_casts(self):
        for field in ["ready", "affordable", "in_range"]:
            with self.subTest(field=field), self.controller() as (run, native, fixture):
                run.step()
                native.after_assist = copy.deepcopy(fixture["frames"]["cast_ready"])
                spell = str(int(native.after_assist["assisted"]["next_spell"]))
                native.after_assist["facts"][field][spell] = {"Ok": False}
                run.step()
                self.assertEqual(native.commands, [("quest", "interact", "--npc", fixture["tank_name"])])
                self.assertEqual(run.run["openings"], [])

    def test_pending_cast_is_not_reset_or_repeated_during_target_loss(self):
        with self.controller("before_last_request") as (run, native, fixture):
            attempt = fixture["last_attempt"]
            run.player.submit_cast(native.frame, fixture["boss"], attempt["spell"])
            pending = run.player.pending
            lost = copy.deepcopy(fixture["frames"]["absent"])
            actor = int(fixture["frames"]["before_last_request"]["actor"])
            lost["spells"]["cast_success_events"] = [event for event in lost["spells"]["cast_success_events"]
                if not (int(event["source"]) == actor and int(event["spell_id"]) == attempt["spell"]
                        and int(event["at_ms"]) >= attempt["at_ms"])]
            native.publish(lost)
            native.commands.clear()
            self.assertFalse(run.step())
            self.assertIs(run.player.pending, pending)
            self.assertEqual(run.opening_target, fixture["boss"])
            self.assert_no_extra_input(native)
            native.publish(fixture["frames"]["absent"])  # Real retained server success releases input.
            self.assertFalse(run.step())
            self.assertIsNone(run.player.pending)
            self.assertIsNone(run.opening_target)
            self.assert_no_extra_input(native)
            self.assertEqual(len(run.player.outcomes), 1)
            self.assertTrue(run.player.outcomes[0]["success"])

    def test_selected_but_missing_facts_or_changed_target_keeps_ten_second_failure(self):
        for changed in [False, True]:
            with self.subTest(changed=changed), self.controller("before") as (run, native, fixture):
                frame = copy.deepcopy(native.frame)
                if changed:
                    frame["target"] = fixture["boss"] + 1
                else:
                    frame["facts"] = None
                native.publish(frame)
                with self.assertRaisesRegex(RuntimeError, "target-bound.*10s"):
                    run.step()
                self.assertGreaterEqual(native.clock, 10)
                self.assertLess(native.clock, 10.1)
                self.assertEqual(run.opening_target, fixture["boss"])
                self.assert_no_extra_input(native)

    def test_out_of_combat_or_loading_absence_is_not_the_approved_recovery(self):
        for context in ["out_of_combat", "loading"]:
            with self.subTest(context=context), self.controller() as (run, native, fixture):
                if context == "out_of_combat":
                    frame = copy.deepcopy(native.frame)
                    frame["assisted"]["in_combat"] = False
                    native.publish(frame)
                else:
                    account = json.loads(native.account_path.read_text())
                    account["screen"] = "Loading"
                    native.account_path.write_text(json.dumps(account))
                with self.assertRaisesRegex(RuntimeError, "target-bound.*10s"):
                    run.step()
                self.assertEqual(run.opening_target, fixture["boss"])
                self.assert_no_extra_input(native)

    def test_dead_or_unobserved_owner_is_not_hidden_as_target_loss(self):
        for health in [0, None]:
            with self.subTest(health=health), self.controller() as (run, native, fixture):
                account = json.loads(native.account_path.read_text())
                account["local_player_health"] = health
                native.account_path.write_text(json.dumps(account))
                with self.assertRaisesRegex(RuntimeError, "dead|health"):
                    run.step()
                self.assertEqual(run.opening_target, fixture["boss"])
                self.assert_no_extra_input(native)

    def test_stale_observation_and_transport_failure_propagate(self):
        with self.controller() as (run, native, fixture):
            native.wall_ms += 10001
            with self.assertRaisesRegex(RuntimeError, "stale"):
                run.step()
            self.assert_no_extra_input(native)
        with self.controller() as (run, native, fixture):
            def unavailable():
                raise RuntimeError("native IPC transport failed")
            run.rotation = unavailable
            with self.assertRaisesRegex(RuntimeError, "IPC transport failed"):
                run.step()
            self.assertEqual(run.opening_target, fixture["boss"])
            self.assert_no_extra_input(native)

    def test_selected_dead_target_keeps_existing_observed_death_path(self):
        with self.controller("before") as (run, native, fixture):
            frame = copy.deepcopy(native.frame)
            frame["spells"]["target_health"] = 0
            native.publish(frame)
            self.assertFalse(run.step())
            self.assertIsNone(run.opening_target)
            self.assert_no_extra_input(native)


if __name__ == "__main__":
    unittest.main()
