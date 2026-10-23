"""Actual CLI publication on disposable canonical trees, never production data."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "publish_model_asset_fragment.py"


def document(assets=(), aliases=(), chains=()):
    return {
        "version": 1,
        "assets": list(assets),
        "aliases": list(aliases),
        "chains": list(chains),
    }


def receipt(fdid, raw, key="a", kind="m2"):
    directory = "textures" if kind == "blp" else "models"
    return {
        "product": "wow",
        "fdid": fdid,
        "kind": kind,
        "build": "12.1.0.69933",
        "build_key": key * 32,
        "path": f"products/wow/{key * 32}/{directory}/{fdid}.{kind}",
        "bytes": len(raw),
        "sha256": hashlib.sha256(raw).hexdigest(),
        "content_key": hashlib.md5(raw).hexdigest(),
    }


class PublishModelFragmentTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.data, self.package = self.root / "canonical", self.root / "package"
        self.backup = self.root / "pre-publication"
        self.filename = "model-asset-index-recovery.json"
        self.old_raw, self.new_raw = b"MD21 old owner", b"BLP2 historical texture"
        self.old = receipt(42, self.old_raw)
        self.new = receipt(43, self.new_raw, kind="blp")
        self.alias = {
            **self.new,
            "build_key": "b" * 32,
            "path": f"products/wow/{'b' * 32}/textures/43.blp",
            "source_build": self.new["build"],
            "source_build_key": self.new["build_key"],
        }
        self.write(
            self.data / "cache/model-asset-index.json",
            json.dumps(document([self.old])).encode(),
        )
        self.original_index = (
            json.dumps(document(chains=[{"keep": "existing chain"}])) + "\n"
        ).encode()
        self.write(self.data / "cache" / self.filename, self.original_index)
        self.write(self.data / self.old["path"], self.old_raw)
        self.write(self.data / self.alias["path"], self.new_raw)
        self.write(self.package / self.old["path"], self.old_raw)
        self.write(self.package / self.new["path"], self.new_raw)
        self.write(self.package / self.alias["path"], self.new_raw)
        self.fragment = document([self.old, self.new], [self.alias])
        self.save_fragment()

    def write(self, path, raw):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)

    def save_fragment(self):
        self.write(
            self.package / "cache" / self.filename, json.dumps(self.fragment).encode()
        )

    def snapshot(self, root):
        return {
            str(p.relative_to(root)): p.read_bytes()
            for p in root.rglob("*")
            if p.is_file()
        }

    def invoke(self, *args):
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--canonical-data",
                str(self.data),
                "--package",
                str(self.package),
                "--fragment",
                f"cache/{self.filename}",
                "--backup",
                str(self.backup),
                *args,
            ],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_default_dry_run_reports_exact_changes_without_writes(self):
        before, source = self.snapshot(self.data), self.snapshot(self.package)
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["mode"], "dry-run")
        self.assertEqual((report["new_owners"], report["new_aliases"]), (1, 1))
        self.assertEqual(
            report["canonical_write_paths"],
            [
                str(self.data / self.new["path"]),
                str(self.data / "cache" / self.filename),
            ],
        )
        self.assertEqual(report["payload_bytes_to_copy"], len(self.new_raw))
        self.assertEqual(report["identical_payload_files"], 2)
        self.assertGreater(
            report["space_required_bytes"], len(self.new_raw) + len(self.original_index)
        )
        self.assertGreater(report["disk_free_bytes"], report["space_required_bytes"])
        self.assertEqual(self.snapshot(self.data), before)
        self.assertEqual(self.snapshot(self.package), source)
        self.assertFalse(self.backup.exists())

    def test_apply_merges_preserves_pre_index_and_rerun_is_noop(self):
        result = self.invoke("--apply")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.data / self.new["path"]).read_bytes(), self.new_raw)
        index = json.loads((self.data / "cache" / self.filename).read_text())
        self.assertEqual(index["assets"], [self.new])
        self.assertEqual(index["aliases"], [self.alias])
        self.assertEqual(index["chains"], [{"keep": "existing chain"}])
        self.assertEqual(
            (self.backup / f"pre-{self.filename}").read_bytes(), self.original_index
        )
        self.assertEqual(
            json.loads((self.data / "cache/model-asset-index.json").read_text())[
                "assets"
            ],
            [self.old],
        )
        before = self.snapshot(self.root)
        again = self.invoke("--apply")
        self.assertEqual(again.returncode, 0, again.stderr)
        self.assertTrue(json.loads(again.stdout)["noop"])
        self.assertEqual(json.loads(again.stdout)["canonical_write_paths"], [])
        self.assertEqual(self.snapshot(self.root), before)

    def test_new_fragment_records_absent_pre_index_and_is_idempotent(self):
        (self.data / "cache" / self.filename).unlink()
        result = self.invoke("--apply")
        self.assertEqual(result.returncode, 0, result.stderr)
        record = json.loads(
            (self.backup / f"pre-{self.filename}.absent.json").read_text()
        )
        self.assertTrue(record["previously_absent"])
        index = json.loads((self.data / "cache" / self.filename).read_text())
        self.assertEqual(index["assets"], [self.new])
        self.assertEqual(index["aliases"], [self.alias])
        before = self.snapshot(self.root)
        again = self.invoke("--apply")
        self.assertEqual(again.returncode, 0, again.stderr)
        self.assertTrue(json.loads(again.stdout)["noop"])
        self.assertEqual(self.snapshot(self.root), before)

    def test_different_hash_existing_owner_is_error_before_any_write(self):
        wrong = receipt(43, b"different owner", key="c", kind="blp")
        self.write(
            self.data / "cache/model-asset-index-other.json",
            json.dumps(document([wrong])).encode(),
        )
        before = self.snapshot(self.data)
        result = self.invoke("--apply")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("conflicting staged receipt", result.stderr)
        self.assertEqual(self.snapshot(self.data), before)
        self.assertFalse(self.backup.exists())

    def test_different_hash_physical_destination_is_error_before_any_write(self):
        self.write(self.data / self.new["path"], b"wrong physical destination")
        before = self.snapshot(self.data)
        result = self.invoke("--apply")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("path conflict", result.stderr)
        self.assertEqual(self.snapshot(self.data), before)
        self.assertFalse(self.backup.exists())

    def test_source_content_md5_is_checked_not_only_sha256(self):
        self.fragment["assets"][1]["content_key"] = "0" * 32
        self.save_fragment()
        before = self.snapshot(self.data)
        result = self.invoke("--apply")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("payload identity", result.stderr)
        self.assertEqual(self.snapshot(self.data), before)
        self.assertFalse(self.backup.exists())

    def test_duplicate_current_canonical_owners_are_refused(self):
        self.write(
            self.data / "cache/model-asset-index-duplicate.json",
            json.dumps(document([self.old])).encode(),
        )
        before = self.snapshot(self.data)
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Duplicate asset receipt", result.stderr)
        self.assertEqual(self.snapshot(self.data), before)
        self.assertFalse(self.backup.exists())


if __name__ == "__main__":
    unittest.main()
