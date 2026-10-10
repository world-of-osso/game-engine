"""Concrete Retail pet records from local CASC, not remotely sourced CSVs."""
import csv
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "data/diagnostics/pets-2026-10-10"


class BattlePetExportTests(unittest.TestCase):
    def export(self, table, fdid, source=SOURCE):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "export.csv"
            result = subprocess.run(
                [sys.executable, str(ROOT / "scripts/export_db2_csv.py"),
                 table, str(source / f"{fdid}.db2"), str(output)],
                capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            with output.open() as handle:
                return {int(row["ID"]): row for row in csv.DictReader(handle)}, result.stderr

    def test_species_preserves_inline_id_and_summon_spell(self):
        rows, report = self.export("BattlePetSpecies", 841622)
        self.assertEqual(rows[39]["CreatureID"], "2671")
        self.assertEqual(rows[39]["SummonSpellID"], "4055")
        self.assertEqual(rows[39]["PetTypeEnum"], "9")
        self.assertIn("11 encrypted records dropped", report)
        self.assertEqual(len(rows), 2994)

    def test_effect_property_labels_preserve_all_six_string_offsets(self):
        rows, _ = self.export("BattlePetEffectProperties", 801580)
        self.assertEqual(rows[24]["ParamLabel_0"], "Points")
        self.assertEqual(rows[24]["ParamLabel_1"], "Accuracy")
        self.assertEqual(rows[24]["ParamLabel_2"], "IsPeriodic")
        self.assertEqual(rows[24]["ParamLabel_3"], "OverrideIndex")
        self.assertEqual(rows[24]["ParamLabel_5"], "")
        self.assertEqual(len(rows), 135)

    def test_decrypted_sections_export_strings_and_only_drop_unknown_keys(self):
        rows, report = self.export("BattlePetSpecies", 841622, SOURCE / "known-keys")
        self.assertEqual(len(rows), 3001)
        self.assertIn("4 encrypted records dropped", report)
        self.assertEqual(rows[4893]["Description_lang"],
                         "The emerald sheen of this murloc is likely formed from constant grooming and a robust exercise routine.")
        creatures, report = self.export("Creature", 841631, SOURCE / "known-keys")
        self.assertEqual(creatures[239798]["Name_lang"], "Vereesa Windrunner")
        self.assertEqual(len(creatures), 23074)
        self.assertIn("5 encrypted records dropped", report)

    def test_runtime_catalog_joins_species_to_local_creature_display(self):
        import json
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "pets.json"
            result = subprocess.run(
                [sys.executable, str(ROOT / "scripts/export_pet_catalog.py"),
                 str(ROOT / "data/db2/12.1.0.69933"), str(output)],
                capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            catalog = json.loads(output.read_text())
            pet = next(row for row in catalog if row["species_id"] == 39)
            self.assertEqual(pet["name"], "Mechanical Squirrel")
            self.assertEqual(pet["display_id"], 7937)
            self.assertEqual(pet["summon_spell_id"], 4055)
            self.assertEqual(len(catalog), 3001)

    def test_creature_names_and_display_arrays_for_companion_models(self):
        rows, report = self.export("Creature", 841631)
        self.assertEqual(rows[2671]["Name_lang"], "Mechanical Squirrel")
        self.assertEqual(rows[2671]["DisplayID_0"], "7937")
        self.assertEqual(rows[2671]["DisplayID_3"], "0")
        self.assertIn("48 encrypted records dropped", report)

    def test_breed_quality_exports_float_not_raw_bits(self):
        rows, _ = self.export("BattlePetBreedQuality", 801578)
        self.assertEqual(float(rows[7]["StateMultiplier"]), 0.5)
        self.assertAlmostEqual(float(rows[10]["StateMultiplier"]), 0.65)


if __name__ == "__main__":
    unittest.main()
