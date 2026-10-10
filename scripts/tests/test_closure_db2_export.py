"""Local authenticated DB2 export produces effect join identities, not field guesses."""

from pathlib import Path
import csv
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).parents[2]
DATA = Path("/home/osso/Projects/world-of-osso/game-engine/data")


class EffectExportTests(unittest.TestCase):
    def test_empty_clone_table_exports_no_rows(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "CloneEffect.csv"
            result = subprocess.run(
                [
                    "python3",
                    str(ROOT / "scripts/export_db2_csv.py"),
                    "CloneEffect",
                    str(DATA / "dbfilesclient/2175218.db2"),
                    str(path),
                ],
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            with path.open() as stream:
                self.assertEqual(list(csv.DictReader(stream)), [])

    def test_beam_rows_keep_chain_identity(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "BeamEffect.csv"
            result = subprocess.run(
                [
                    "python3",
                    str(ROOT / "scripts/export_db2_csv.py"),
                    "BeamEffect",
                    str(DATA / "dbfilesclient/1525607.db2"),
                    str(path),
                ],
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            with path.open() as stream:
                rows = {int(r["ID"]): r for r in csv.DictReader(stream)}
            self.assertEqual(int(rows[3]["BeamID"]), 743)
            self.assertEqual(int(rows[4]["BeamID"]), 6301)
            self.assertIn("encrypted records dropped", result.stderr)


if __name__ == "__main__":
    unittest.main()
