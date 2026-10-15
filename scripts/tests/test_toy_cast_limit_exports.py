"""Export genuine local69933 required-area records; no reconstructed source rows."""
import csv
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "export_db2_csv.py"
SOURCE = SCRIPT.parent / "tests/fixtures/toy-cast-limits-12.1.0.69933"


class ToyCastLimitExports(unittest.TestCase):
    def export_table(self, table, fdid):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "out.csv"
            result = subprocess.run(
                [sys.executable, str(SCRIPT), table, str(SOURCE / f"{fdid}.db2"), str(output)],
                capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            with output.open() as handle:
                return list(csv.DictReader(handle)), result.stderr

    def test_local_cast_requirements_keep_region_and_facing_values(self):
        rows, diagnostic = self.export_table("SpellCastingRequirements", 1002166)
        self.assertEqual(len(rows), 28152)
        self.assertIn("3 encrypted records dropped", diagnostic)
        spells = {int(row["SpellID"]): row for row in rows}
        self.assertEqual(spells[171352], {
            "ID": "14922", "SpellID": "171352", "FacingCasterFlags": "0",
            "MinFactionID": "0", "MinReputation": "0", "RequiredAreasID": "4031",
            "RequiredAuraVision": "0", "RequiresSpellFocus": "0",
        })
        self.assertEqual(spells[61810]["RequiredAreasID"], "1103")
        self.assertEqual(spells[406870]["FacingCasterFlags"], "1")

    def test_local_area_group_relationships_preserve_all_draenor_members(self):
        rows, diagnostic = self.export_table("AreaGroupMember", 1093406)
        self.assertEqual(len(rows), 27273)
        self.assertIn("0 encrypted records dropped", diagnostic)
        members = {int(row["AreaID"]) for row in rows if row["AreaGroupID"] == "4031"}
        self.assertEqual(members, {6662, 6719, 6720, 6721, 6722, 6723, 6755, 7462, 6682, 7025, 7663})
        row = next(row for row in rows if row["ID"] == "4660")
        self.assertEqual(row, {"ID": "4660", "AreaID": "6662", "AreaGroupID": "4031"})

    def test_layout_drift_refuses_before_publishing_output(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "drift.db2"
            output = Path(directory) / "out.csv"
            data = bytearray((SOURCE / "1002166.db2").read_bytes())
            struct.pack_into("<I", data, 156, 0x02074291)
            source.write_bytes(data)
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "SpellCastingRequirements", str(source), str(output)],
                capture_output=True, text=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("layout 02074291, expected 02074290", result.stderr)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
