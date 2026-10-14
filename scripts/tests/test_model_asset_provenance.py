"""Actual-build identity and verified byte receipts, not legacy filename inference."""
import hashlib
import importlib.util
import json
import sqlite3
import sys
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "import_model_asset_chains.py"
sys.path.insert(0, str(SCRIPT.parent))
SPEC = importlib.util.spec_from_file_location("model_chains", SCRIPT)
chains = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(chains)


class ModelAssetProvenanceTests(unittest.TestCase):
    def test_verified_wmo_root_and_groups_publish_without_m2_relabelling(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            key = "dcfc90fffd79ba00406ae46f5f657592"
            identity = {"product": "wow", "build_key": key, "build": "12.1.0.69933"}
            raw = b"REVM" + (4).to_bytes(4, "little") + (17).to_bytes(4, "little")
            resolution = root / "resolution.sqlite"
            with sqlite3.connect(resolution) as db:
                db.execute("CREATE TABLE resolution (fdid INTEGER, content_key BLOB)")
                db.executemany("INSERT INTO resolution VALUES (?, ?)",
                               [(6429362, hashlib.md5(raw).digest()),
                                (6429658, hashlib.md5(raw).digest())])
            db.close()
            identity["resolution_sha256"] = hashlib.sha256(resolution.read_bytes()).hexdigest()
            assets = []
            for fdid in [6429362, 6429658]:
                source = root / f"{fdid}.wmo"
                source.write_bytes(raw)
                assets.append({"product": "wow", "build_key": key, "fdid": fdid,
                               "kind": "wmo", "source_path": str(source),
                               "content_key": hashlib.md5(raw).hexdigest(),
                               "sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw)})
            staged = root / "staged.json"
            staged.write_text(json.dumps({"source_identity": identity, "assets": assets,
                                          "dependencies": {"6429362.wmo": {"wmo": [6429658]}},
                                          "metadata": [], "failures": []}))
            data = root / "data"
            result = chains.import_staged_chain(data, "wow", staged, resolution, "housing-wmo")
            self.assertEqual([(asset["fdid"], asset["kind"]) for asset in result["assets"]],
                             [(6429362, "wmo"), (6429658, "wmo")])
            for asset in result["assets"]:
                self.assertEqual((data / asset["path"]).read_bytes(), raw)
                self.assertTrue(asset["path"].endswith(".wmo"))
            self.assertFalse(list(data.rglob("*.m2")))

    def test_installed_asset_build_is_not_relabelled_to_metadata_build(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".build.info").write_text(
                "Active!DEC:1|Build Key!HEX:16|Version!STRING:0|Product!STRING:0\n"
                "1|e8dd824cf6c3d96cd01f804ca2ea5a63|1.60.1.70291|wow_classic_beta\n"
            )
            config = root / "Data/config/e8/dd/e8dd824cf6c3d96cd01f804ca2ea5a63"
            config.parent.mkdir(parents=True)
            config.write_text("root = 0123456789abcdef0123456789abcdef\n")
            self.assertEqual(chains.read_installed_identity(root, "wow_classic_beta"), {
                "product": "wow_classic_beta",
                "build_key": "e8dd824cf6c3d96cd01f804ca2ea5a63",
                "build": "1.60.1.70291",
                "build_config_sha256": hashlib.sha256(config.read_bytes()).hexdigest(),
            })

    def test_receipt_namespaces_same_fdid_and_retains_actual_byte_identity(self):
        for product, key, raw in [
            ("wow", "dcfc90fffd79ba00406ae46f5f657592", b"retail-model"),
            ("wow_classic_beta", "e8dd824cf6c3d96cd01f804ca2ea5a63", b"forever-model"),
        ]:
            identity = {"product": product, "build_key": key, "build": "actual-build"}
            receipt = chains.create_asset_receipt(identity, 1100087, "m2", "models/1100087.m2", raw, hashlib.md5(raw).hexdigest())
            self.assertEqual(receipt, {
                **identity, "fdid": 1100087, "kind": "m2",
                "path": f"products/{product}/{key}/models/1100087.m2",
                "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest(),
                "content_key": hashlib.md5(raw).hexdigest(),
            })

    def test_import_publishes_frozen_build_after_install_advances_and_keeps_retail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            install, data = root / "install", root / "data"
            key = "e8dd824cf6c3d96cd01f804ca2ea5a63"
            config = install / f"Data/config/e8/dd/{key}"
            config.parent.mkdir(parents=True)
            config.write_text("root = 0123456789abcdef0123456789abcdef\n")
            (install / ".build.info").write_text(
                "Active!DEC:1|Build Key!HEX:16|Version!STRING:0|Product!STRING:0\n"
                f"1|{key}|1.60.1.70291|wow_classic_beta\n"
            )
            frozen_identity = chains.read_installed_identity(install, "wow_classic_beta")
            raw = b"MD21" + (4).to_bytes(4, "little") + b"MD20"
            source = root / "1100087.dat"
            source.write_bytes(raw)
            resolution = root / "resolution.sqlite"
            connection = sqlite3.connect(resolution)
            connection.execute("CREATE TABLE resolution (fdid INTEGER, content_key BLOB)")
            connection.execute("INSERT INTO resolution VALUES (?, ?)", (1100087, hashlib.md5(raw).digest()))
            connection.commit()
            connection.close()
            frozen_identity["resolution_sha256"] = hashlib.sha256(resolution.read_bytes()).hexdigest()
            staged = root / "chain-receipts.json"
            staged.write_text(json.dumps({"source_identity": frozen_identity, "assets": [{
                "product": "wow_classic_beta", "build_key": key, "fdid": 1100087,
                "kind": "m2", "source_path": str(source), "sha256": hashlib.sha256(raw).hexdigest(),
                "content_key": hashlib.md5(raw).hexdigest(), "bytes": len(raw),
            }], "dependencies": {}, "failures": []}))
            retail = data / "products/wow/dcfc90fffd79ba00406ae46f5f657592/models/1100087.m2"
            retail.parent.mkdir(parents=True)
            retail.write_bytes(b"retail-sentinel")
            # Extraction survives an upgrade that deletes its former build config.
            advanced_key = "029dc2e024aadc9e0d9073ac37126868"
            (install / ".build.info").write_text(
                "Active!DEC:1|Build Key!HEX:16|Version!STRING:0|Product!STRING:0\n"
                f"1|{advanced_key}|1.60.1.70334|wow_classic_beta\n"
            )
            config.unlink()
            advanced_config = install / f"Data/config/02/9d/{advanced_key}"
            advanced_config.parent.mkdir(parents=True)
            advanced_config.write_text("root = 99999999999999999999999999999999\n")
            try:
                result = chains.import_staged_chain(data, "wow_classic_beta", staged, resolution)
            except ValueError as error:
                self.fail(f"Verified frozen source rejected after install advanced: {error}")
            self.assertEqual(result.get("version"), 1)
            asset = result["assets"][0]
            self.assertEqual(asset["build"], "1.60.1.70291")
            self.assertEqual((data / asset["path"]).read_bytes(), raw)
            self.assertEqual(retail.read_bytes(), b"retail-sentinel")
            self.assertEqual(json.loads((data / "cache/model-asset-index.json").read_text()), result)

    def test_incremental_frozen_publication_retains_existing_model_chains(self):
        with tempfile.TemporaryDirectory() as directory:
            data = Path(directory)
            identity = {"product": "wow_classic_beta", "build_key": "e8dd824cf6c3d96cd01f804ca2ea5a63", "build": "1.60.1.70291"}
            for fdid in [1100087, 1100258]:
                raw = str(fdid).encode()
                receipt = chains.create_asset_receipt(identity, fdid, "m2", f"models/{fdid}.m2", raw, hashlib.md5(raw).hexdigest())
                chains.write_asset_files(data, [(receipt, raw)])
                result = chains.write_asset_index(data, identity,
                    {"dependencies": {f"{fdid}.m2": {}}, "metadata": []}, [(receipt, raw)], [])
            self.assertEqual(result["chains"][0]["dependencies"], {"1100087.m2": {}, "1100258.m2": {}})
            self.assertEqual([row["fdid"] for row in result["assets"]], [1100087, 1100258])
            for row in result["assets"]:
                self.assertEqual(hashlib.md5((data / row["path"]).read_bytes()).hexdigest(), row["content_key"])

    def test_additive_fragment_preserves_existing_index_and_rejects_conflicts(self):
        with tempfile.TemporaryDirectory() as directory:
            data = Path(directory)
            identity = {"product": "wow", "build_key": "dcfc90fffd79ba00406ae46f5f657592", "build": "12.1.0.69933"}
            def asset(fdid, raw):
                receipt = chains.create_asset_receipt(identity, fdid, "m2", f"models/{fdid}.m2", raw, hashlib.md5(raw).hexdigest())
                return receipt, raw
            squirrel = asset(1129448, b"fixture squirrel")
            soul = asset(574802, b"fixture soul")
            chains.write_asset_index(data, identity, {"dependencies": {}, "metadata": []}, [squirrel], [])
            base = data / "cache/model-asset-index.json"
            original, inode = base.read_bytes(), base.stat().st_ino
            staged = {"dependencies": {"574802.m2": {}}, "metadata": []}
            fragment = chains.write_asset_fragment(data, identity, staged, [squirrel, soul], [], "petbattle")
            self.assertEqual([r["fdid"] for r in fragment["assets"]], [574802])
            self.assertEqual((base.read_bytes(), base.stat().st_ino), (original, inode))
            path = data / "cache/model-asset-index-petbattle.json"
            self.assertEqual(json.loads(path.read_text()), fragment)
            with self.assertRaisesRegex(ValueError, "conflicting existing receipt"):
                chains.write_asset_fragment(data, identity, staged, [asset(1129448, b"wrong squirrel")], [], "conflict")
            self.assertFalse((data / "cache/model-asset-index-conflict.json").exists())
            with self.assertRaisesRegex(ValueError, "fragment name"):
                chains.write_asset_fragment(data, identity, staged, [soul], [], "../escape")

    def test_new_build_fragment_supersedes_base_and_fragment_receipts(self):
        for same_bytes in (True, False):
            with self.subTest(same_bytes=same_bytes), tempfile.TemporaryDirectory() as directory:
                data = Path(directory)
                old = {"product": "wow", "build_key": "1" * 32, "build": "12.1.0.69933"}
                new = {"product": "wow", "build_key": "2" * 32, "build": "12.1.5.70391"}
                def asset(identity, fdid, raw):
                    return chains.create_asset_receipt(identity, fdid, "blp", f"textures/{fdid}.blp", raw, hashlib.md5(raw).hexdigest()), raw
                first = asset(old, 126862, b"old texture")
                second = asset(old, 126863, b"old texture")
                chains.write_asset_files(data, [first, second])
                chains.write_asset_index(data, old, {"dependencies": {}, "metadata": []}, [first], [])
                chains.write_asset_fragment(data, old, {"dependencies": {}, "metadata": []}, [second], [], "prior")
                raw = b"old texture" if same_bytes else b"changed texture"
                current = [asset(new, fdid, raw) for fdid in (126862, 126863)]
                chains.write_asset_files(data, current)
                chains.write_asset_fragment(data, new, {"dependencies": {}, "metadata": []}, current, [], "current")
                rows = [r for p in (data / "cache").glob("model-asset-index*.json") for r in json.loads(p.read_text())["assets"]]
                self.assertEqual(len(rows), 2)
                self.assertEqual({r["build_key"] for r in rows}, {new["build_key"]})
                for row in rows:
                    self.assertEqual((data / row["path"]).read_bytes(), raw)
                with self.assertRaisesRegex(ValueError, "stale"):
                    chains.write_asset_fragment(data, old, {"dependencies": {}, "metadata": []}, [first], [], "obsolete")

    def test_existing_different_scoped_bytes_are_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            data = Path(directory)
            receipt = {"path": "products/wow/build/models/42.m2"}
            path = data / receipt["path"]
            path.parent.mkdir(parents=True)
            path.write_bytes(b"existing-build")
            with self.assertRaisesRegex(ValueError, "conflicting existing asset"):
                chains.write_asset_files(data, [(receipt, b"new-build")])
            self.assertEqual(path.read_bytes(), b"existing-build")

    def test_existing_matching_scoped_bytes_keep_their_inode(self):
        with tempfile.TemporaryDirectory() as directory:
            data = Path(directory)
            receipt = {"path": "products/wow/build/models/42.m2"}
            path = data / receipt["path"]
            path.parent.mkdir(parents=True)
            path.write_bytes(b"same-build")
            inode = path.stat().st_ino
            chains.write_asset_files(data, [(receipt, b"same-build")])
            self.assertEqual(path.stat().st_ino, inode)
            self.assertEqual(path.read_bytes(), b"same-build")

    def test_receipt_rejects_borrowed_or_zero_filled_bytes(self):
        with self.assertRaisesRegex(ValueError, "content key"):
            chains.create_asset_receipt({"product": "wow_classic_beta", "build_key": "e8dd824cf6c3d96cd01f804ca2ea5a63", "build": "1.60.1.70291"}, 1100087, "m2", "models/1100087.m2", b"retail-model", hashlib.md5(b"forever-model").hexdigest())


if __name__ == "__main__":
    unittest.main()
