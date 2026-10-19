"""Concrete native observation -> path-map trace behavior."""
import os
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_path_trace import compile_trace


class BotPathTraceTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get("BOT_PATH_TEST_RED"), "behavioral RED: trace capture not implemented")
    def test_native_poses_combat_edges_and_death_keep_observed_identity_and_clocks(self):
        samples = [
            {"at_ms": 1000, "monotonic_ns": 10, "map_id": 34,
             "actors": {"101": {"position": [10, 2, 20], "facing": 0.5}},
             "in_combat": False},
            {"at_ms": 1100, "monotonic_ns": 100000010, "map_id": 34,
             "actors": {"101": {"position": [11, 2, 21], "facing": 0.75}},
             "in_combat": True},
            {"at_ms": 1200, "monotonic_ns": 200000010, "map_id": 34,
             "actors": {"101": {"position": [12, 2, 22], "facing": 1.0}},
             "in_combat": False},
        ]
        run = {"run_id": "small-stockade", "revision": "abc123", "actor": "101",
               "party_names": {"101": "Tank"}, "start_ms": 1000, "end_ms": 1300}
        combat = [{"kind": "death", "dst": "101", "at_ms": 1200}]
        trace = compile_trace(samples, combat, run)
        poses = [row for row in trace if row["event"] == "pose"]
        self.assertEqual([row["position"] for row in poses], [[10, 2, 20], [11, 2, 21], [12, 2, 22]])
        self.assertEqual([row["facing"] for row in poses], [0.5, 0.75, 1.0])
        self.assertEqual(poses[1]["monotonic_ns"], 100000010)
        self.assertIsNone(poses[1]["simulation_s"])
        self.assertEqual([row["event"] for row in trace if row["event"] in {"combat_start", "combat_end", "death"}],
                         ["combat_start", "combat_end", "death"])
        self.assertEqual(trace[0]["run_id"], "small-stockade")
        self.assertEqual(trace[-1]["event"], "run_end")

    @unittest.skipUnless(os.environ.get("BOT_PATH_TEST_RED"), "behavioral RED: roster wipe capture not implemented")
    def test_wipe_requires_every_observed_party_member_dead_not_target_changes(self):
        run = {"run_id": "wipe-fixture", "revision": "abc123", "actor": "101",
               "party_names": {"101": "Tank", "102": "Healer"},
               "start_ms": 1000, "end_ms": 1300,
               "party_observations": [
                   {"at_ms": 1100, "monotonic_ns": 100, "members": [
                       {"name": "Tank", "health": 0, "death": "Corpse", "position": [1, 2, 3]},
                       {"name": "Healer", "health": 50, "death": "Alive", "position": [2, 2, 3]}]},
                   {"at_ms": 1200, "monotonic_ns": 200, "members": [
                       {"name": "Tank", "health": 0, "death": "Corpse", "position": [1, 2, 3]},
                       {"name": "Healer", "health": 0, "death": "Corpse", "position": [2, 2, 3]}]}]}
        trace = compile_trace([], [], run)
        wipes = [row for row in trace if row["event"] == "wipe"]
        self.assertEqual([row["at_ms"] for row in wipes], [1200])
        self.assertEqual(wipes[0]["monotonic_clock"], "python_host_monotonic")
        self.assertFalse(any(row["event"] == "overlap_pull" for row in trace))

    @unittest.skipUnless(os.environ.get("BOT_PATH_TEST_RED"), "behavioral RED: exact boss identity mapping not implemented")
    def test_only_known_boss_entity_death_marks_its_source_spawn_dead(self):
        run = {"run_id": "boss-fixture", "revision": "abc123", "actor": "101",
               "party_names": {"101": "Tank"}, "start_ms": 1000, "end_ms": 1300,
               "bosses": [{"spawn_guid": 900, "entity": "201"}, {"spawn_guid": 901, "entity": "202"}]}
        trace = compile_trace([], [{"kind": "death", "dst": "201", "at_ms": 1200},
                                   {"kind": "death", "dst": "999", "at_ms": 1250}], run)
        states = [row for row in trace if row["event"] == "spawn_state"]
        self.assertEqual([(row["spawn_guid"], row["alive"]) for row in states], [(900, False)])
        self.assertEqual(states[0]["at_ms"], 1200)


if __name__ == "__main__":
    unittest.main()
