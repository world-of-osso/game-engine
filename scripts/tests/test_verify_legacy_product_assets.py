"""Legacy publication requires a matching authenticated product content key."""

import hashlib
import json
from pathlib import Path
import sys
import struct
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import verify_legacy_product_assets as verifier


class LegacyProductTests(unittest.TestCase):
    def fixture(self, raw=b"BLP2 authentic", content=None, existing=None):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        data = Path(temporary.name)
        source = data / "textures/42.blp"
        source.parent.mkdir()
        source.write_bytes(raw)
        identity = {"product": "wow", "build_key": "a" * 32, "build": "12.1.0.69933"}
        asset = {
            "product": "wow",
            "fdid": 42,
            "extension": "blp",
            "legacy_candidate_path": "textures/42.blp",
        }
        target = data / ("products/wow/" + "a" * 32 + "/textures/42.blp")
        if existing is not None:
            target.parent.mkdir(parents=True)
            target.write_bytes(existing)
        keys = {} if content is None else {42: content}
        return data, source, target, identity, asset, keys

    def test_matching_bytes_publish_receipt_without_changing_legacy_inode(self):
        raw = b"BLP2 authentic"
        data, source, target, identity, asset, keys = self.fixture(
            content=hashlib.md5(raw).digest()
        )
        inode = source.stat().st_ino
        row = verifier.publish_legacy(data, identity, asset, keys)
        self.assertEqual(row["status"], "verified")
        self.assertEqual(target.read_bytes(), raw)
        self.assertEqual(source.stat().st_ino, inode)
        index = json.loads((data / "cache/model-asset-index.json").read_text())
        self.assertEqual(
            index["assets"][0]["content_key"], hashlib.md5(raw).hexdigest()
        )
        self.assertEqual(index["assets"][0]["product"], "wow")

    def test_wrong_product_bytes_never_receive_receipt(self):
        data, source, target, identity, asset, keys = self.fixture(
            content=hashlib.md5(b"BLP2 other product").digest()
        )
        row = verifier.publish_legacy(data, identity, asset, keys)
        self.assertEqual(row["status"], "content_mismatch")
        self.assertFalse(target.exists())
        self.assertFalse((data / "cache/model-asset-index.json").exists())
        self.assertEqual(source.read_bytes(), b"BLP2 authentic")

    def test_root_absent_bytes_never_receive_receipt(self):
        data, source, target, identity, asset, keys = self.fixture()
        self.assertEqual(
            verifier.publish_legacy(data, identity, asset, keys)["status"],
            "root_absent",
        )
        self.assertFalse(target.exists())
        self.assertFalse((data / "cache/model-asset-index.json").exists())

    def test_existing_conflicting_scoped_bytes_remain_unchanged(self):
        raw = b"BLP2 authentic"
        data, source, target, identity, asset, keys = self.fixture(
            content=hashlib.md5(raw).digest(), existing=b"preserve"
        )
        row = verifier.publish_legacy(data, identity, asset, keys)
        self.assertEqual(row["status"], "scoped_conflict")
        self.assertEqual(target.read_bytes(), b"preserve")
        self.assertFalse((data / "cache/model-asset-index.json").exists())

    def test_authenticated_local_metadata_controls_the_published_content_key(self):
        self.check_authenticated_root("12.1.0.69933")

    def test_explicit_current_build_pin_authenticates_and_publishes(self):
        self.check_authenticated_root("12.1.5.70448")

    def check_authenticated_root(self, version):
        raw = b"BLP2 authentic"
        data, source, target, _, asset, _ = self.fixture()
        install = data / "install"
        content = hashlib.md5(raw).digest()
        root = struct.pack("<IIII", 1, 0, 2, 42) + content + bytes(8)
        root_ck = hashlib.md5(root).digest()
        root_ek, encoding_ek = b"r" * 16, b"e" * 16
        entry = bytes([1]) + len(root).to_bytes(5, "big") + root_ck + root_ek
        page = entry + bytes(1024 - len(entry))
        encoding = struct.pack(">2sBBBHHIIBI", b"EN", 1, 16, 16, 1, 1, 1, 0, 0, 0)
        encoding += root_ck + hashlib.md5(page).digest() + page
        indices, archive = {}, bytearray()
        for key, payload in [(root_ek, root), (encoding_ek, encoding)]:
            encoded = bytes(30) + b"BLTE" + bytes(4) + b"N" + payload
            indices[key[:9]] = (0, len(archive), len(encoded))
            archive.extend(encoded)
        archive_path = install / "Data/data/data.000"
        archive_path.parent.mkdir(parents=True)
        archive_path.write_bytes(archive)
        config = f"root = {root_ck.hex()}\nencoding = {hashlib.md5(encoding).hexdigest()} {encoding_ek.hex()}\n".encode()
        build = hashlib.md5(config).hexdigest()
        config_path = install / f"Data/config/{build[:2]}/{build[2:4]}/{build}"
        config_path.parent.mkdir(parents=True)
        config_path.write_bytes(config)
        product = {"Product": "wow", "Version": version, "Build Key": build}
        identity, keys = verifier.authenticate_root(
            install, product, {42}, indices, data, expected_build=version
        )
        self.assertEqual(keys, {42: content})
        self.assertEqual(
            verifier.publish_legacy(data, identity, asset, keys)["status"], "verified"
        )
        receipt = json.loads((data / "cache/model-asset-index.json").read_text())[
            "assets"
        ][0]
        self.assertEqual((data / receipt["path"]).read_bytes(), raw)
        # Even valid-looking metadata cannot be used after its content digest changes.
        archive[-1] ^= 1
        archive_path.write_bytes(archive)
        with self.assertRaisesRegex(ValueError, "content key mismatch"):
            verifier.authenticate_root(
                install, product, {42}, indices, data, expected_build=version
            )

    def test_unapproved_build_is_rejected_before_publication(self):
        with self.assertRaisesRegex(ValueError, "Unexpected actual build"):
            verifier.authenticate_root(
                Path("unused"),
                {"Product": "wow", "Version": "12.1.1.70000"},
                {42},
                {},
                Path("unused"),
                expected_build="12.1.5.70448",
            )

    def test_explicit_build_pins_are_not_historical_defaults(self):
        self.assertEqual(
            verifier.parse_build_pins(["wow=12.1.5.70448"]), {"wow": "12.1.5.70448"}
        )
        for pins in [[], ["wow"], ["wow="], ["=12.1.5.70448"], ["wow=one", "wow=two"]]:
            with self.subTest(pins=pins), self.assertRaises(ValueError):
                verifier.parse_build_pins(pins)

    def skin_fixture(self, reference=43, corrupt_parent=False):
        raw = b"SKIN authentic"
        data, _, _, identity, _, _ = self.fixture()
        parent = (
            b"MD21" + struct.pack("<I", 0) + b"SFID" + struct.pack("<II", 4, reference)
        )
        (data / "models").mkdir()
        parent_path = data / ("products/wow/" + "a" * 32 + "/models/42.m2")
        parent_path.parent.mkdir(parents=True)
        parent_path.write_bytes(parent)
        (data / "models/4200.skin").write_bytes(raw)
        asset = {
            "product": "wow",
            "fdid": 43,
            "extension": "skin",
            "actual_build": "a" * 32,
            "legacy_candidate_path": "models/4200.skin",
        }
        roots = {
            42: hashlib.md5(b"other" if corrupt_parent else parent).digest(),
            43: hashlib.md5(raw).digest(),
        }
        return data, identity, asset, roots, raw

    def test_authenticated_sfid_companion_publishes_alias_and_canonical_receipt(self):
        data, identity, asset, roots, raw = self.skin_fixture()
        row = verifier.publish_model_companion(data, identity, asset, roots)
        self.assertEqual(row["status"], "verified")
        index = json.loads((data / "cache/model-asset-index.json").read_text())
        alias = index["aliases"][0]
        self.assertEqual(alias["fdid"], 43)
        self.assertEqual((data / alias["path"]).read_bytes(), raw)
        self.assertEqual((data / index["assets"][0]["path"]).read_bytes(), raw)
        self.assertEqual(index["chains"][0]["dependencies"]["42.m2"], {"skin": [43]})

    def test_unrelated_skin_never_publishes_alias_or_receipt(self):
        data, identity, asset, roots, _ = self.skin_fixture(reference=44)
        row = verifier.publish_model_companion(data, identity, asset, roots)
        self.assertEqual(row["status"], "companion_mismatch")
        self.assertFalse((data / "cache/model-asset-index.json").exists())

    def test_unauthenticated_parent_never_publishes_alias_or_receipt(self):
        data, identity, asset, roots, _ = self.skin_fixture(corrupt_parent=True)
        row = verifier.publish_model_companion(data, identity, asset, roots)
        self.assertEqual(row["status"], "parent_content_mismatch")
        self.assertFalse((data / "cache/model-asset-index.json").exists())

    def test_authenticated_skid_companion_publishes_parent_alias(self):
        data, identity, asset, roots, _ = self.skin_fixture()
        parent = b"MD21" + struct.pack("<I", 0) + b"SKID" + struct.pack("<II", 4, 43)
        (data / ("products/wow/" + "a" * 32 + "/models/42.m2")).write_bytes(parent)
        raw = b"SKL1 authentic skeleton"
        (data / "models/42.skel").write_bytes(raw)
        asset.update(
            kind="missing_scoped_bytes",
            extension="skel",
            legacy_candidate_path="models/42.skel",
        )
        roots.update({42: hashlib.md5(parent).digest(), 43: hashlib.md5(raw).digest()})
        row = verifier.publish_miss(data, identity, asset, roots)
        self.assertEqual(row["status"], "verified")
        index = json.loads((data / "cache/model-asset-index.json").read_text())
        alias = index["aliases"][0]
        self.assertEqual(alias["fdid"], 43)
        self.assertEqual(alias["path"], "products/wow/" + "a" * 32 + "/models/42.skel")
        self.assertEqual((data / alias["path"]).read_bytes(), raw)

    def test_scoped_texture_miss_publishes_only_authenticated_texture(self):
        raw = b"BLP2 authentic"
        data, _, target, identity, asset, roots = self.fixture(
            content=hashlib.md5(raw).digest()
        )
        asset.update(kind="missing_scoped_bytes", actual_build="a" * 32)
        row = verifier.publish_miss(data, identity, asset, roots)
        self.assertEqual(row["status"], "verified")
        self.assertEqual(target.read_bytes(), raw)

    def test_existing_fragment_mapping_is_authenticated_without_base_publication(self):
        raw = b"BLP2 authentic"
        data, _, target, identity, asset, roots = self.fixture(
            content=hashlib.md5(raw).digest()
        )
        receipt = verifier.chains.create_asset_receipt(
            identity, 42, "blp", "textures/42.blp", raw, hashlib.md5(raw).hexdigest()
        )
        verifier.chains.write_asset_files(data, [(receipt, raw)])
        fragment = data / "cache/model-asset-index-other.json"
        fragment.parent.mkdir()
        fragment.write_text(
            json.dumps({"version": 1, "assets": [receipt], "aliases": []})
        )
        before, inode = fragment.read_bytes(), target.stat().st_ino
        row = verifier.publish_legacy(data, identity, asset, roots)
        self.assertEqual(row["status"], "verified")
        self.assertEqual(row["receipt_location"], str(fragment))
        self.assertFalse((data / "cache/model-asset-index.json").exists())
        self.assertEqual(fragment.read_bytes(), before)
        self.assertEqual(target.stat().st_ino, inode)

    def test_fragment_owned_companion_is_not_republished_in_base(self):
        data, identity, asset, roots, raw = self.skin_fixture()
        receipt = verifier.chains.create_asset_receipt(
            identity, 43, "skin", "models/43.skin", raw, roots[43].hex()
        )
        alias = verifier.chains.create_asset_receipt(
            identity, 43, "skin", "models/4200.skin", raw, roots[43].hex()
        )
        verifier.chains.write_asset_files(data, [(receipt, raw), (alias, raw)])
        fragment = data / "cache/model-asset-index-other.json"
        fragment.parent.mkdir()
        fragment.write_text(
            json.dumps({"version": 1, "assets": [receipt], "aliases": [alias]})
        )
        row = verifier.publish_model_companion(data, identity, asset, roots)
        self.assertEqual(row["status"], "verified")
        self.assertFalse((data / "cache/model-asset-index.json").exists())
        self.assertEqual(row["alias"], alias)

    def test_conflicting_fragment_mapping_is_preserved_and_rejected(self):
        raw = b"BLP2 authentic"
        data, _, target, identity, asset, roots = self.fixture(
            content=hashlib.md5(raw).digest()
        )
        receipt = verifier.chains.create_asset_receipt(
            identity,
            42,
            "blp",
            "textures/42.blp",
            b"BLP2 other",
            hashlib.md5(b"BLP2 other").hexdigest(),
        )
        fragment = data / "cache/model-asset-index-other.json"
        fragment.parent.mkdir()
        fragment.write_text(
            json.dumps({"version": 1, "assets": [receipt], "aliases": []})
        )
        before = fragment.read_bytes()
        row = verifier.publish_legacy(data, identity, asset, roots)
        self.assertEqual(row["status"], "scoped_conflict")
        self.assertEqual(fragment.read_bytes(), before)
        self.assertFalse(target.exists())
        self.assertFalse((data / "cache/model-asset-index.json").exists())

    def test_shared_alias_audit_rows_do_not_block_an_unrelated_companion(self):
        data, identity, asset, roots, raw = self.skin_fixture()
        shared = verifier.chains.create_asset_receipt(
            identity, 43, "skin", "models/9900.skin", raw, roots[43].hex()
        )
        (data / "cache").mkdir()
        for name in ["one", "two"]:
            (data / f"cache/model-asset-index-{name}.json").write_text(
                json.dumps({"version": 1, "assets": [], "aliases": [shared]})
            )
        row = verifier.publish_model_companion(data, identity, asset, roots)
        self.assertEqual(row["status"], "verified")
        self.assertEqual((data / row["alias"]["path"]).read_bytes(), raw)
        self.assertEqual(
            json.loads((data / "cache/model-asset-index.json").read_text())["assets"][
                0
            ]["fdid"],
            43,
        )

    def test_missing_bytes_do_not_publish(self):
        data, source, target, identity, asset, keys = self.fixture(
            content=hashlib.md5(b"BLP2 authentic").digest()
        )
        source.unlink()
        self.assertEqual(
            verifier.publish_legacy(data, identity, asset, keys)["status"],
            "bytes_absent",
        )
        self.assertFalse(target.exists())


if __name__ == "__main__":
    unittest.main()
