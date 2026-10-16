"""Dungeon accounting uses native identities, death events and authoritative credit."""
import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_harness import parse_combat
from bot_dungeon import summarize_dungeon


def event(kind, source, target, spell, amount, at):
    return (f'{kind} src={source} dst={target} spell={spell} amount={amount} '
            f'aura=- text=native timestamp_unix_ms={at} spell_name="fixture"')


class DungeonReportTests(unittest.TestCase):
    def setUp(self):
        self.run = {"spec": 72, "role": 2, "actor": "1", "party_actors": ["2", "3", "4", "5"],
                    "start_ms": 1000, "end_ms": 10000, "capture_complete": True,
                    "clear_elapsed_ms": 8000, "elapsed_ms": 9000,
                    "bosses": [{"entry": entry, "entity": str(actor), "encounter": encounter,
                                "name": name, "state": "Done"}
                               for entry, actor, encounter, name in
                               [(46383, 11, 1146, "Randolph Moloch"),
                                (46264, 12, 1145, "Lord Overheat"), (46254, 13, 1144, "Hogger")]]}
        self.events = parse_combat("\n".join([
            event("damage", 2, 11, 31935, 80, 2000),
            event("damage", 11, 2, 1, 20, 2200),
            event("damage", 1, 11, 1464, 100, 2500),
            event("summon", 1, 6, 228562, "-", 2600),
            event("summon", 2, 7, 1, "-", 2900),
            event("damage", 6, 11, 1, 50, 3000),
            event("damage", 1, 99, 1464, 999, 3000),
            event("damage", 7, 11, 1, 60, 3100),
            event("damage", 1, 11, 1464, 10, 4000),
            event("death", 1, 11, "-", "-", 4000),
            event("death", 11, 1, "-", "-", 4500),
            event("death", 11, 3, "-", "-", 4600),
            event("death", 11, 88, "-", "-", 4700),
            event("damage", 2, 12, 31935, 70, 5000),
            event("damage", 1, 12, 1464, 80, 5500),
            event("death", 1, 12, "-", "-", 6000),
            event("damage", 1, 13, 1464, 60, 7000),
            event("damage", 1, 13, 1464, 40, 8000),
            event("death", 1, 13, "-", "-", 8000),
        ]))

    def test_unconfirmed_native_owner_still_reports_every_expected_boss_as_unknown(self):
        self.run['actor'] = None
        for boss in self.run['bosses']:
            del boss['entity']
        result = summarize_dungeon(self.run, [])
        self.assertFalse(result['clear_complete'])
        self.assertIsNone(result['clear_time_ms'])
        self.assertIsNone(result['player_deaths'])
        self.assertIsNone(result['party_deaths'])
        self.assertEqual(len(result['bosses']), 3)
        self.assertTrue(all(row['dps'] is None for row in result['bosses']))
        self.assertFalse(result['metrics_complete'])

    def test_clear_requires_all_credit_and_owns_only_player_pet_boss_damage(self):
        result = summarize_dungeon(self.run, self.events)
        self.assertTrue(result["clear_complete"])
        self.assertEqual(result["clear_time_ms"], 8000)
        self.assertEqual(result["player_deaths"], 1)
        self.assertEqual(result["party_deaths"], 2)
        boss = result["bosses"][0]
        self.assertEqual((boss["damage"], boss["duration_ms"], boss["dps"]), (160, 2000, 80))
        self.assertEqual([row["dps"] for row in result["bosses"]], [80, 80, 100])
        self.assertEqual(boss["damage_by_actor"], {"1": 110, "6": 50})

    def test_summon_history_before_entry_keeps_owned_damage_attribution(self):
        events = copy.deepcopy(self.events)
        for row in events:
            if row["kind"] == "summon" and row["src"] == "1":
                row["at_ms"] = 900
        result = summarize_dungeon(self.run, events)
        self.assertEqual(result["bosses"][0]["damage"], 160)
        self.assertEqual(result["bosses"][0]["dps"], 80)

    def test_death_without_authoritative_done_is_not_a_clear_or_qualified_dps(self):
        self.run["bosses"][2]["state"] = "InProgress"
        result = summarize_dungeon(self.run, self.events)
        self.assertFalse(result["clear_complete"])
        self.assertIsNone(result["clear_time_ms"])
        self.assertIsNone(result["bosses"][2]["dps"])
        self.assertEqual(result["bosses"][2]["damage"], 100)

    def test_done_without_native_boss_death_or_capture_continuity_is_unqualified(self):
        events = [row for row in self.events if not (row["kind"] == "death" and row["dst"] == "12")]
        result = summarize_dungeon(self.run, events)
        self.assertIsNone(result["bosses"][1]["dps"])
        self.assertFalse(result["metrics_complete"])
        self.assertTrue(result["clear_complete"], "authoritative credit is separate from missing combat metrics")
        self.run["capture_complete"] = False
        result = summarize_dungeon(self.run, self.events)
        self.assertTrue(all(row["dps"] is None for row in result["bosses"]))

    def test_unknown_positive_boss_damage_withholds_dps_not_authoritative_credit(self):
        extra = parse_combat(event("damage", 777, 11, 1, 20, 3500))
        result = summarize_dungeon(self.run, self.events + extra)
        self.assertIsNone(result["bosses"][0]["dps"])
        self.assertIn("owned_damage_attribution", result["bosses"][0]["unobserved"])
        self.assertFalse(result["metrics_complete"])
        self.assertTrue(result["clear_complete"])

    def test_unstarted_boss_and_missing_terminal_observation_never_invent_duration(self):
        self.run["bosses"][2]["entity"] = "555"
        result = summarize_dungeon(self.run, self.events)
        self.assertIsNone(result["bosses"][2]["duration_ms"])
        self.assertIsNone(result["bosses"][2]["dps"])
        self.run["clear_elapsed_ms"] = None
        self.assertIsNone(summarize_dungeon(self.run, self.events)["clear_time_ms"])
        run = copy.deepcopy(self.run)
        run["start_ms"] = run["end_ms"]
        with self.assertRaises(ValueError):
            summarize_dungeon(run, self.events)


if __name__ == "__main__":
    unittest.main()
