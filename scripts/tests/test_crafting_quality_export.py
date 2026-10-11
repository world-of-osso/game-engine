"""Authored crafting tiers and inventory overlays from pinned local Retail CASC."""
import csv
import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("export_db2_csv", ROOT / "scripts/export_db2_csv.py")
exporter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(exporter)


class CraftingQualityExportTests(unittest.TestCase):
    def export(self, table, fdid):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "table.csv"
            args = ["export", table, str(ROOT / f"data/dbfilesclient/{fdid}.db2"), str(output)]
            with patch.object(sys, "argv", args):
                exporter.main()
            with output.open() as handle:
                return {int(row["ID"]): row for row in csv.DictReader(handle)}

    def test_midnight_quality_ids_are_tiers_not_numeric_id_ranks(self):
        rows = self.export("CraftingQuality", 4373538)
        self.assertEqual(rows[13]["QualityTier"], "1")
        self.assertEqual(rows[14]["QualityTier"], "2")
        self.assertEqual(rows[14]["CraftingQualityAtlasSetID"], "7")
        self.assertEqual(rows[7]["QualityTier"], "4")

    def test_inventory_overlay_is_the_authored_atlas_element(self):
        rows = self.export("CraftingQualityAtlasSet", 7012110)
        self.assertEqual(rows[1]["IconInv"], "20209")
        self.assertNotEqual(rows[7]["IconInv"], rows[2]["IconInv"])

    def test_output_variants_preserve_the_crafting_data_relation(self):
        rows = self.export("CraftingDataItemQuality", 4545612)
        self.assertEqual(rows[1]["ItemID"], "190645")
        self.assertEqual(rows[2]["ItemID"], "190643")
        self.assertEqual(rows[1]["CraftingDataID"], "1")
        self.assertEqual(rows[2]["CraftingDataID"], "1")


if __name__ == "__main__":
    unittest.main()
