"""Frozen model closures must retain authenticated source and path ownership."""

from contextlib import closing
import hashlib
import json
from pathlib import Path
import sqlite3
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "performance"))
import stage_frozen_model_closure as staging


def chunk(tag, payload):
    return tag + struct.pack("<I", len(payload)) + payload


def receipt(fdid, kind, build_key, payload):
    directory = "textures" if kind == "blp" else "models"
    return {
        "product": "wow",
        "build": "12.1.0.69933",
        "build_key": build_key,
        "fdid": fdid,
        "kind": kind,
        "path": f"products/wow/{build_key}/{directory}/{fdid}.{kind}",
        "bytes": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
        "content_key": hashlib.md5(payload).hexdigest(),
    }


class FrozenModelClosureTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.data = self.root / "data"
        (self.data / "cache").mkdir(parents=True)
        (self.data / "models").mkdir()
        (self.data / "textures").mkdir()
        self.old = "a" * 32
        self.current = "b" * 32
        header = bytearray(0x58)
        header[:4] = b"MD20"
        self.model = (
            chunk(b"MD21", header)
            + chunk(b"SFID", struct.pack("<I", 202))
            + chunk(b"TXID", struct.pack("<I", 303))
        )
        self.skin = b"SKIN authenticated fixture"
        self.texture = b"BLP2 authenticated fixture"
        model_receipt = receipt(100, "m2", self.old, self.model)
        texture_receipt = receipt(303, "blp", self.current, self.texture)
        for row, raw in [(model_receipt, self.model), (texture_receipt, self.texture)]:
            path = self.data / row["path"]
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw)
        (self.data / "models/202.skin").write_bytes(self.skin)
        (self.data / "textures/303.blp").write_bytes(self.texture)
        index = {
            "version": 1,
            "assets": [model_receipt, texture_receipt],
            "aliases": [],
            "chains": [],
        }
        (self.data / "cache/model-asset-index-fixture.json").write_text(json.dumps(index))
        local = self.data / "local-listfile-cache.sqlite"
        with closing(sqlite3.connect(local)) as db:
            db.execute(
                "CREATE TABLE local_listfile_entries (fdid INTEGER, path TEXT, lower_path TEXT)"
            )
            db.commit()
        self.resolution = self.root / "resolution.sqlite"
        with closing(sqlite3.connect(self.resolution)) as db:
            db.execute("CREATE TABLE resolution (fdid INTEGER PRIMARY KEY, content_key BLOB)")
            db.executemany(
                "INSERT INTO resolution VALUES (?, ?)",
                [
                    (100, hashlib.md5(self.model).digest()),
                    (202, hashlib.md5(self.skin).digest()),
                    (303, hashlib.md5(self.texture).digest()),
                ],
            )
            db.commit()
        self.source_root = self.root / "root.bin"
        self.source_root.write_bytes(b"frozen authenticated root")
        identity = {
            "product": "wow",
            "build": "12.1.0.69933",
            "build_key": self.old,
            "root_sha256": staging.sha256(self.source_root),
            "resolution_sha256": staging.sha256(self.resolution),
        }
        self.manifest = self.root / "frozen.json"
        self.manifest.write_text(json.dumps({"source_identity": identity}))
        self.output = self.root / "staged"

    def stage(self):
        return staging.stage_closure(
            self.data,
            self.data,
            [100],
            self.manifest,
            self.source_root,
            self.resolution,
            self.output,
            "fixture-20261030",
        )

    def test_stages_new_skin_and_historical_texture_alias_without_changing_inputs(self):
        index = self.data / "cache/model-asset-index-fixture.json"
        before = index.read_bytes()
        fragment = json.loads(self.stage().read_text())
        self.assertEqual(
            [(r["fdid"], r["kind"]) for r in fragment["assets"]], [(202, "skin")]
        )
        self.assertEqual(
            [(r["fdid"], r["kind"]) for r in fragment["aliases"]], [(303, "blp")]
        )
        for row, raw in [
            (fragment["assets"][0], self.skin),
            (fragment["aliases"][0], self.texture),
        ]:
            self.assertEqual((self.output / row["path"]).read_bytes(), raw)
        self.assertEqual(index.read_bytes(), before)

    def test_rejects_wrong_frozen_child_bytes_before_creating_fragment(self):
        (self.data / "models/202.skin").write_bytes(b"SKIN incorrect bytes")
        with self.assertRaisesRegex(ValueError, "202.skin.*authenticated source bytes unavailable"):
            self.stage()
        self.assertFalse(self.output.exists())

    def test_rejects_snapshot_mismatch(self):
        self.source_root.write_bytes(b"different root")
        with self.assertRaisesRegex(ValueError, "snapshot hash mismatch"):
            self.stage()
        self.assertFalse(self.output.exists())

    def test_rejects_owned_parent_bytes_that_do_not_match_receipt(self):
        parent = self.data / f"products/wow/{self.old}/models/100.m2"
        parent.write_bytes(b"different model")
        with self.assertRaisesRegex(ValueError, "M2 100 does not match"):
            self.stage()
        self.assertFalse(self.output.exists())

    def test_rejects_missing_payload_for_existing_receipt(self):
        index = self.data / "cache/model-asset-index-fixture.json"
        document = json.loads(index.read_text())
        document["assets"].append(receipt(202, "skin", self.old, self.skin))
        index.write_text(json.dumps(document))
        with self.assertRaisesRegex(ValueError, "Existing owner payload missing"):
            self.stage()
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
