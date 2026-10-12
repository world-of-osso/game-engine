"""Mount-root cache extension: real file IO, cache reopen and conflict refusal."""
import contextlib
import hashlib
import io
import sqlite3
import tempfile
import unittest
from pathlib import Path

from import_mount_appearance import main
from import_npc_appearance import write_database
from test_import_npc_appearance import fixture


class MountAppearanceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        db2 = self.root / "db2"
        db2.mkdir()
        (db2 / "1264997.db2").write_bytes(fixture())
        (db2 / "3692043.db2").write_bytes(fixture(layout=0x2F331C33, related=True))
        (db2 / "1720141.db2").write_bytes(fixture(layout=0x5E539080, related=True))
        data = self.root / "data"
        data.mkdir()
        # Mount18 Chestnut Mare and Mount6 Brown Horse are ordinary creature displays.
        (data / "CreatureDisplayInfo.csv").write_text("ID,ExtendedDisplayInfoID\n2404,0\n2405,0\n")
        (data / "TextureFileData.csv").write_text("MaterialResourcesID,FileDataID,UsageType\n")
        (data / "community-listfile.csv").write_text("")
        with sqlite3.connect(self.root / "models.sqlite") as conn:
            conn.execute("CREATE TABLE creature_displays(display_id INTEGER,model_fdid INTEGER)")
        write_database(self.root / "base.sqlite", ([], [], [], [(2404, 0)]))
        (self.root / "MountXDisplay.csv").write_text("ID,MountID,CreatureDisplayInfoID\n16,18,2405\n17,18,2405\n")

    def run_import(self):
        argv = ["--mount-catalog", str(self.root / "MountXDisplay.csv"),
                "--db2-dir", str(self.root / "db2"), "--data-dir", str(self.root / "data"),
                "--model-cache", str(self.root / "models.sqlite"),
                "--base-cache", str(self.root / "base.sqlite"), "--output", str(self.root / "output.sqlite")]
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            return main(argv)

    def test_extends_mount_coverage_preserving_read_only_base_and_refuses_replacement(self):
        before = hashlib.sha256((self.root / "base.sqlite").read_bytes()).digest()
        self.assertEqual(self.run_import(), 0)
        self.assertEqual(hashlib.sha256((self.root / "base.sqlite").read_bytes()).digest(), before)
        with sqlite3.connect(self.root / "output.sqlite") as conn:
            self.assertEqual(conn.execute("SELECT * FROM display_coverage ORDER BY display_id").fetchall(), [(2404, 0), (2405, 0)])
        self.assertEqual(self.run_import(), 1)

    def test_refuses_downgrading_required_appearance_to_ordinary_display(self):
        with sqlite3.connect(self.root / "base.sqlite") as conn:
            conn.execute("INSERT INTO display_coverage VALUES (2405,1)")
        self.assertEqual(self.run_import(), 1)
        self.assertFalse((self.root / "output.sqlite").exists())

    def test_empty_mount_catalog_cannot_produce_cache(self):
        (self.root / "MountXDisplay.csv").write_text("ID,MountID,CreatureDisplayInfoID\n")
        self.assertEqual(self.run_import(), 1)
        self.assertFalse((self.root / "output.sqlite").exists())


if __name__ == "__main__":
    unittest.main()
