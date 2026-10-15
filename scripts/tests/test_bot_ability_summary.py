import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bot_ability_summary
from bot_harness import parse_combat

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

    def test_actual_log_exposes_hits_and_source_coefficients_but_not_lost_crit_flags(self):
        fixture = json.loads(FIXTURE.read_text())
        combat = FIXTURE.with_name("native-arms-ability-summary-combat.log").read_text()
        result = bot_ability_summary.summarize_run(fixture["run"], fixture["report"], parse_combat(combat))
        ability = next(row for row in result["abilities"] if row["spell"] == 7384)
        self.assertEqual(ability["hits"], 3)
        self.assertEqual(ability["misses"], 0)
        self.assertIsNone(ability["crits"])
        self.assertEqual(ability["source_effects"][0]["BonusCoefficientFromAP"], "3.8561399")
        self.assertEqual(result["auto_attack"]["damage"], 0)
        self.assertEqual(result["auto_attack"]["share"], 0)
        self.assertIn("crit_outcomes", result["unobserved"])
        self.assertEqual(result["authored_order"], fixture["run"]["authored_order"])

    def test_crit_miss_and_swing_damage_keep_real_event_spell_identity(self):
        combat = "\n".join([
            'damage src=1 dst=9 spell=- amount=100 aura=- text=1 hit 9 for 100 timestamp_unix_ms=1000 spell_name=- crit=true',
            'damage src=1 dst=9 spell=- amount=- aura=- text=1 missed 9: Dodge timestamp_unix_ms=1100 spell_name=- crit=false',
            'damage src=1 dst=9 spell=133 amount=50 aura=- text=1 damaged 9 for 50 timestamp_unix_ms=1200 spell_name="Fireball" crit=false',
        ])
        manifest = {"spec": 63, "role": 2, "actor": "1", "target": "9", "start_ms": 0, "end_ms": 2000,
                    "attempts": [], "spells": [{"id": 133, "damage_ids": [133]}], "owner_snapshot": []}
        report = {"duration_ms": 2000, "measurement_complete": True, "dps": 75, "damage": 150,
                  "casts": {}, "damage_by_spell": {"null": 100, "133": 50}, "findings": [], "native_errors": []}
        result = bot_ability_summary.summarize_run(manifest, report, parse_combat(combat))
        swing = next(row for row in result["abilities"] if row["spell"] is None)
        self.assertEqual((swing["hits"], swing["crits"], swing["misses"]), (1, 1, 1))
        self.assertEqual(swing["miss_by_kind"], {"Dodge": 1})
        self.assertEqual(swing["event_damage"], 100)
        self.assertEqual(result["auto_attack"]["share"], 2 / 3)

    def test_sampled_gcd_idle_and_resource_net_change_are_not_invented_spending(self):
        manifest = {"spec": 72, "role": 2, "actor": "1", "target": "9", "start_ms": 0, "end_ms": 2000,
                    "attempts": [], "spells": [], "owner_snapshot": []}
        report = {"duration_ms": 2000, "measurement_complete": True, "dps": 0, "damage": 0,
                  "casts": {}, "damage_by_spell": {}, "findings": [], "native_errors": []}
        samples = [{"at_ms": stamp, "spells": {"casting": 0, "gcd_ms": gcd},
                    "assisted": {"evaluation_json": json.dumps({"facts": {"powers": {"1": power}}})}}
                   for stamp, gcd, power in [(0, 1500, 10), (1000, 500, 7), (2000, 0, 9)]]
        result = bot_ability_summary.summarize_run(manifest, report, [], samples)
        self.assertEqual(result["sampling"]["covered_ms"], 2000)
        self.assertEqual(result["sampling"]["sampled_gcd_busy_ms"], 1500)
        self.assertEqual(result["sampling"]["sampled_idle_ms"], 500)
        self.assertEqual(result["sampling"]["sampled_gcd_utilization"], .75)
        self.assertEqual(result["resource_net_change"]["powers"]["1"]["net"], -1)
        self.assertIsNone(result["resource_spent"])
        self.assertIn("resource_spending", result["unobserved"])

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
