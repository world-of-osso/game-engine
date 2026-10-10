"""Model resource joins retain inline FDIDs after the six-float bounding box."""
import csv
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


def model_table(layout=0x2AE4E788):
    fields = [(0, 192), (192, 32), (224, 32), (256, 32), (288, 32)]
    start = 204 + 40 + len(fields) * 4 + len(fields) * 24
    data = bytearray(start)
    data[:4] = b"WDC5"
    struct.pack_into("<6I", data, 136, 2, len(fields), 40, 0, 0, layout)
    struct.pack_into("<HH7I", data, 172, 0, 1, 0, len(fields), 120, 0, 0, 0, 1)
    struct.pack_into("<Q8I", data, 204, 0, start, 2, 0, start + 80, 0, 0, 0, 0)
    storage = 204 + 40 + len(fields) * 4
    for i, (offset, width) in enumerate(fields):
        struct.pack_into("<HH5I", data, storage + i * 24, offset, width, 0, 0, 0, 0, 0)
    for fdid, resource, flags, lod in [(7567115, 85100, 0, 0), (42, 18251, 2, 3)]:
        data.extend(struct.pack("<6f4I", -1, -2, -3, 1, 2, 3, fdid, flags, lod, resource))
    return data


class ModelFileDataExportTests(unittest.TestCase):
    def export(self, layout=0x2AE4E788):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "table.db2"
            output = Path(directory) / "table.csv"
            source.write_bytes(model_table(layout))
            result = subprocess.run(
                ["python3", str(ROOT / "scripts/export_db2_csv.py"), "ModelFileData", str(source), str(output)],
                capture_output=True, text=True,
            )
            rows = []
            if output.exists():
                with output.open() as stream:
                    rows = list(csv.DictReader(stream))
            return result, rows

    def test_inline_id_after_bbox_and_resource_are_exported(self):
        result, rows = self.export()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(rows, [
            {"FileDataID": "42", "Flags": "2", "LodCount": "3", "ModelResourcesID": "18251"},
            {"FileDataID": "7567115", "Flags": "0", "LodCount": "0", "ModelResourcesID": "85100"},
        ])

    def test_different_layout_is_rejected_before_csv_creation(self):
        result, rows = self.export(0x12345678)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("layout 12345678, expected 2AE4E788", result.stderr)
        self.assertEqual(rows, [])
