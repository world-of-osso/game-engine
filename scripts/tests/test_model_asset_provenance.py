"""Actual-build identity and verified byte receipts, not legacy filename inference."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "import_model_asset_chains.py"
SPEC = importlib.util.spec_from_file_location("model_chains", SCRIPT)
chains = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(chains)


class ModelAssetProvenanceTests(unittest.TestCase):
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

    def test_receipt_rejects_borrowed_or_zero_filled_bytes(self):
        with self.assertRaisesRegex(ValueError, "content key"):
            chains.create_asset_receipt({"product": "wow_classic_beta", "build_key": "e8dd824cf6c3d96cd01f804ca2ea5a63", "build": "1.60.1.70291"}, 1100087, "m2", "models/1100087.m2", b"retail-model", hashlib.md5(b"forever-model").hexdigest())


if __name__ == "__main__":
    unittest.main()
