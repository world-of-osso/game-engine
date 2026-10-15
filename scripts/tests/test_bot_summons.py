"""Accounting fixtures, not Retail damage values; server tests prove real summons."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bot_harness


def event(kind, source, target, spell, amount, at):
    return f'{kind} src={source} dst={target} spell={spell} amount={amount} aura=- text=fixture timestamp_unix_ms={at} spell_name="fixture"'


class SummonAccountingTests(unittest.TestCase):
    def setUp(self):
        self.run = {"actor": "42", "target": "999", "start_ms": 1000, "end_ms": 2000,
                    "scenario": "dummy", "spec": 263, "spells": [], "attempts": [],
                    "measurement_complete": True}

    def analyze(self, *lines):
        return bot_harness.analyze(self.run, bot_harness.parse_combat("\n".join(lines)))

    def test_short_lived_guardian_needs_no_later_unit_snapshot(self):
        report = self.analyze(event("summon", 42, 80, 228562, "-", 1000),
                              event("damage", 80, 999, 58867, 12, 1100),
                              event("damage", 42, 999, 17364, 3, 1200))
        self.assertEqual(report["damage"], 15)
        self.assertEqual(report["dps"], 15)
        self.assertTrue(report.get("attribution_complete"))
        self.assertEqual(report["damage_by_actor"], {"42": 3, "80": 12})

    def test_persistent_pet_uses_observed_owner_relation_without_fake_summon(self):
        self.run["owner_snapshot"] = [{"unit": "80", "owner": "42", "at_ms": 900}]
        report = self.analyze(event("damage", 80, 999, 3110, 12, 1100))
        self.assertEqual(report["damage"], 12)
        self.assertTrue(report.get("attribution_complete"))

    def test_follower_and_its_pet_are_not_player_damage(self):
        self.run["party_actors"] = ["50"]
        report = self.analyze(event("summon", 50, 51, 688, "-", 1000),
                              event("damage", 50, 999, 133, 100, 1100),
                              event("damage", 51, 999, 3110, 50, 1200),
                              event("damage", 42, 999, 17364, 3, 1300))
        self.assertEqual(report["damage"], 3)
        self.assertTrue(report.get("attribution_complete"))

    def test_unknown_or_before_summon_damage_cannot_be_certified(self):
        report = self.analyze(event("damage", 80, 999, 58867, 12, 1100),
                              event("summon", 42, 80, 228562, "-", 1200))
        self.assertFalse(report["measurement_complete"])
        self.assertIsNone(report["dps"])
        self.assertIn("owned_damage_attribution", report["unobserved"])

    def test_summon_requires_a_real_spell_and_consistent_owner(self):
        with self.assertRaisesRegex(ValueError, "summoning spell"):
            self.analyze(event("summon", 42, 80, "-", "-", 1000))
        with self.assertRaisesRegex(ValueError, "Conflicting"):
            self.analyze(event("summon", 42, 80, 228562, "-", 1000),
                         event("summon", 43, 80, 228562, "-", 1100))


if __name__ == "__main__":
    unittest.main()
