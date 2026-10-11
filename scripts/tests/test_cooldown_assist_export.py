"""Pinned local-CASC behavioral export proof, not source-shape assertions."""
import csv
import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "export_db2_csv.py"
SPEC = importlib.util.spec_from_file_location("export_cooldown_assist", SCRIPT)
EXPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(EXPORT)
ASSETS = Path(__file__).resolve().parents[2] / "data/diagnostics/cooldown-assist-2026-10-11"


class CooldownAssistExportTests(unittest.TestCase):
    def export(self, table, fdid):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "table.csv"
            with patch("sys.argv", [str(SCRIPT), table, str(ASSETS / f"{fdid}.db2"), str(output)]):
                EXPORT.main()
            with output.open() as stream:
                return list(csv.DictReader(stream))

    def test_authored_spec_categories_and_order(self):
        sets = self.export("CooldownSet", 6405756)
        self.assertIn({"ID": "12", "ChrSpecializationID": "71", "Field_11_1_5_59571_002": "0", "PlayerConditionID": "0"}, sets)
        spells = self.export("CooldownSetSpell", 6405757)
        strike = next(row for row in spells if row["ID"] == "27")
        self.assertEqual((strike["CooldownSetID"], strike["SpellID"], strike["Category"], strike["OrderIndex"]), ("12", "12294", "0", "0"))
        links = self.export("CooldownSetLinkedSpell", 6422327)
        self.assertIn({"ID": "101", "SpellID": "16979", "Field_11_1_5_59571_001": "0", "CooldownSetSpellID": "280"}, links)

    def test_authored_rotation_priority_and_rule_join(self):
        combat = self.export("AssistedCombat", 6698666)
        self.assertIn({"ID": "4", "ChrSpecializationID": "259"}, combat)
        steps = self.export("AssistedCombatStep", 6698668)
        self.assertIn({"ID": "9", "SpellID": "703", "AssistedCombatID": "4", "OrderIndex": "26"}, steps)
        rules = self.export("AssistedCombatRule", 6698667)
        self.assertIn({"ID": "32", "OrderIndex": "0", "Field_11_1_7_60520_002": "0", "ConditionType": "15", "ConditionValue1": "703", "ConditionValue2": "0", "ConditionValue3": "0", "AssistedCombatStepID": "9"}, rules)


if __name__ == "__main__":
    unittest.main()
