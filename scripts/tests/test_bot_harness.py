"""Behavioral tests use actual native IPC combat, never fabricated damage values."""
import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bot_harness as harness

FIXTURE = Path(__file__).parent / "fixtures/bot_harness/native-combat.log"
ACTOR = "4294966795"


class BotHarnessTests(unittest.TestCase):
    def setUp(self):
        self.events = harness.parse_combat(FIXTURE.read_text())
        self.run = {
            "actor": ACTOR, "target": "4294966750", "start_ms": 1791992753000,
            "end_ms": 1791992759000, "scenario": "dummy", "spec": 62, "measurement_complete": True,
            "spells": [{"id": 30451, "damage": True}, {"id": 44425, "damage": True}],
            "attempts": [{"spell": 44425, "at_ms": 1791992756000, "deadline_ms": 1791992758500}],
        }

    def test_actual_native_request_without_cast_success_is_flagged(self):
        directory = FIXTURE.parent
        run = json.loads((directory / "native-ret-run.json").read_text())
        events = harness.parse_combat((directory / "native-ret-combat.log").read_text())
        report = harness.analyze(run, events)
        self.assertIn({"kind": "cast_never_fired", "spell": 20271, "at_ms": 1792056099829}, report["findings"])
        self.assertEqual(report["casts"], {20271: 3})
        self.assertEqual(report["damage"], 899)
        self.assertIsNone(report["dps"])

    def test_known_missing_cast_in_real_log_is_reported(self):
        report = harness.analyze(self.run, self.events)
        self.assertIn({"kind": "cast_never_fired", "spell": 44425, "at_ms": 1791992756000}, report["findings"])
        self.assertIn({"kind": "ability_never_used", "spell": 44425}, report["findings"])

    def test_damage_excludes_followers_and_uses_full_fixed_duration(self):
        report = harness.analyze(self.run, self.events)
        expected = sum(e["amount"] for e in self.events if e["kind"] == "damage" and e["src"] == ACTOR)
        self.assertEqual(report["damage"], expected)
        self.assertEqual(report["dps"], expected / 6)
        self.assertGreater(expected, 0)

    def test_killed_ambient_npc_is_not_reported_as_a_bot_or_follower_death(self):
        events = harness.parse_combat((FIXTURE.parent / "native-ambient-death.log").read_text())
        self.run.update(actor="4294739522", start_ms=1792057507000, end_ms=1792057508000)
        report = harness.analyze(self.run, events)
        self.assertFalse(any(f["kind"] == "death" for f in report["findings"]))
        self.run["party_actors"] = ["4294959697"]
        report = harness.analyze(self.run, events)
        self.assertIn({"kind": "death", "actor": "4294959697", "at_ms": 1792057507198}, report["findings"])

    def test_aborted_observation_does_not_call_pending_request_missing(self):
        self.run["observed_until_ms"] = 1791992756100
        report = harness.analyze(self.run, self.events)
        self.assertNotIn({"kind": "cast_never_fired", "spell": 44425, "at_ms": 1791992756000}, report["findings"])
        self.assertEqual(report["pending_casts"], [self.run["attempts"][0]])

    def test_missing_damage_is_distinct_from_missing_cast(self):
        events = [e for e in self.events if not (e["src"] == ACTOR and e["kind"] == "damage")]
        report = harness.analyze(self.run, events)
        self.assertIn({"kind": "zero_damage_spell", "spell": 30451}, report["findings"])
        self.assertNotIn({"kind": "zero_damage_spell", "spell": 44425}, report["findings"])

    def test_nested_request_windows_do_not_create_a_false_missing_cast(self):
        stamps = [e["at_ms"] for e in self.events if e["kind"] == "cast_success" and e["src"] == ACTOR]
        first, last = min(stamps), max(stamps)
        self.run["attempts"] = [
            dict(spell=30451, at_ms=first - 100, deadline_ms=last + 100),
            dict(spell=30451, at_ms=first - 100, deadline_ms=first + 1),
        ]
        findings = harness.analyze(self.run, self.events)["findings"]
        self.assertFalse(any(f["kind"] == "cast_never_fired" for f in findings))

    def test_overlapping_missing_cast_cannot_claim_an_exact_request_timestamp(self):
        event = next(e for e in self.events if e["kind"] == "cast_success" and e["src"] == ACTOR)
        self.run["attempts"] = [dict(spell=30451, at_ms=event["at_ms"] - offset, deadline_ms=event["at_ms"] + 100) for offset in [100, 50]]
        findings = harness.analyze(self.run, [event])["findings"]
        missing = [f for f in findings if f["kind"] == "cast_never_fired"]
        self.assertEqual(len(missing), 1)
        self.assertIsNone(missing[0]["at_ms"])
        self.assertEqual(missing[0]["attribution"], "overlapping request windows")

    def test_same_cast_success_cannot_satisfy_two_attempts(self):
        self.run["attempts"] = [dict(spell=30451, at_ms=1791992755700, deadline_ms=1791992756000)] * 2
        findings = harness.analyze(self.run, self.events)["findings"]
        self.assertEqual(sum(f["kind"] == "cast_never_fired" for f in findings), 1)

    def test_overlapping_polls_preserve_duplicate_events_without_double_counting(self):
        text = FIXTURE.read_text()
        lines = [line for line in text.splitlines() if line.startswith("damage ")]
        merged = harness.merge_polls([text, text, lines[0] + "\n" + lines[0]])
        count = sum(e["text"] == lines[0] for e in merged)
        self.assertEqual(count, 2)
        self.assertEqual(len(merged), len(self.events) + 1)

    def test_ring_overflow_uses_observed_retention_not_requested_lines(self):
        self.assertTrue(harness.poll_has_gap(1000, 1201, 200))
        self.assertFalse(harness.poll_has_gap(1000, 1200, 200))

    def test_incomplete_measurements_cannot_be_balance_evidence(self):
        self.run["measurement_complete"] = False
        self.assertIsNone(harness.analyze(self.run, self.events)["dps"])
        with self.assertRaisesRegex(ValueError, "complete"):
            harness.balance([{"dps": 10, "spec": 62, "role": 2, "comparison": {"level": 32}}])

    def test_native_cast_rejections_are_machine_readable_without_invented_spell_id(self):
        self.run["native_errors"] = [{"at_ms": 1791992756000, "errors": ["No charges remain."]}]
        findings = harness.analyze(self.run, self.events)["findings"]
        self.assertIn({"kind": "native_cast_error", "at_ms": 1791992756000, "error": "No charges remain.", "spell": None}, findings)

    def test_server_error_and_cast_error_remain_findings_without_success(self):
        self.run["attempts"][0]["error"] = "OutOfRange"
        self.run["server_errors"] = ["ERROR failed effect"]
        findings = harness.analyze(self.run, self.events)["findings"]
        self.assertIn({"kind": "cast_error", "spell": 44425, "error": "OutOfRange"}, findings)
        self.assertIn({"kind": "server_error", "error": "ERROR failed effect"}, findings)

    def test_outside_window_and_wrong_target_are_not_damage(self):
        self.run["target"] = "999"
        self.assertEqual(harness.analyze(self.run, self.events)["damage"], 0)
        self.run["start_ms"] = self.run["end_ms"]
        with self.assertRaises(ValueError):
            harness.analyze(self.run, self.events)

    def test_unknown_aura_effect_is_not_falsely_certified(self):
        report = harness.analyze(self.run, self.events)
        self.assertIn("aura_effect", report["unobserved"])
        self.assertIn("charges", report["unobserved"])

    def test_matching_level_alone_is_not_a_comparable_stat_receipt(self):
        rows = [{"spec": 62, "role": 2, "dps": 100, "measurement_complete": True, "comparison": {"level": 32}}]
        with self.assertRaisesRegex(ValueError, "stats.*dummy.*duration_ms"):
            harness.balance(rows)

    def test_median_weights_dps_specs_not_run_counts_or_tank_damage(self):
        common = {"measurement_complete": True, "comparison": {"level": 32, "stats": "same", "dummy": 44548, "duration_ms": 60000}}
        rows = [dict(common, spec=s, role=2, dps=d) for s, d in [(62, 100), (62, 300), (63, 400), (64, 600)]]
        rows.append(dict(common, spec=66, role=0, dps=999999))
        report = harness.balance(rows)
        self.assertEqual(report["median_dps"], 400)
        self.assertEqual(report["max_over_median"], 1.5)
        self.assertEqual(report["min_over_median"], .5)
        self.assertEqual(len(report["specs"]), 3)

    def test_spread_has_no_pass_fail_and_rejects_unmatched_comparisons(self):
        rows = [{"spec": s, "dps": d, "role": 2, "measurement_complete": True, "comparison": {"level": 32, "stats": "same", "dummy": 44548, "duration_ms": 60000}} for s, d in [(62, 100), (63, 200), (64, 300)]]
        result = harness.balance(rows)
        self.assertEqual(result["max_over_median"], 1.5)
        self.assertEqual(result["min_over_median"], .5)
        self.assertNotIn("passed", result)
        rows[0]["comparison"]["level"] = 25
        with self.assertRaises(ValueError):
            harness.balance(rows)


if __name__ == "__main__":
    unittest.main()
