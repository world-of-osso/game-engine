"""Synthetic cleared-area preconditions; no historical live stall is claimed."""

import json
import math
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_controller import PlayerController
from run_stockade_bot import StockadeRun

FIXTURE = Path(__file__).parent / "fixtures/bot_harness/native-stockade-boss-progression.json"


class BossProgressionTests(unittest.TestCase):
    def build_controller(self, directory):
        fixture = json.loads(FIXTURE.read_text())
        controller = StockadeRun.__new__(StockadeRun)
        controller.output = Path(directory)
        controller.opening_target = None
        controller.blocked_steps = 0
        controller.tank = "Captain Garrick"
        controller.run = {"clear_elapsed_ms": None, "bosses": fixture["bosses"], "openings": []}
        controller.poll_combat = lambda: None
        controller.record_credit = lambda: None
        controller.party = lambda: [{"health": 100, "max_health": 100}]
        controller.account = lambda: {"local_player_position": str(tuple(fixture["here"]))}
        controller.rotation = lambda: fixture["rotation"]
        commands = []
        inputs = []

        def request(kind, payload):
            inputs.append(kind)
            return fixture["empty_tab"]

        controller.player = PlayerController(
            lambda *argv: commands.append(argv),
            lambda target: self.fail("No target was selected"),
            lambda here, goal, label: fixture["strict_routes"][
                str(next(boss["encounter"] for boss in fixture["bosses"] if boss["goal"] == goal))
            ],
            SimpleNamespace(request=request),
        )
        controller.player.assist = lambda tank: None
        return controller, fixture, commands, inputs

    def test_no_hostile_advances_one_bounded_native_corner_toward_first_boss(self):
        with tempfile.TemporaryDirectory() as directory:
            controller, fixture, commands, inputs = self.build_controller(directory)
            with patch("run_stockade_bot.time.sleep"):
                try:
                    complete = controller.step()
                except ValueError as error:
                    self.fail(f"Cleared-area no-hostile signal stopped boss progression: {error}")
                self.assertFalse(complete)
            self.assertEqual(inputs, ["enemy-tab"])
            self.assertEqual(controller.run["openings"], [])
            self.assertEqual(len(commands), 1)
            self.assertEqual(commands[0][:2], ("movement", "forward"))
            self.assertLessEqual(float(commands[0][3]), 0.55)
            corner = fixture["strict_route_corners"][1]
            expected_yaw = math.degrees(
                math.atan2(corner[0] - fixture["here"][0], corner[2] - fixture["here"][2])
            )
            self.assertAlmostEqual(float(commands[0][5]), expected_yaw)
            receipt = json.loads((Path(directory) / "boss-progression.jsonl").read_text())
            self.assertEqual(receipt["encounter"], 1146)
            self.assertFalse(receipt["reached"])

    def test_done_boss_is_skipped_without_reordering_remaining_source_goals(self):
        with tempfile.TemporaryDirectory() as directory:
            controller, fixture, commands, inputs = self.build_controller(directory)
            controller.run["bosses"][0]["state"] = "Done"
            with patch("run_stockade_bot.time.sleep"):
                controller.step()
            receipt = json.loads((Path(directory) / "boss-progression.jsonl").read_text())
            self.assertEqual(receipt["encounter"], 1145)
            self.assertEqual(receipt["goal"], fixture["bosses"][1]["goal"])

    def test_combat_start_during_empty_tab_prevents_progression(self):
        with tempfile.TemporaryDirectory() as directory:
            controller, fixture, commands, inputs = self.build_controller(directory)
            original = controller.player.bridge.request

            def enter_combat(kind, payload):
                fixture["rotation"]["assisted"]["in_combat"] = True
                return original(kind, payload)

            controller.player.bridge.request = enter_combat
            with patch("run_stockade_bot.time.sleep"):
                controller.step()
            self.assertEqual(commands, [])
            self.assertEqual(inputs, ["enemy-tab"])

    def test_recovery_does_not_tab_or_walk(self):
        with tempfile.TemporaryDirectory() as directory:
            controller, fixture, commands, inputs = self.build_controller(directory)
            controller.party = lambda: [{"health": 99, "max_health": 100}]
            with patch("run_stockade_bot.time.sleep"):
                controller.step()
            self.assertEqual((commands, inputs), ([], []))

    def test_damage_received_during_empty_tab_prevents_progression(self):
        with tempfile.TemporaryDirectory() as directory:
            controller, fixture, commands, inputs = self.build_controller(directory)
            parties = iter([[{"health": 100, "max_health": 100}], [{"health": 99, "max_health": 100}]])
            controller.party = lambda: next(parties)
            with patch("run_stockade_bot.time.sleep"):
                controller.step()
            self.assertEqual(commands, [])
            self.assertEqual(inputs, ["enemy-tab"])

    def test_all_done_does_not_walk_an_old_goal(self):
        with tempfile.TemporaryDirectory() as directory:
            controller, fixture, commands, inputs = self.build_controller(directory)
            for boss in fixture["bosses"]:
                boss["state"] = "Done"
            with patch("run_stockade_bot.time.sleep"):
                controller.step()
            self.assertEqual(commands, [])
            self.assertFalse((Path(directory) / "boss-progression.jsonl").exists())

    def test_reached_boss_stops_without_walking_to_a_later_boss(self):
        with tempfile.TemporaryDirectory() as directory:
            controller, fixture, commands, inputs = self.build_controller(directory)
            fixture["here"] = list(fixture["bosses"][0]["goal"])
            with patch("run_stockade_bot.time.sleep"):
                controller.step()
            self.assertEqual(commands, [])
            receipt = json.loads((Path(directory) / "boss-progression.jsonl").read_text())
            self.assertEqual(receipt["encounter"], 1146)
            self.assertTrue(receipt["reached"])

    def test_observation_error_is_not_converted_to_no_hostile(self):
        with tempfile.TemporaryDirectory() as directory:
            controller, fixture, commands, inputs = self.build_controller(directory)

            def fail_query(kind, payload):
                raise RuntimeError("Native hostility observation unavailable")

            controller.player.bridge.request = fail_query
            with self.assertRaisesRegex(RuntimeError, "unavailable"):
                controller.step()
            self.assertEqual(commands, [])


if __name__ == "__main__":
    unittest.main()
