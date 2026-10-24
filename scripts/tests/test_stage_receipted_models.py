"""Private staging authenticates exact generic receipts before native-owned files."""

import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from test_asset_closure import chunk, ints, model
from closure_receipts import ReceiptClosure

SOURCE_KEY = "dcfc90fffd79ba00406ae46f5f657592"
PARENT_KEY = "88d5e8ef05ec5a8446cdec93d142d7c6"


class ReceiptedModelStageTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.data = self.root / "data"
        self.data.mkdir()
        self.output = self.root / "package"
        self.rows = []
        self.requests = []

    def put(self, relative, raw):
        path = self.data / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
        return path

    def source(self, fdid, kind, raw, requested_path=None):
        directory = "textures" if kind == "blp" else "models"
        relative = f"{directory}/{fdid}.{kind}"
        self.put(relative, raw)
        self.rows.append(
            {
                "product": "wow",
                "fdid": fdid,
                "type": kind,
                "path": relative,
                "actual_build": "12.1.0.69933",
                "build_config": SOURCE_KEY,
                "disposition": "verified_existing",
                "size": len(raw),
                "sha256": hashlib.sha256(raw).hexdigest(),
                "content_key": hashlib.md5(raw).hexdigest(),
            }
        )
        request = {
            "product": "wow",
            "fdid": fdid,
            "kind": kind,
            "source_path": relative,
            "source_build": "12.1.0.69933",
            "source_build_key": SOURCE_KEY,
            "requested_path": requested_path,
        }
        self.requests.append(request)
        return request

    def stage(self, ledger_sha256=None):
        from stage_receipted_models import stage

        raw = (
            "\n".join(json.dumps(row) for row in self.rows)
            + ("\n" if self.rows else "")
        ).encode()
        self.put("provenance/closure-extract.jsonl", raw)
        document = {
            "metadata_build": "12.1.5.70448",
            "fragment_name": "fixture-recovery",
            "ledger_sha256": ledger_sha256 or hashlib.sha256(raw).hexdigest(),
            "assets": self.requests,
        }
        return stage(self.data, self.output, document)

    def fragment(self):
        return json.loads(
            (self.output / "cache/model-asset-index-fixture-recovery.json").read_text()
        )

    def test_owned_model_becomes_verified_without_changing_source_data(self):
        raw = model(chunk("SFID", ints(132008)))
        self.source(132007, "m2", raw)
        result = self.stage()
        graph = ReceiptClosure(self.output, {}, "wow", "12.1.5.70448")
        graph.add(132007, "m2", "concrete owner recovery")
        manifest = graph.run()
        asset = next(row for row in manifest["assets"] if row["fdid"] == 132007)
        self.assertEqual(asset["identity_status"], "verified_receipt")
        self.assertEqual(asset["actual_build"], "12.1.0.69933")
        self.assertEqual(asset["asset_build_key"], SOURCE_KEY)
        self.assertEqual((self.data / "models/132007.m2").read_bytes(), raw)
        self.assertFalse((self.data / "cache").exists())
        self.assertEqual(result["owners"], 1)
        findings = [
            json.loads(line)
            for line in (self.output / "_proof/historical-source-findings.jsonl")
            .read_text()
            .splitlines()
        ]
        self.assertEqual(findings[0]["code"], "HISTORICAL_ASSET_SOURCE_BUILD_MISMATCH")
        self.assertEqual(findings[0]["actual_build_key"], SOURCE_KEY)
        self.assertEqual(findings[0]["metadata_build"], "12.1.5.70448")
        self.assertEqual(findings[0]["sha256"], hashlib.sha256(raw).hexdigest())

    def test_companion_alias_preserves_source_key_and_requested_namespace(self):
        raw = b"BLP2actualtexture"
        destination = f"products/wow/{PARENT_KEY}/textures/122979.blp"
        self.source(122979, "blp", raw, destination)
        result = self.stage()
        fragment = self.fragment()
        owner, alias = fragment["assets"][0], fragment["aliases"][0]
        self.assertEqual(owner["build_key"], SOURCE_KEY)
        self.assertEqual(owner["build"], "12.1.0.69933")
        self.assertEqual(alias["path"], destination)
        self.assertEqual(alias["source_build_key"], SOURCE_KEY)
        self.assertEqual(alias["source_build"], "12.1.0.69933")
        self.assertEqual((self.output / destination).read_bytes(), raw)
        self.assertEqual(result["aliases"], 1)

    def test_same_owner_requested_as_root_and_companion_is_not_duplicated(self):
        raw = b"BLP2sameowner"
        self.source(122979, "blp", raw)
        self.source(
            122979, "blp", raw, f"products/wow/{PARENT_KEY}/textures/122979.blp"
        )
        result = self.stage()
        self.assertEqual(result["owners"], 1)
        self.assertEqual(result["aliases"], 1)
        self.assertEqual(result["findings"], 2)

    def test_wrong_product_receipt_cannot_stage_matching_bytes(self):
        self.source(132007, "m2", model())
        self.rows[0]["product"] = "wow_classic_beta"
        with self.assertRaisesRegex(ValueError, "source receipt"):
            self.stage()
        self.assertFalse((self.output / "cache").exists())

    def test_payload_or_content_key_mismatch_cannot_publish_a_fragment(self):
        for field, value in [
            ("size", 999),
            ("sha256", "c" * 64),
            ("content_key", "c" * 32),
        ]:
            with self.subTest(field=field):
                self.output = self.root / f"package-{field}"
                self.rows = []
                self.requests = []
                self.source(132007, "m2", model())
                self.rows[0][field] = value
                with self.assertRaisesRegex(ValueError, "payload"):
                    self.stage()
                self.assertFalse((self.output / "cache").exists())

    def test_alias_cannot_cross_product_namespaces(self):
        self.source(
            122979,
            "blp",
            b"BLP2texture",
            f"products/wow_classic_beta/{PARENT_KEY}/textures/122979.blp",
        )
        with self.assertRaisesRegex(ValueError, "namespace"):
            self.stage()

    def test_existing_unique_owner_is_not_refreshed_or_replaced(self):
        raw = model()
        self.source(132007, "m2", raw)
        owner = {
            "product": "wow",
            "fdid": 132007,
            "kind": "m2",
            "build": "12.1.5.70448",
            "build_key": PARENT_KEY,
            "path": f"products/wow/{PARENT_KEY}/models/132007.m2",
            "bytes": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest(),
            "content_key": hashlib.md5(raw).hexdigest(),
        }
        original = json.dumps({"version": 1, "assets": [owner]}).encode()
        self.put("cache/model-asset-index.json", original)
        with self.assertRaisesRegex(ValueError, "owner"):
            self.stage()
        self.assertEqual(
            (self.data / "cache/model-asset-index.json").read_bytes(), original
        )
        self.assertFalse((self.output / "cache").exists())

    def test_changed_extraction_ledger_is_refused(self):
        self.source(132007, "m2", model())
        with self.assertRaisesRegex(ValueError, "ledger"):
            self.stage("c" * 64)

    def test_flat_residual_without_receipt_remains_unpublished(self):
        self.put("models/122893.m2", model())
        self.requests.append(
            {
                "product": "wow",
                "fdid": 122893,
                "kind": "m2",
                "source_path": "models/122893.m2",
                "source_build": "12.1.0.69933",
                "source_build_key": SOURCE_KEY,
                "requested_path": None,
            }
        )
        with self.assertRaisesRegex(ValueError, "source receipt"):
            self.stage()
        graph = ReceiptClosure(self.data, {}, "wow", "12.1.5.70448")
        graph.add(122893, "m2", "unreceipted residual")
        self.assertFalse(graph.run()["assets"][0]["present"])

    @unittest.skipUnless(os.environ.get("P4_RED") == "1", "SKIN ANIM recovery RED")
    def test_skin_owned_destination_retains_source_without_alias(self):
        raw = b"SKIN" + b"\0" * 40
        destination = f"products/wow/{SOURCE_KEY}/models/473319.skin"
        self.source(473319, "skin", raw, destination)
        try:
            result = self.stage()
        except ValueError as error:
            self.fail(f"Authenticated SKIN owner was refused: {error}")
        self.assertEqual(
            (result["owners"], result["aliases"], result["findings"]), (1, 0, 1)
        )
        owner = self.fragment()["assets"][0]
        self.assertEqual(owner["path"], destination)
        self.assertEqual(owner["build"], "12.1.0.69933")
        self.assertEqual((self.output / destination).read_bytes(), raw)
        self.assertEqual((self.data / "models/473319.skin").read_bytes(), raw)
        finding = json.loads(
            (self.output / "_proof/historical-source-findings.jsonl").read_text()
        )
        self.assertEqual(finding["alias_destination"], destination)
        self.assertEqual(finding["actual_build_key"], SOURCE_KEY)
        graph = ReceiptClosure(self.output, {}, "wow", "12.1.5.70448")
        graph.add(473319, "skin", "historical SKIN owner")
        self.assertEqual(
            graph.run()["assets"][0]["identity_status"], "verified_receipt"
        )

    @unittest.skipUnless(os.environ.get("P4_RED") == "1", "SKIN ANIM recovery RED")
    def test_chunked_and_raw_animation_owners_keep_source_namespace(self):
        for index, raw in enumerate([chunk("AFM2", ints(1, 2)), ints(10, 20, 30)]):
            with self.subTest(chunked=index == 0):
                self.rows, self.requests = [], []
                self.output = self.root / f"package-anim-{index}"
                fdid = 7001 + index
                destination = f"products/wow/{SOURCE_KEY}/models/{fdid}.anim"
                self.source(fdid, "anim", raw, destination)
                try:
                    result = self.stage()
                except ValueError as error:
                    self.fail(f"Authenticated AFID owner was refused: {error}")
                self.assertEqual((result["owners"], result["aliases"]), (1, 0))
                owner = self.fragment()["assets"][0]
                self.assertEqual(owner["path"], destination)
                self.assertEqual(owner["content_key"], hashlib.md5(raw).hexdigest())
                self.assertEqual((self.output / destination).read_bytes(), raw)

    @unittest.skipUnless(os.environ.get("P4_RED") == "1", "SKIN ANIM recovery RED")
    def test_skin_requires_native_minimum_header_before_fragment(self):
        self.source(473319, "skin", b"SKIN" + b"\0" * 8)
        with self.assertRaisesRegex(ValueError, "SKIN.*44"):
            self.stage()
        self.assertFalse((self.output / "cache").exists())

    @unittest.skipUnless(os.environ.get("P4_RED") == "1", "SKIN ANIM recovery RED")
    def test_skin_animation_cross_namespace_aliases_remain_refused(self):
        for kind, raw in [("skin", b"SKIN" + b"\0" * 40), ("anim", ints(1, 2))]:
            with self.subTest(kind=kind):
                self.rows, self.requests = [], []
                self.output = self.root / f"package-cross-{kind}"
                destination = f"products/wow/{PARENT_KEY}/models/473319.{kind}"
                self.source(473319, kind, raw, destination)
                with self.assertRaisesRegex(ValueError, "namespace"):
                    self.stage()
                self.assertFalse((self.output / "cache").exists())

    @unittest.skipUnless(os.environ.get("P4_RED") == "1", "SKIN ANIM recovery RED")
    def test_skin_animation_payload_drift_cannot_stage(self):
        for kind, raw in [("skin", b"SKIN" + b"\0" * 40), ("anim", ints(1, 2))]:
            with self.subTest(kind=kind):
                self.rows, self.requests = [], []
                self.output = self.root / f"package-drift-{kind}"
                request = self.source(473319, kind, raw)
                self.put(request["source_path"], b"different unreceipted bytes")
                with self.assertRaisesRegex(ValueError, "payload"):
                    self.stage()
                self.assertFalse((self.output / "cache").exists())

    @unittest.skipUnless(os.environ.get("P4_RED") == "1", "SKIN ANIM recovery RED")
    def test_other_companion_kinds_are_not_enabled(self):
        for kind, raw in [("bone", b"BONEdata"), ("skel", b"SKL1data")]:
            with self.subTest(kind=kind):
                self.rows, self.requests = [], []
                self.output = self.root / f"package-unsupported-{kind}"
                self.source(473319, kind, raw)
                with self.assertRaisesRegex(ValueError, "Unsupported"):
                    self.stage()

    @unittest.skipUnless(os.environ.get("P4_RED") == "1", "SKIN ANIM recovery RED")
    def test_existing_skin_animation_owners_are_immutable(self):
        for kind, raw in [("skin", b"SKIN" + b"\0" * 40), ("anim", ints(1, 2))]:
            with self.subTest(kind=kind):
                self.rows, self.requests = [], []
                self.output = self.root / f"package-existing-{kind}"
                self.source(473319, kind, raw)
                owner = {
                    "product": "wow",
                    "fdid": 473319,
                    "kind": kind,
                    "build": "12.1.5.70448",
                    "build_key": PARENT_KEY,
                    "path": f"products/wow/{PARENT_KEY}/models/473319.{kind}",
                    "bytes": len(raw),
                    "sha256": hashlib.sha256(raw).hexdigest(),
                    "content_key": hashlib.md5(raw).hexdigest(),
                }
                original = json.dumps({"version": 1, "assets": [owner]}).encode()
                self.put("cache/model-asset-index.json", original)
                with self.assertRaisesRegex(ValueError, "owner"):
                    self.stage()
                self.assertEqual(
                    (self.data / "cache/model-asset-index.json").read_bytes(), original
                )
                self.assertFalse((self.output / "cache").exists())


if __name__ == "__main__":
    unittest.main()
