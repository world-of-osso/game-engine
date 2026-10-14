import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from prepare_ui_icons import prepare_ui_icons  # noqa: E402

BUILD = "12.1.5.70340"


def write_fixture(root: Path, exceptions: list[int]) -> None:
    db2 = root / "data/db2" / BUILD
    db2.mkdir(parents=True)
    (db2.parent / 'active').symlink_to(BUILD, target_is_directory=True)
    (db2 / 'receipt.json').write_text(json.dumps({'identity': {'product': 'wow', 'build': BUILD}}))
    (db2 / "SpellMisc.csv").write_text("ID,SpellIconFileDataID,ActiveIconFileDataID\n1,101,0\n2,102,103\n")
    (db2 / "TraitDefinition.csv").write_text("ID,OverrideIcon\n1,0\n")
    (db2 / "ChrClasses.csv").write_text("ID,IconFileDataID\n1,0\n")
    (db2 / "ChrSpecialization.csv").write_text("ID,SpellIconFileID\n1,0\n")
    textures = root / "data/textures"
    textures.mkdir(parents=True)
    (textures / "101.blp").write_bytes(b"BLP2" + b"\0" * 16)
    (root / "scripts").mkdir()
    (root / "scripts/ui-icon-exceptions.json").write_text(json.dumps({
        "version": 1,
        "spell_db2_build": BUILD,
        "exceptions": [{"fdid": fdid, "reason": "fixture: absent from local CASC"} for fdid in exceptions],
    }))
    # Extractor that finds nothing, like casc-local for root-absent FDIDs.
    extractor = root / "casc-local"
    extractor.write_text("#!/bin/sh\nexit 1\n")
    extractor.chmod(0o755)


class ReleaseIconExceptions(unittest.TestCase):
    def test_listed_exceptions_ship_absent_and_are_recorded(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_fixture(root, exceptions=[102, 103])
            manifest = prepare_ui_icons(root, root / "casc-local")
            self.assertEqual(manifest['spell_db2_build'], BUILD)
            self.assertEqual([icon["fdid"] for icon in manifest["icons"]], [101, 102, 103])
            provenance = json.loads((root / "data/cache/ui-icon-provenance.json").read_text())
            self.assertEqual(provenance["status"], "complete-with-exceptions")
            self.assertEqual(provenance["missing_fdids"], [102, 103])
            self.assertEqual([entry["fdid"] for entry in provenance["excepted"]], [102, 103])
            self.assertEqual([icon["fdid"] for icon in provenance["icons"]], [101])

    def test_unlisted_missing_icon_still_fails_release(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_fixture(root, exceptions=[102])
            with self.assertRaisesRegex(ValueError, r"1 required icons absent/invalid without an exception.*\[103\]"):
                prepare_ui_icons(root, root / "casc-local")
            missing = json.loads((root / "data/diagnostics/ui-icon-preparation/missing.json").read_text())
            self.assertEqual(missing["fdids"], [103])


if __name__ == "__main__":
    unittest.main()
