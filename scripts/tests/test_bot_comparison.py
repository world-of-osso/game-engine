"""Comparison envelope fixtures; numbers are accounting inputs, not a DPS oracle."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run_bot_roster


class BotComparisonTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.output = Path(self.directory.name)
        self.bots = [{"spec": spec, "role": 2} for spec in (71, 577, 255)]
        self.comparison = {"level": 32, "stats": {"item_level": 30, "quality": 2, "item_squish_era": 0},
                           "dummy": {"entry": 44548, "level": 30, "max_health": 13137,
                                     "difficulty": 0, "world_sha256": "fixture-world"},
                           "duration_ms": 60000}
        for spec, values in [(71, [10, 14]), (577, [6, 8]), (255, [8, 10])]:
            for index, dps in enumerate(values, 1):
                directory = self.output / str(spec) / f"run-{index:03d}"
                directory.mkdir(parents=True)
                row = {"spec": spec, "role": 2, "measurement_complete": True,
                       "dps": dps, "comparison": self.comparison}
                (directory / "report.json").write_text(json.dumps(row))

    def test_live_dummy_fixture_retains_max_health_not_current_damage(self):
        text = "Training Dummy (entity 42) level 30 health 12/13137 difficulty 0\nTraining Dummy (entity 99) level 32 health 900/900 difficulty 1"
        self.assertEqual(run_bot_roster.dummy_fixture(text, 42),
                         {"entry": 44548, "level": 30, "max_health": 13137, "difficulty": 0})
        with self.assertRaisesRegex(ValueError, "dummy fixture"):
            run_bot_roster.dummy_fixture(text, 123)

    def test_three_complete_equal_budget_specs_produce_spec_weighted_median(self):
        run_bot_roster.write_roster_comparison(self.output, self.bots, 2)
        result = json.loads((self.output / "balance.json").read_text())
        self.assertTrue(result["available"])
        self.assertEqual(result["median_dps"], 9)
        self.assertEqual(result["max_over_median"], 12 / 9)
        self.assertIsNone(result["tolerance"])
        self.assertTrue((self.output / "comparison.md").exists())

    def test_blocked_or_missing_spec_cannot_silently_be_dropped(self):
        (self.output / "255/run-002/report.json").unlink()
        run_bot_roster.write_roster_comparison(self.output, self.bots, 2)
        result = json.loads((self.output / "balance.json").read_text())
        self.assertFalse(result["available"])
        self.assertEqual(result["expected_specs"], [71, 577, 255])
        self.assertIn("255/run-002/report.json", result["missing_runs"])
        self.assertNotIn("median_dps", result)

    def test_different_source_budgets_withhold_comparison_instead_of_pooling(self):
        path = self.output / "577/run-002/report.json"
        row = json.loads(path.read_text())
        row["comparison"]["stats"]["quality"] = 3
        path.write_text(json.dumps(row))
        run_bot_roster.write_roster_comparison(self.output, self.bots, 2)
        result = json.loads((self.output / "balance.json").read_text())
        self.assertFalse(result["available"])
        self.assertIn("Non-comparable", result["reason"])
        self.assertNotIn("median_dps", result)


if __name__ == "__main__":
    unittest.main()
