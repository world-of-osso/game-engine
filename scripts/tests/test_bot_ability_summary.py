import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bot_ability_summary

FIXTURE = Path(__file__).parent / "fixtures/bot_harness/native-arms-ability-summary.json"


class BotAbilitySummaryTests(unittest.TestCase):
    def test_actual_native_requests_successes_damage_and_unattributed_errors(self):
        fixture = json.loads(FIXTURE.read_text())
        result = bot_ability_summary.summarize_run(fixture["run"], fixture["report"])
        abilities = {row["spell"]: row for row in result["abilities"]}
        self.assertEqual(abilities[7384]["requests"], 16)
        self.assertEqual(abilities[7384]["successes"], 3)
        self.assertEqual(abilities[7384]["event_damage"], 1163)
        self.assertEqual(abilities[7384]["missing_casts"], 13)
        self.assertEqual(abilities[100]["requests"], 0)
        self.assertTrue(abilities[100]["unused"])
        self.assertEqual(result["unattributed_native_errors"],
                         {"No charges remain.": 12, "Spell is not ready yet.": 1})
        self.assertTrue(result["measurement_complete"])

    def test_child_event_damage_keeps_its_real_spell_instead_of_inventing_root_credit(self):
        manifest = {"spec": 64, "role": 2, "attempts": [{"spell": 116}],
                    "spells": [{"id": 116, "damage_ids": [228597]}]}
        report = {"duration_ms": 60000, "measurement_complete": True, "dps": 2.0,
                  "damage": 120, "casts": {"116": 1}, "damage_by_spell": {"228597": 120},
                  "findings": [], "native_errors": []}
        result = bot_ability_summary.summarize_run(manifest, report)
        abilities = {row["spell"]: row for row in result["abilities"]}
        self.assertEqual(abilities[116]["event_damage"], 0)
        self.assertEqual(abilities[116]["damage_ids"], [228597])
        self.assertEqual(abilities[228597]["event_damage"], 120)
        self.assertEqual(abilities[228597]["requests"], 0)
        self.assertEqual(abilities[228597]["successes"], 0)

    def test_expected_specs_and_missing_repeats_do_not_disappear(self):
        fixture = json.loads(FIXTURE.read_text())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            run = root / "71/run-001"
            run.mkdir(parents=True)
            (run / "run.json").write_text(json.dumps(fixture["run"]))
            (run / "report.json").write_text(json.dumps(fixture["report"]))
            result = bot_ability_summary.summarize_roster(root, [{"spec": 71}, {"spec": 72}], 2)
        self.assertEqual([row["spec"] for row in result["specs"]], [71, 72])
        self.assertEqual(result["specs"][0]["missing_runs"], ["run-002"])
        self.assertEqual(result["specs"][1]["missing_runs"], ["run-001", "run-002"])
        self.assertEqual(len(result["specs"][0]["runs"]), 1)
        self.assertEqual(result["specs"][1]["runs"], [])


if __name__ == "__main__":
    unittest.main()
