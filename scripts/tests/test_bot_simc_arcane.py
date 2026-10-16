"""Pinned SimC Arcane provider over captured native fixtures; offline decisions, not live-game proof."""
import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_controller import PlayerController
from bot_simc_arcane import ArcaneSimcProvider

FIXTURES = Path(__file__).resolve().parent / "fixtures/bot_harness/simc_arcane32"
COMMIT = "515f4a3fe9d1f35e1dbaf253745b6105fba906fb"
APL_SHA256 = "b1d85887ec62431864cc2db45b95993868bc02a0c0852b5189997542f47cabd4"


def load(name):
    return json.loads((FIXTURES / name).read_text())


def captured_snapshot():
    """Captured native facts + kit; overrides below each test are explicit scenario inputs."""
    state = load("facts-result.json")["state"]
    kit = load("native-spells-state.json")
    snapshot = {**copy.deepcopy(state), "spells": copy.deepcopy(kit), "at_ms": 1000}
    for spell in ("2139", "30451", "44425"):
        snapshot["facts"]["in_range"][spell] = {"Ok": True}
    snapshot["facts"]["powers"]["16"] = 4
    return snapshot


def dungeon_route(**overrides):
    return {"fight_style": "DUNGEON_ROUTE", "expected_iteration_s": 2700, "started_at_ms": 0,
            "potion": False, "external_buffs": [], "on_use_items": [], **overrides}


def provider(receipt=None, apl=FIXTURES / "mage_arcane.simc", kit=None, initial=None, context=None):
    return ArcaneSimcProvider(
        receipt=load("source-receipt.json") if receipt is None else receipt, apl_path=apl,
        captured_kit=load("native-spells-state.json") if kit is None else kit,
        db2_dir=FIXTURES / "db2", initial_snapshot=captured_snapshot() if initial is None else initial,
        context=dungeon_route() if context is None else context)


