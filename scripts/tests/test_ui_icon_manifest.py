"""Concrete local DB2/BLP contract; no engine, server, CDN or synthetic art."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("prepare_ui_icons", ROOT / "scripts/prepare_ui_icons.py")
icons = importlib.util.module_from_spec(spec)
spec.loader.exec_module(icons)


class RequiredUiIconManifestTests(unittest.TestCase):
    def test_manifest_covers_real_class_spell_and_records_its_db2_source(self):
        manifest = icons.collect_required_icons(ROOT / "data")
        by_id = {entry["fdid"]: entry for entry in manifest["icons"]}
        self.assertIn(135875, by_id)  # Avenging Wrath31884/SpellMisc16623.
        self.assertTrue(any("SpellMisc" in source for source in by_id[135875]["sources"]))
        self.assertTrue(any("TraitDefinition" in path for path in manifest["source_tables"]))

    def test_manifest_fails_loudly_with_missing_requested_fdid(self):
        manifest = {"icons": [{"fdid": 135875, "sources": ["SpellMisc:31884"]}]}
        with tempfile.TemporaryDirectory(prefix="ui-icon-manifest-") as directory:
            missing = icons.missing_required_icons(manifest, Path(directory))
        self.assertEqual(missing, [135875])

    def test_failed_preparation_still_records_available_and_missing_provenance(self):
        with tempfile.TemporaryDirectory(prefix="ui-icon-provenance-") as directory:
            root = Path(directory)
            data = root / "data"
            data.mkdir()
            (data / "db2").symlink_to(ROOT / "data/db2", target_is_directory=True)
            with self.assertRaises(ValueError):
                icons.prepare_ui_icons(root, root / "no-extractor")
            path = data / "cache/ui-icon-provenance.json"
            self.assertTrue(path.is_file(), "Blocked preparation must preserve provenance")
            provenance = json.loads(path.read_text())
            self.assertEqual(provenance["status"], "incomplete")
            self.assertIn(135875, provenance["missing_fdids"])
            self.assertEqual(provenance["icons"], [])

    def test_generated_manifest_requires_every_icon_in_the_extracted_set(self):
        path = ROOT / "data/cache/required-ui-icons.json"
        manifest = json.loads(path.read_text())
        self.assertGreater(len(manifest["icons"]), 0)
        missing = icons.missing_required_icons(manifest, ROOT / "data")
        self.assertEqual(len(missing), 0,
            f"{len(missing)} required icon FDIDs absent/invalid, first cases {missing[:32]}; manifest {path}")

    def test_real_extracted_file_satisfies_the_manifest_without_other_sources(self):
        manifest = {"icons": [{"fdid": 135875, "sources": ["SpellMisc:31884"]}]}
        with tempfile.TemporaryDirectory(prefix="ui-icon-manifest-") as directory:
            data = Path(directory)
            (data / "textures").mkdir()
            shutil.copy2(ROOT / "data/textures/135875.blp", data / "textures/135875.blp")
            self.assertEqual(icons.missing_required_icons(manifest, data), [])


if __name__ == "__main__":
    unittest.main()
