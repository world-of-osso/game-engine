"""Behavioral controller tests; native file responses are produced by a child process."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_controller import FileBridge, PlayerController, requested_snapshot


def snapshot(spec=62, spell=30451):
    return {"at_ms": 1000, "target": 41, "actor": 7, "trait_config": {"spec_id": spec},
            "spells": {"spec": spec, "known": [spell], "casting": None, "gcd_ms": 0,
                       "cast_success_events": [], "cast_failure_events": [], "cast_failure_seq": 0},
            "facts": {"known": [spell], **{field: {str(spell): {"Ok": True}}
                       for field in ("ready", "affordable", "in_range")}}}


class ControllerTests(unittest.TestCase):
    def setUp(self):
        self.commands = []
        self.native = snapshot()
        self.controller = PlayerController(self.command, lambda target: self.native,
                                           lambda start, end, label: [end], None)

    def command(self, *argv):
        self.commands.append(argv)
        return "submitted"

    def test_two_specs_use_explicit_provider_without_mage_branch(self):
        for spec, spell in [(62, 30451), (71, 12294)]:
            self.native = snapshot(spec, spell)
            self.controller = PlayerController(self.command, lambda target: self.native,
                                               None, None)
            result = self.controller.cast_next(41, lambda state, target: {"spell": spell, "source": "test fixture"})
            self.assertEqual(result["spell"], spell)
            self.assertEqual(self.commands[-1], ("spell", "cast", "--spell", str(spell), "--target", "41"))

    def test_unselected_or_unlearned_or_unusable_action_never_submits(self):
        for change in [lambda s: s.update(target=42),
                       lambda s: s["facts"]["ready"].update({"30451": {"Err": "charge categories not loaded"}}),
                       lambda s: s["spells"].update(known=[])]:
            self.native = snapshot()
            change(self.native)
            with self.assertRaisesRegex(ValueError, "target|learned|usable|disagree"):
                self.controller.cast_next(41, lambda state, target: {"spell": 30451})
        self.assertEqual(self.commands, [])

    def test_provider_error_does_not_invoke_another_rotation(self):
        def blocked(state, target):
            raise ValueError("unsupported APL predicate")
        with self.assertRaisesRegex(ValueError, "unsupported APL predicate"):
            self.controller.cast_next(41, blocked)
        self.assertEqual(self.commands, [])

    def test_temporarily_unusable_spell_waits_without_a_request_or_alternate(self):
        for field in ["ready", "affordable", "in_range"]:
            self.native = snapshot()
            self.native["facts"][field]["30451"] = {"Ok": False}
            self.assertIsNone(self.controller.cast_next(41, lambda state, target: {"spell": 30451}))
        self.assertEqual(self.commands, [])

    def test_busy_player_never_submits(self):
        self.native["spells"]["casting"] = 30451
        self.assertIsNone(self.controller.cast_next(41, lambda state, target: {"spell": 30451}))
        self.assertEqual(self.commands, [])

    def test_route_uses_first_unreached_sourced_corner_not_direct_goal(self):
        self.controller.route = lambda start, goal, label: [[0, 0, 0], [7, 0, 0], goal]
        result = self.controller.step_toward([0, 0, 0], [21, 0, 0], "authored-leg")
        self.assertFalse(result)
        self.assertEqual(self.commands, [("movement", "forward", "--seconds", "0.55", "--yaw-degrees", "90.0")])

    def test_empty_route_blocks_without_straight_line_fallback(self):
        self.controller.route = lambda start, goal, label: []
        with self.assertRaisesRegex(ValueError, "route"):
            self.controller.step_toward([0, 0, 0], [21, 0, 0], "authored-leg")
        self.assertEqual(self.commands, [])

    def test_native_loot_choice_preserves_actual_visible_button_identity(self):
        tree = "GroupLootFrame1GreedButton [Button] visible enabled x=10 y=20 w=30 h=40\n"
        requests = []
        class Bridge:
            def request(self, kind, payload):
                requests.append((kind, payload))
                return {"input": "native viewport mouse", **payload}
        self.controller.bridge = Bridge()
        self.controller.invoke = lambda *argv: tree
        result = self.controller.vote_loot()
        self.assertEqual(result["name"], "GroupLootFrame1GreedButton")
        self.assertEqual(requests, [("loot-click", {"name": "GroupLootFrame1GreedButton", "x": 25.0, "y": 40.0})])

    def test_native_tab_accepts_only_observed_alive_hostile_without_entity_command(self):
        class Bridge:
            result = {"after": {"target": 41}, "selected_health": 100, "selected_hostile": True}
            def request(self, kind, payload):
                return self.result
        bridge = Bridge()
        self.controller.bridge = bridge
        self.assertEqual(self.controller.tab_to_enemy(), 41)
        for change in [{"selected_health": 0}, {"selected_hostile": False}, {"after": {"target": None}}]:
            bridge.result = {"after": {"target": 41}, "selected_health": 100,
                             "selected_hostile": True, **change}
            with self.assertRaisesRegex(ValueError, "hostile|alive"):
                self.controller.tab_to_enemy()
        self.assertEqual(self.commands, [])

    def test_native_assist_rejects_unchanged_leader_instead_of_target_shortcut(self):
        class Bridge:
            def request(self, kind, payload):
                return {"before": {"target": 4, "target_name": "Tank"}, "after": {"target": 4}}
        self.controller.bridge = Bridge()
        self.assertIsNone(self.controller.assist("Tank"))
        self.assertEqual(self.commands, [("quest", "interact", "--npc", "Tank")])


class CastAcknowledgementTests(unittest.TestCase):
    def setUp(self):
        self.native = snapshot(spell=217200)
        self.native["actor"] = 7
        self.native["spells"].update(cast_success_events=[], cast_failure_events=[],
                                      cast_failure_seq=0, combat_log_seq=0,
                                      combat_log_count=0, attack_start_events=[], attack_start_seq=0)
        self.commands = []
        self.controller = PlayerController(lambda *args: self.commands.append(args),
                                           lambda target: self.native, None, None)

    def cast(self):
        return self.controller.cast_next(41, lambda state, target: {"spell": 217200})

    def test_stale_ready_state_cannot_submit_again_before_real_outcome(self):
        self.assertIsNotNone(self.cast())
        for at in [1001, 2500, 5900, 9900]:
            self.native["at_ms"] = at
            self.assertIsNone(self.cast())
        self.assertEqual(len(self.commands), 1)
        self.native["spells"]["cast_success_events"] = [
            {"source": 7, "spell_id": 217200, "at_ms": 9800}]
        self.assertIsNotNone(self.cast())
        self.assertEqual(len(self.commands), 2)

    def test_old_foreign_and_other_spell_success_cannot_acknowledge_request(self):
        old = {"source": 7, "spell_id": 217200, "at_ms": 500}
        self.native["spells"]["cast_success_events"] = [old]
        self.cast()
        self.native["spells"]["cast_success_events"] += [
            {"source": 8, "spell_id": 217200, "at_ms": 1001},
            {"source": 7, "spell_id": 34026, "at_ms": 1001}]
        self.assertIsNone(self.cast())
        self.assertEqual(len(self.commands), 1)

    def test_distinct_identical_failure_receipt_releases_each_request(self):
        self.cast()
        for sequence in [1, 2]:
            self.native["spells"]["cast_failure_seq"] = sequence
            self.native["spells"]["cast_failure_events"].append(
                {"sequence": sequence, "spell_id": 217200, "reason": "NoCharges", "detail": None})
            self.assertIsNotNone(self.cast())
        self.assertEqual(len(self.commands), 3)
        self.assertEqual([row["reason"] for row in self.controller.outcomes], ["NoCharges", "NoCharges"])

    def test_server_rejection_does_not_bypass_new_unusable_state(self):
        self.cast()
        self.native["spells"].update(cast_failure_seq=1, cast_failure_events=[
            {"sequence": 1, "spell_id": 217200, "reason": "NotReady"}])
        self.native["facts"]["ready"]["217200"] = {"Ok": False}
        self.assertIsNone(self.cast())
        self.assertEqual(len(self.commands), 1)


class SnapshotTests(unittest.TestCase):
    def test_native_query_target_and_authoritative_traits_are_preserved(self):
        query = {"target": 41, "facts": {"known": [12294]}, "trait_config": {"spec_id": 71}}
        state = {"priority": query, "spells": {"known": [12294]}, "at_ms": 1000,
                 "account": {"local_player_id": 7}}
        result = requested_snapshot(state, 41)
        self.assertEqual(result, {**query, "spells": state["spells"], "at_ms": 1000, "actor": 7})
        self.assertEqual(state["priority"], query)

    def test_unavailable_or_other_target_facts_block(self):
        for query in [{"error": "native query loading"}, {"target": 42}]:
            with self.assertRaisesRegex(ValueError, "loading|target"):
                requested_snapshot({"priority": query}, 41)


class BridgeTests(unittest.TestCase):
    def test_atomic_process_response_correlation_and_native_errors(self):
        import subprocess
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            # Real separate process observes published request and emits stale then current ACK.
            program = '''import json,sys,time
from pathlib import Path
p=Path(sys.argv[1]); request=p/'native-assist-request.json'
while not request.exists(): time.sleep(.005)
r=json.loads(request.read_text())
(p/'native-assist-result.json').write_text(json.dumps({'request_id':'old'}))
time.sleep(.05)
t=p/'response.writing';t.write_text(json.dumps({'request_id':r['request_id'],'error':'wrong leader'}));t.replace(p/'native-assist-result.json')
'''
            child = subprocess.Popen([sys.executable, "-c", program, directory])
            try:
                with self.assertRaisesRegex(RuntimeError, "wrong leader"):
                    FileBridge(path, timeout=2).request("assist", {"leader_name": "Tank"})
            finally:
                try:
                    child.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
            self.assertEqual(child.returncode, 0)
            self.assertFalse((path / "native-assist-request.json").exists(),
                             "Completed request must not be visible to a restarted native client")

    def test_native_enemy_tab_receipt_crosses_a_real_process_boundary(self):
        import subprocess
        with tempfile.TemporaryDirectory() as directory:
            program = '''import json,sys,time
from pathlib import Path
p=Path(sys.argv[1]); request=p/'native-enemy-tab-request.json'
while not request.exists(): time.sleep(.005)
r=json.loads(request.read_text())
result={'request_id':r['request_id'],'after':{'target':41},'selected_health':100,'selected_hostile':True}
t=p/'response.writing';t.write_text(json.dumps(result));t.replace(p/'native-enemy-tab-result.json')
'''
            child = subprocess.Popen([sys.executable, "-c", program, directory])
            try:
                result = FileBridge(directory, timeout=2).request("enemy-tab", {})
                self.assertEqual(result["after"]["target"], 41)
                self.assertTrue(result["selected_hostile"])
            finally:
                if child.poll() is None:
                    child.terminate()
                child.wait(timeout=3)

    def test_timeout_does_not_resubmit_input(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(TimeoutError, "assist"):
                FileBridge(Path(directory), timeout=.03).request("assist", {"leader_name": "Tank"})
            self.assertFalse((Path(directory) / "native-assist-request.json").exists(),
                             "Expired request must not be replayed by a restarted native client")


if __name__ == "__main__":
    unittest.main()
