"""Captured NewWorld-before-input refusal; successful completion is synthetic."""

import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from run_stockade_bot import StockadeRun

FIXTURE = Path(__file__).parent / "fixtures/bot_harness/native-stockade-exit-transfer.json"


class ExitTransferTests(unittest.TestCase):
    def build_controller(self, directory, stderr=None):
        fixture = json.loads(FIXTURE.read_text())
        root = Path(directory)
        path = root / "native-account-state.json"
        path.write_text(json.dumps(fixture["before"]))
        controller = StockadeRun.__new__(StockadeRun)
        controller.args = SimpleNamespace(world_db=root / "unused.db", character="Bwarfury")
        controller.output = root
        controller.sequence = 0
        controller.account = lambda: json.loads(path.read_text())
        requested = []

        def native_command(argv, **kwargs):
            requested.append(argv)
            path.write_text(json.dumps(fixture["loading"]))
            return SimpleNamespace(returncode=1, stdout="", stderr=stderr or fixture["refusal"]["stderr"])

        def movement(here, goal, label):
            receipt = fixture["refusal"]
            controller.call(receipt["argv"][0], receipt["argv"][1:])

        controller.move = movement
        return controller, fixture, requested, native_command, path

    def test_transfer_refusal_awaits_destination_without_repeating_movement(self):
        with tempfile.TemporaryDirectory() as directory:
            controller, fixture, requested, native_command, path = self.build_controller(directory)

            def finish_loading(seconds):
                self.assertEqual(json.loads(path.read_text())["screen"], "Loading")
                path.write_text(json.dumps(fixture["confirmed"]))

            with (
                patch("run_stockade_bot.stockade_exit_plan", return_value=fixture["plan"]),
                patch("run_stockade_bot.subprocess.run", side_effect=native_command),
                patch("run_stockade_bot.time.sleep", side_effect=finish_loading),
            ):
                try:
                    controller.exit_instance()
                except RuntimeError as error:
                    self.fail(f"Expected transfer confirmation, not failed movement: {error}")
            self.assertEqual(requested, [fixture["refusal"]["argv"]])
            confirmation = json.loads((Path(directory) / "exit-confirmation.json").read_text())
            self.assertEqual(confirmation["account"]["world_map_id"], 0)
            self.assertEqual(confirmation["account"]["screen"], "InWorld")

    def test_other_command_failure_is_not_hidden_as_a_transfer(self):
        with tempfile.TemporaryDirectory() as directory:
            controller, fixture, requested, native_command, path = self.build_controller(
                directory, "error: native movement blocked\n"
            )
            with (
                patch("run_stockade_bot.stockade_exit_plan", return_value=fixture["plan"]),
                patch("run_stockade_bot.subprocess.run", side_effect=native_command),
            ):
                with self.assertRaisesRegex(RuntimeError, "movement blocked"):
                    controller.exit_instance()
            self.assertEqual(len(requested), 1)
            self.assertFalse((Path(directory) / "exit-confirmation.json").exists())

    def test_missing_destination_keeps_original_bound_and_no_success_or_retry(self):
        with tempfile.TemporaryDirectory() as directory:
            controller, fixture, requested, native_command, path = self.build_controller(directory)
            with (
                patch("run_stockade_bot.stockade_exit_plan", return_value=fixture["plan"]),
                patch("run_stockade_bot.subprocess.run", side_effect=native_command),
                patch("run_stockade_bot.time.monotonic", side_effect=[0, 1, 2, 241]),
                patch("run_stockade_bot.time.sleep"),
            ):
                with self.assertRaisesRegex(TimeoutError, "240s"):
                    controller.exit_instance()
            self.assertEqual(len(requested), 1)
            self.assertFalse((Path(directory) / "exit-confirmation.json").exists())


if __name__ == "__main__":
    unittest.main()
