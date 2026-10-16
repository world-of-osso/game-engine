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
                state = {"account": {"local_player_health": 1000}}
                (directory / "native-before.json").write_text(json.dumps(state))
                (directory / "native-samples.jsonl").write_text(json.dumps(state) + "\n")

    def test_actual_dead_start_is_excluded_without_rewriting_raw_reports(self):
        source = Path(__file__).parent / "fixtures/bot_harness/native-dead-player.json"
        state = json.loads(source.read_text())
        directory = self.output / "70/run-001"
        directory.mkdir(parents=True)
        row = {"spec": 70, "role": 2, "measurement_complete": True,
               "dps": 0, "comparison": self.comparison}
        path = directory / "report.json"
        original = json.dumps(row)
        path.write_text(original)
        (directory / "native-before.json").write_text(json.dumps(state))
        (directory / "native-samples.jsonl").write_text(json.dumps(state) + "\n")
        reports, missing = run_bot_roster.read_roster_reports(self.output, [70], 1)
        self.assertFalse(reports[0]["measurement_complete"])
        self.assertIsNone(reports[0]["dps"])
        self.assertIn("dead", reports[0]["health_qualification"]["error"])
        self.assertEqual(reports[0]["reported_dps"], 0)
        self.assertEqual(missing, ["70/run-001/report.json"])
        self.assertEqual(path.read_text(), original)

    def test_missing_health_artifacts_cannot_qualify_a_report(self):
        (self.output / "71/run-001/native-samples.jsonl").unlink()
        reports, missing = run_bot_roster.read_roster_reports(self.output, [71], 2)
        self.assertFalse(reports[0]["measurement_complete"])
        self.assertIsNone(reports[0]["dps"])
        self.assertIn("71/run-001/report.json", missing)
        self.assertTrue(reports[1]["measurement_complete"])

    def test_live_dummy_fixture_retains_max_health_not_current_damage(self):
        text = ('Training Dummy (entity 42) level 30 health 12/13137 difficulty 0 '
                'combat={"creature_armor":155.0,"stats":null,"threat":{},"victim":17}\n'
                'Training Dummy (entity 99) level 32 health 900/900 difficulty 1 '
                'combat={"creature_armor":160.0,"stats":null,"threat":null,"victim":null}')
        self.assertEqual(run_bot_roster.dummy_fixture(text, 42),
                         {"entry": 44548, "level": 30, "max_health": 13137, "difficulty": 0,
                          "creature_armor": 155.0})
        with self.assertRaisesRegex(ValueError, "dummy fixture"):
            run_bot_roster.dummy_fixture(text, 123)

    def test_actual_native_dummy_combat_suffix_preserves_static_fixture_armor(self):
        source = Path(__file__).parent / "fixtures/bot_harness/native-dummy-combat.txt"
        self.assertEqual(run_bot_roster.dummy_fixture(source.read_text(), 4294959692),
                         {"entry": 44548, "level": 30, "max_health": 13137, "difficulty": 0,
                          "creature_armor": 155.0})

    def test_dummy_fixture_refuses_missing_or_malformed_combat_metadata(self):
        header = "Training Dummy (entity 42) level 30 health 12/13137 difficulty 0"
        for text in (header, header + ' combat={}', header + ' combat={broken}'):
            with self.subTest(text=text), self.assertRaises(ValueError):
                run_bot_roster.dummy_fixture(text, 42)

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
