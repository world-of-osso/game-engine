import hashlib
import json
import sqlite3
import struct
import tempfile
import unittest
from contextlib import closing
from pathlib import Path
from unittest.mock import patch

from scripts import import_forever_skyborne as importer


class RefreshTests(unittest.TestCase):
    def test_stale_model_and_skin_are_reextracted_with_recursive_alias(self):
        def chunk(tag, payload):
            return tag + struct.pack("<I", len(payload)) + payload

        current = {
            "7478487.m2": chunk(b"MD21", b""),
            "7478494.m2": chunk(b"MD21", b"") + chunk(b"SFID", struct.pack("<I", 9002)),
            "9002.skin": b"SKINcurrent",
            "8200220.blp": b"BLP2current",
            "8199012.blp": b"BLP2current",
        }
        tables = {
            name: []
            for name in (
                "ChrCustomizationOption",
                "ChrCustomizationChoice",
                "ChrCustomizationElement",
                "ChrCustomizationMaterial",
                "TextureFileData",
                "ChrCustomizationSkinnedModel",
                "Map",
                "Light",
                "LightParams",
                "LightSkybox",
                "ChrRaces",
            )
        }
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            staging = root / "staging"
            staging.mkdir()
            (root / "db2/12.1.0.69933").mkdir(parents=True)
            (root / "db2" / importer.BUILD).mkdir()
            (root / "db2/12.1.0.69933/Map.csv").write_text("ID\n0\n")
            for name, raw in current.items():
                folder = root / ("textures" if name.endswith(".blp") else "models")
                folder.mkdir(exist_ok=True)
                (folder / name).write_bytes(raw)
                (staging / name).write_bytes(raw)
            (root / "models/7478494.m2").write_bytes(chunk(b"MD21", b"old"))
            (staging / "7478494.m2").write_bytes(chunk(b"MD21", b"old"))
            (root / "models/9002.skin").write_bytes(b"SKINold")
            (staging / "9002.skin").write_bytes(b"SKINold")
            (root / "models/747849400.skin").write_bytes(b"SKINold")
            with closing(sqlite3.connect(root / "resolution.sqlite")) as db, db:
                db.execute("create table resolution(fdid integer,content_key blob)")
                db.executemany(
                    "insert into resolution values (?, ?)",
                    [
                        (int(Path(name).stem), hashlib.md5(raw).digest())
                        for name, raw in current.items()
                    ],
                )
            executable = root / "extract"
            executable.write_text(
                "#!/usr/bin/env python3\nimport sys\nfrom pathlib import Path\n"
                + f"assets={current!r}\n"
                + "out=Path(sys.argv[sys.argv.index('-o')+1])\n"
                + "for fdid in sys.argv[1:sys.argv.index('-o')]:\n"
                + " for name,raw in assets.items():\n"
                + "  if name.split('.')[0]==fdid: (out/name).write_bytes(raw)\n"
            )
            executable.chmod(0o755)
            with (
                patch.object(importer, "CACHE", root),
                patch.object(importer, "PROBE_DIRECTORY", root / "probes"),
                patch.object(importer, "CASC_LOCAL", executable),
            ):
                failures = importer.import_assets(root, staging, tables)
            self.assertEqual(failures, [])
            self.assertEqual(
                (root / "models/7478494.m2").read_bytes(), current["7478494.m2"]
            )
            self.assertEqual(
                (root / "models/747849400.skin").read_bytes(), current["9002.skin"]
            )
            manifest = json.loads(
                (root / "db2" / importer.BUILD / "assets.json").read_text()
            )
            self.assertEqual(manifest["counts"]["extracted"], 2)


if __name__ == "__main__":
    unittest.main()
