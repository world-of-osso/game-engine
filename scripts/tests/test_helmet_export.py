"""The independent CSV export preserves concrete local DB2 helmet rules."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("export_db2_csv", ROOT / "scripts/export_db2_csv.py")
exporter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(exporter)


class HelmetExportTests(unittest.TestCase):
    def test_cached_helmet_rules_include_hair_ears_and_facial_groups(self):
        layout, columns = exporter.TABLES["HelmetGeosetData"]
        records, dropped, _, _ = exporter.read_wdc5(
            (ROOT / "data/db2/HelmetGeosetData.db2").read_bytes(), layout
        )
        self.assertEqual(dropped, 0)
        self.assertEqual(len(records), 18302)
        expected = {
            1: [0, 7, 51], 2: [0, 3, 7, 35, 36, 51],
            3: [0, 7, 16, 35, 39, 51], 4: [0, 3, 34, 37, 39, 51],
            6: [0, 7, 35, 37, 39, 51], 10: [0, 35, 51],
        }
        for race, groups in expected.items():
            actual = sorted({values[1] for values, parent, _ in records.values()
                             if parent in (460, 461) and values[0] == race})
            self.assertEqual(actual, groups)
        self.assertEqual([name for name, _ in columns], [
            "ID", "RaceID", "HideGeosetGroup", "RaceBitSelection",
            "Field_10_0_0_46047_003", "HelmetGeosetVisDataID",
        ])


if __name__ == "__main__":
    unittest.main()