class ArcaneSimcProviderTests(unittest.TestCase):
    def setUp(self):
        self.provider = provider()
        self.target = int(captured_snapshot()["target"])

    def choose(self, snapshot):
        before = copy.deepcopy(snapshot)
        try:
            return self.provider(snapshot, self.target)
        finally:
            self.assertEqual(snapshot, before, "provider mutated the native snapshot")

    def blocked(self, snapshot):
        with self.assertRaises(ValueError) as caught:
            self.choose(snapshot)
        return str(caught.exception).lower()

    # Original 13 cases.
    def test_native_known_spells_resolve_combat_actions_and_no_apl_talents(self):
        context = self.provider.context
        self.assertEqual(set(context["known_apl_actions"]), {"counterspell", "arcane_blast", "arcane_barrage"})
        self.assertEqual(context["selected_talents"], [])
        self.assertEqual(context["trait_spells"], [235450])
        self.assertEqual(context["started_at_ms"], 0)
        self.assertEqual(context["expected_iteration_s"], 2700)

    def test_context_without_authoritative_trait_preflight_refuses(self):
        initial = captured_snapshot()
        del initial["trait_config"]
        with self.assertRaisesRegex(ValueError, "trait"):
            provider(initial=initial)

    def test_changed_native_trait_config_is_not_silently_reused(self):
        packet = captured_snapshot()
        packet["trait_config"]["entries"][0]["rank"] = 2
        self.assertIn("trait", self.blocked(packet))

    def test_native_extra_apl_spell_is_a_kit_gap(self):
        packet = captured_snapshot()
        packet["spells"]["known"].append(5143)
        packet["facts"]["known"].append(5143)
        self.assertIn("kit", self.blocked(packet))

    def test_base_kit_reaches_pinned_sunfury_filler(self):
        action = self.choose(captured_snapshot())
        self.assertEqual(action, {"spell": 30451, "apl_line": 64, "source_commit": COMMIT,
                                  "source_sha256": APL_SHA256})

    def test_no_invented_barrage_on_oom(self):
        packet = captured_snapshot()
        packet["facts"]["affordable"]["30451"] = {"Ok": False}
        self.assertIsNone(self.choose(packet))

    def test_declared_dungeon_end_guard_spends_four_charges(self):
        packet = captured_snapshot()
        packet["at_ms"] = 2698000
        action = self.choose(packet)
        self.assertEqual((action["spell"], action["apl_line"]), (44425, 36))

    def test_end_guard_does_not_spend_three_charges(self):
        packet = captured_snapshot()
        packet["at_ms"] = 2698000
        packet["facts"]["powers"]["16"] = 3
        self.assertEqual(self.choose(packet)["spell"], 30451)

    def test_ready_counterspell_precedes_damage_when_target_casts(self):
        packet = captured_snapshot()
        packet["target_cast"].update(casting=True, interruptible=True)
        action = self.choose(packet)
        self.assertEqual((action["spell"], action["apl_line"]), (2139, 24))

    def test_counterspell_cooldown_permits_sourced_damage(self):
        packet = captured_snapshot()
        packet["target_cast"].update(casting=True, interruptible=True)
        packet["facts"]["ready"]["2139"] = {"Ok": False}
        self.assertEqual(self.choose(packet)["spell"], 30451)

    def test_unknown_fight_context_is_not_target_ttd_or_false(self):
        context = dungeon_route()
        del context["expected_iteration_s"]
        with self.assertRaisesRegex(ValueError, "fight"):
            provider(context=context)
        with self.assertRaisesRegex(ValueError, "fight"):
            provider(context=dungeon_route(expected_iteration_s=300))

    def test_selected_talents_remain_an_explicit_coverage_gap(self):
        initial = captured_snapshot()
        initial["trait_config"]["entries"].append({"entry_id": 117267, "node_id": 0, "rank": 1})
        with self.assertRaisesRegex(ValueError, "talent"):
            provider(initial=initial)

    def test_cannot_silently_drop_a_newly_learned_apl_action(self):
        kit = load("native-spells-state.json")
        kit["known"].append(5143)
        initial = captured_snapshot()
        initial["spells"]["known"].append(5143)
        initial["facts"]["known"].append(5143)
        with self.assertRaisesRegex(ValueError, "kit"):
            provider(kit=kit, initial=initial)

    # Source receipt and APL bytes.
    def test_receipt_must_name_the_approved_pin(self):
        for field, value in [("commit", "0" * 40), ("sha256", "0" * 64),
                             ("raw_url", load("source-receipt.json")["raw_url"].replace("515f4a3", "deadbee"))]:
            receipt = {**load("source-receipt.json"), field: value}
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "receipt"):
                provider(receipt=receipt)

    def test_changed_apl_bytes_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            changed = Path(directory) / "mage_arcane.simc"
            changed.write_bytes((FIXTURES / "mage_arcane.simc").read_bytes() + b"\n")
            with self.assertRaisesRegex(ValueError, "APL bytes"):
                provider(apl=changed)

    # Captured kit, target and preflight boundaries.
    def test_unsupported_captured_kit_is_rejected(self):
        for field, value in [("spec", 63), ("level", 33)]:
            kit = {**load("native-spells-state.json"), field: value}
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "kit"):
                provider(kit=kit)

    def test_initial_snapshot_must_match_captured_kit(self):
        initial = captured_snapshot()
        initial["spells"]["known"].remove(44425)
        initial["facts"]["known"].remove(44425.0)
        with self.assertRaisesRegex(ValueError, "kit"):
            provider(initial=initial)

    def test_consumables_external_buffs_and_on_use_items_are_out_of_scope(self):
        for field, value in [("potion", True), ("external_buffs", ["power_infusion"]),
                             ("on_use_items", ["font_of_venomous_rage"])]:
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "Consumable"):
                provider(context=dungeon_route(**{field: value}))

    def test_wrong_target_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "target"):
            self.provider(captured_snapshot(), self.target + 1)

    def test_snapshot_before_declared_fight_start_is_rejected(self):
        guarded = provider(context=dungeon_route(started_at_ms=5000))
        with self.assertRaisesRegex(ValueError, "fight"):
            guarded(captured_snapshot(), self.target)

    def test_preflight_inputs_are_frozen_against_caller_mutation(self):
        initial = captured_snapshot()
        frozen = provider(initial=initial)
        initial["trait_config"]["entries"][0]["rank"] = 2
        self.assertEqual(frozen(captured_snapshot(), self.target)["spell"], 30451)

    def test_controller_casts_provider_selection_directly(self):
        commands = []
        native = captured_snapshot()
        controller = PlayerController(lambda *argv: commands.append(argv) or "submitted",
                                      lambda target: native, lambda *route: [], None)
        result = controller.cast_next(self.target, self.provider)
        self.assertEqual((result["spell"], result["apl_line"]), (30451, 64))
        self.assertEqual(commands, [("spell", "cast", "--spell", "30451", "--target", str(self.target))])


if __name__ == "__main__":
    unittest.main()
