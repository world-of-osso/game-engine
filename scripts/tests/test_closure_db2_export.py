"""Local authenticated DB2 export produces effect join identities, not field guesses."""

from pathlib import Path
import csv
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).parents[2]
DATA = Path("/home/osso/Projects/world-of-osso/game-engine/data")


class EffectExportTests(unittest.TestCase):
    def test_reader_import_by_path_in_isolated_process(self):
        code = (
            "import importlib.util; from pathlib import Path; "
            f's=importlib.util.spec_from_file_location("exporter", {str(ROOT / "scripts/export_db2_csv.py")!r}); '
            "m=importlib.util.module_from_spec(s); s.loader.exec_module(m); "
            f'r,*_=m.read_wdc5(Path({str(DATA / "dbfilesclient/1525607.db2")!r}).read_bytes(),m.TABLES["BeamEffect"][0]); '
            "print(r[3][0][0])"
        )
        result = subprocess.run(
            ["python3", "-I", "-c", code], capture_output=True, text=True
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "743")

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
