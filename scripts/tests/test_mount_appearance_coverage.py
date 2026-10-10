"""Extend authored coverage without losing existing required NPC appearances."""
import contextlib
import importlib.util
from pathlib import Path
import sqlite3
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("npc_importer", ROOT / "scripts/import_npc_appearance.py")
importer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(importer)


class MountAppearanceCoverageTests(unittest.TestCase):
    def test_append_ordinary_mount_preserves_profiles_choices_geosets_and_coverage(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory) / "base.sqlite"
            old = ([(19177, 1, 0, 1, 120192)], [(19177, 42)], [(19177, 7, 3)], [(19177, 1)])
            importer.write_database(base, old)
            merged = importer.merge_existing_rows(base, ([], [], [], [(27237, 0)]))
            output = Path(directory) / "mounts.sqlite"
            importer.write_database(output, merged)
            with contextlib.closing(sqlite3.connect(output)) as connection:
                self.assertEqual(connection.execute("SELECT * FROM appearances").fetchall(), old[0])
                self.assertEqual(connection.execute("SELECT * FROM choices").fetchall(), old[1])
                self.assertEqual(connection.execute("SELECT * FROM geosets").fetchall(), old[2])
                self.assertEqual(connection.execute("SELECT * FROM display_coverage ORDER BY display_id").fetchall(), [(19177, 1), (27237, 0)])
            with contextlib.closing(sqlite3.connect(base)) as connection:
                self.assertEqual(connection.execute("SELECT * FROM display_coverage").fetchall(), old[3])

    def test_conflicting_existing_coverage_is_rejected_not_downgraded(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory) / "base.sqlite"
            importer.write_database(base, ([(19177, 1, 0, 1, 120192)], [], [], [(19177, 1)]))
            with self.assertRaisesRegex(ValueError, "conflicting.*display_coverage"):
                importer.merge_existing_rows(base, ([], [], [], [(19177, 0)]))


if __name__ == "__main__":
    unittest.main()
