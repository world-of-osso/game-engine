"""Companion appearance roots extend coverage without altering existing NPCs."""
import importlib.util
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("npc_import", ROOT / "scripts/import_npc_appearance.py")
importer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(importer)


class MergeNpcAppearanceTests(unittest.TestCase):
    def test_missing_companion_coverage_is_added_and_existing_profiles_are_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            base, incoming, output = (Path(directory) / name for name in ("base.sqlite", "incoming.sqlite", "output.sqlite"))
            importer.write_database(base, ([(900, 1, 0, 1, 777)], [(900, 100)], [(900, 1, 2)], [(900, 1)]))
            importer.write_database(incoming, ([(900, 2, 1, 2, 888), (901, 3, 0, 1, 999)], [(901, 200)], [(7937, 1, 3)], [(900, 1), (901, 1), (7937, 0)]))
            result = subprocess.run([sys.executable, str(ROOT / "scripts/merge_npc_appearance.py"), str(base), str(incoming), str(output)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            with sqlite3.connect(output) as conn:
                self.assertEqual(conn.execute("SELECT * FROM appearances WHERE display_id=900").fetchall(), [(900, 1, 0, 1, 777)])
                self.assertEqual(conn.execute("SELECT * FROM choices WHERE display_id=900").fetchall(), [(900, 100)])
                self.assertEqual(conn.execute("SELECT * FROM display_coverage ORDER BY display_id").fetchall(), [(900, 1), (901, 1), (7937, 0)])
                self.assertEqual(conn.execute("SELECT * FROM geosets WHERE display_id=7937").fetchall(), [(7937, 1, 3)])
            refused = subprocess.run([sys.executable, str(ROOT / "scripts/merge_npc_appearance.py"), str(base), str(incoming), str(output)], capture_output=True, text=True)
            self.assertNotEqual(refused.returncode, 0)
            with sqlite3.connect(output) as conn:
                self.assertEqual(conn.execute("SELECT count(*) FROM display_coverage").fetchone()[0], 3)

    def test_local_companion_catalog_drives_strict_importer_display_roots(self):
        source = ROOT / "data/diagnostics/pets-2026-10-10"
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "pets.sqlite"
            result = subprocess.run([sys.executable, str(ROOT / "scripts/import_npc_appearance.py"),
                "--db2-dir", str(source / "appearance-db2"), "--data-dir", str(source / "appearance-csv"),
                "--model-cache", str(ROOT / "data/cache/creature_display.sqlite"),
                "--outfit-cache", str(ROOT / "data/cache/outfit_links.sqlite"),
                "--pet-catalog", str(ROOT / "data/battle-pets.json"), "--output", str(output)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            with sqlite3.connect(output) as connection:
                self.assertEqual(connection.execute("SELECT count(*) FROM display_coverage").fetchone()[0], 2607)
                self.assertEqual(connection.execute("SELECT requires_appearance FROM display_coverage WHERE display_id=7937").fetchone()[0], 0)
                self.assertEqual(connection.execute("SELECT count(*) FROM appearances").fetchone()[0], 11)

    def test_required_profile_missing_does_not_publish_an_ordinary_substitute(self):
        with tempfile.TemporaryDirectory() as directory:
            base, incoming, output = (Path(directory) / name for name in ("base.sqlite", "incoming.sqlite", "output.sqlite"))
            importer.write_database(base, ([], [], [], [(500, 0)]))
            importer.write_database(incoming, ([], [], [], [(901, 1)]))
            result = subprocess.run([sys.executable, str(ROOT / "scripts/merge_npc_appearance.py"), str(base), str(incoming), str(output)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(output.exists())
            self.assertIn("required appearance missing for display 901", result.stderr)


if __name__ == "__main__":
    unittest.main()
