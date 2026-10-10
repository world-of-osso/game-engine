"""Passenger-seat records exported from the pinned local Retail CASC build."""
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


class VehicleExportTests(unittest.TestCase):
    def export(self, table, fdid):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "table.csv"
            args = ["export", table, str(ROOT / f"data/dbfilesclient/{fdid}.db2"), str(output)]
            with patch.object(sys, "argv", args):
                exporter.main()
            with output.open() as handle:
                return {int(row["ID"]): row for row in csv.DictReader(handle)}

    def test_mammoth_and_chopper_preserve_sparse_passenger_seat_indices(self):
        rows = self.export("Vehicle", 1368621)
        self.assertEqual([int(rows[312][f"SeatID_{i}"]) for i in range(8)],
                         [0, 2764, 2765, 0, 0, 0, 0, 0])
        self.assertEqual([int(rows[318][f"SeatID_{i}"]) for i in range(8)],
                         [0, 2804, 0, 0, 0, 0, 0, 0])

    def test_passenger_attachment_animation_and_flags_are_not_driver_defaults(self):
        rows = self.export("VehicleSeat", 1345447)
        self.assertEqual(int(rows[2764]["AttachmentID"]), 14)
        self.assertEqual(int(rows[2765]["AttachmentID"]), 13)
        self.assertEqual(int(rows[2764]["RideAnimLoop"]), 91)
        self.assertEqual(int(rows[2804]["RideAnimLoop"]), 500)
        self.assertTrue(int(rows[2764]["Flags"]) & 0x02000000)
        self.assertEqual([float(rows[2764][f"AttachmentOffset_{i}"]) for i in range(3)],
                         [0.0, 0.0, 0.0])


if __name__ == "__main__":
    unittest.main()
