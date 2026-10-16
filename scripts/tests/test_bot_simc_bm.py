"""Pinned BM APL behavior on retained native character47 metadata.

Charge clocks below are explicit arithmetic fixtures, not asserted Retail values.
"""
import copy
import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_simc_bm import BeastMasterySimcProvider, charge_clock

FIXTURE = Path(__file__).parent / "fixtures/bot_harness/simc_bm"


class BeastMasteryTests(unittest.TestCase):
    def setUp(self):
        self.state = json.loads((FIXTURE / "captured-native.json").read_text())
        self.state["spells"].update(casting=0, gcd_ms=0, cast_success_events=[],
                                     cast_failure_seq=0, cast_failure_events=[])
        self.state["spell_charge_state"] = {
            "217200": {"current": 2, "max": 2, "recharge_ms": 12000, "remaining_ms": 0},
            "34026": {"current": 1, "max": 1, "recharge_ms": 6000, "remaining_ms": 0}}
        self.state["spell_gcd_ms"] = {"217200": 1500, "19574": 1500, "34026": 1500, "193455": 1500}
        self.context = {"active_enemies": 1, "potion": False, "external_buffs": [], "on_use_items": []}
        self.provider = self.make_provider()
        self.start_auto_shot()

    def make_provider(self, receipt=None):
        return BeastMasterySimcProvider(
            receipt or json.loads((FIXTURE / "source.json").read_text()),
            FIXTURE / "hunter_beast_mastery.simc", self.state["spells"],
            FIXTURE, self.state, self.context)

    def start_auto_shot(self):
        self.state["spells"]["cast_success_events"].append({
            "source": self.state["actor"], "spell_id": 75, "at_ms": self.state["at_ms"] + 1})

    def choose(self):
        return self.provider(self.state, self.state["target"])

    def put_on_cooldown(self, spell, ms):
        self.state["facts"]["ready"][str(spell)] = {"Ok": False}
        self.state["facts"]["cooldowns_ms"][str(spell)] = ms

    def test_literal_st_order_barbed_then_wrath_not_native_wrath_first(self):
        self.assertEqual(self.choose()["spell"], 217200)
        self.state["spell_charge_state"]["217200"].update(current=1, remaining_ms=12000)
        self.assertEqual(self.choose()["spell"], 217200)
        self.state["spell_charge_state"]["217200"].update(current=0)
        self.assertEqual(self.choose()["spell"], 19574)

    def test_empty_native_charge_blocks_even_if_ready_boolean_is_stale(self):
        self.state["spell_charge_state"]["217200"].update(current=0, remaining_ms=1)
        self.assertEqual(self.choose()["spell"], 19574)

    def test_timed_recharge_predicate_uses_next_and_all_missing_charges(self):
        self.put_on_cooldown(19574, 90000)
        self.state["spell_charge_state"]["217200"].update(current=1, remaining_ms=1000)
        self.assertEqual(self.choose()["spell"], 217200)
        self.state["spell_charge_state"]["217200"].update(remaining_ms=2000)
        self.assertEqual(self.choose()["spell"], 193455)
        self.assertEqual(charge_clock({"current": 0, "max": 2, "recharge_ms": 12000, "remaining_ms": 1000}),
                         (13000, 11 / 12))

    def test_single_charge_and_absent_howl_does_not_invent_kill_command_permission(self):
        self.put_on_cooldown(19574, 90000)
        self.state["spell_charge_state"]["217200"].update(current=0, remaining_ms=12000)
        self.assertEqual(self.choose()["spell"], 193455)

    def test_literal_fractional_and_howl_ready_controls(self):
        self.put_on_cooldown(19574, 90000)
        self.state["spell_charge_state"]["217200"].update(current=0, remaining_ms=12000)
        self.state["spell_charge_state"]["34026"].update(max=2, remaining_ms=600)
        self.assertEqual(self.choose()["spell"], 34026)
        self.state["spell_charge_state"]["34026"].update(max=1, remaining_ms=0)
        self.state["facts"]["player_auras"].append(471878)
        self.assertEqual(self.choose()["spell"], 34026)

    def test_focus_condition_and_native_affordability_are_both_required(self):
        self.put_on_cooldown(19574, 90000)
        self.state["spell_charge_state"]["217200"].update(current=1, remaining_ms=12000)
        self.state["facts"]["powers"]["2"] = 50
        self.assertEqual(self.choose()["spell"], 217200)
        self.state["facts"]["affordable"]["217200"] = {"Ok": False}
        self.assertEqual(self.choose()["spell"], 193455)

    def test_missing_charge_authority_and_trait_drift_block(self):
        del self.state["spell_charge_state"]["217200"]
        with self.assertRaisesRegex(ValueError, "charge"):
            self.choose()
        self.state = json.loads((FIXTURE / "captured-native.json").read_text())
        self.state["trait_config"]["entries"][0]["rank"] += 1
        with self.assertRaisesRegex(ValueError, "trait"):
            self.choose()

    def test_corrupt_source_or_non_single_target_context_refuses(self):
        receipt = json.loads((FIXTURE / "source.json").read_text())
        receipt["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "source"):
            self.make_provider(receipt)
        self.context["active_enemies"] = 2
        with self.assertRaisesRegex(ValueError, "single.target"):
            self.make_provider()

    def test_auto_shot_start_requires_new_matching_owner_receipt(self):
        self.provider = self.make_provider()
        self.assertEqual(self.choose()["spell"], 75)
        self.state["spells"]["cast_success_events"].append({
            "source": 1, "spell_id": 75, "at_ms": self.state["at_ms"] + 2})
        self.assertEqual(self.choose()["spell"], 75)
        self.start_auto_shot()
        self.assertEqual(self.choose()["spell"], 217200)

    def test_native_busy_state_and_unavailable_readiness_never_submit(self):
        self.state["spells"]["gcd_ms"] = 200
        self.assertIsNone(self.choose())
        self.state["spells"]["gcd_ms"] = 0
        self.state["facts"]["ready"]["217200"] = {"Err": "not received"}
        with self.assertRaisesRegex(ValueError, "ready"):
            self.choose()


if __name__ == "__main__":
    unittest.main()
