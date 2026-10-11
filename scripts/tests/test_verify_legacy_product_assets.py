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
    def fixture(self, raw=b'BLP2 authentic', content=None, existing=None):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        data = Path(temporary.name)
        source = data / 'textures/42.blp'
        source.parent.mkdir()
        source.write_bytes(raw)
        identity = {'product': 'wow', 'build_key': 'a' * 32, 'build': '12.1.0.69933'}
        asset = {'product': 'wow', 'fdid': 42, 'extension': 'blp',
                 'legacy_candidate_path': 'textures/42.blp'}
        target = data / ('products/wow/' + 'a' * 32 + '/textures/42.blp')
        if existing is not None:
            target.parent.mkdir(parents=True)
            target.write_bytes(existing)
        keys = {} if content is None else {42: content}
        return data, source, target, identity, asset, keys

    def test_matching_bytes_publish_receipt_without_changing_legacy_inode(self):
        raw = b'BLP2 authentic'
        data, source, target, identity, asset, keys = self.fixture(content=hashlib.md5(raw).digest())
        inode = source.stat().st_ino
        row = verifier.publish_legacy(data, identity, asset, keys)
        self.assertEqual(row['status'], 'verified')
        self.assertEqual(target.read_bytes(), raw)
        self.assertEqual(source.stat().st_ino, inode)
        index = json.loads((data / 'cache/model-asset-index.json').read_text())
        self.assertEqual(index['assets'][0]['content_key'], hashlib.md5(raw).hexdigest())
        self.assertEqual(index['assets'][0]['product'], 'wow')

    def test_wrong_product_bytes_never_receive_receipt(self):
        data, source, target, identity, asset, keys = self.fixture(content=hashlib.md5(b'BLP2 other product').digest())
        row = verifier.publish_legacy(data, identity, asset, keys)
        self.assertEqual(row['status'], 'content_mismatch')
        self.assertFalse(target.exists())
        self.assertFalse((data / 'cache/model-asset-index.json').exists())
        self.assertEqual(source.read_bytes(), b'BLP2 authentic')

    def test_root_absent_bytes_never_receive_receipt(self):
        data, source, target, identity, asset, keys = self.fixture()
        self.assertEqual(verifier.publish_legacy(data, identity, asset, keys)['status'], 'root_absent')
        self.assertFalse(target.exists())
        self.assertFalse((data / 'cache/model-asset-index.json').exists())

    def test_existing_conflicting_scoped_bytes_remain_unchanged(self):
        raw = b'BLP2 authentic'
        data, source, target, identity, asset, keys = self.fixture(content=hashlib.md5(raw).digest(), existing=b'preserve')
        row = verifier.publish_legacy(data, identity, asset, keys)
        self.assertEqual(row['status'], 'scoped_conflict')
        self.assertEqual(target.read_bytes(), b'preserve')
        self.assertFalse((data / 'cache/model-asset-index.json').exists())

    def test_authenticated_local_metadata_controls_the_published_content_key(self):
        raw = b'BLP2 authentic'
        data, source, target, _, asset, _ = self.fixture()
        install = data / 'install'
        content = hashlib.md5(raw).digest()
        root = struct.pack('<IIII', 1, 0, 2, 42) + content + bytes(8)
        root_ck = hashlib.md5(root).digest()
        root_ek, encoding_ek = b'r' * 16, b'e' * 16
        entry = bytes([1]) + len(root).to_bytes(5, 'big') + root_ck + root_ek
        page = entry + bytes(1024 - len(entry))
        encoding = struct.pack('>2sBBBHHIIBI', b'EN', 1, 16, 16, 1, 1, 1, 0, 0, 0)
        encoding += root_ck + hashlib.md5(page).digest() + page
        indices, archive = {}, bytearray()
        for key, payload in [(root_ek, root), (encoding_ek, encoding)]:
            encoded = bytes(30) + b'BLTE' + bytes(4) + b'N' + payload
            indices[key[:9]] = (0, len(archive), len(encoded))
            archive.extend(encoded)
        archive_path = install / 'Data/data/data.000'
        archive_path.parent.mkdir(parents=True)
        archive_path.write_bytes(archive)
        config = f'root = {root_ck.hex()}\nencoding = {hashlib.md5(encoding).hexdigest()} {encoding_ek.hex()}\n'.encode()
        build = hashlib.md5(config).hexdigest()
        config_path = install / f'Data/config/{build[:2]}/{build[2:4]}/{build}'
        config_path.parent.mkdir(parents=True)
        config_path.write_bytes(config)
        product = {'Product': 'wow', 'Version': '12.1.0.69933', 'Build Key': build}
        identity, keys = verifier.authenticate_root(install, product, {42}, indices, data)
        self.assertEqual(keys, {42: content})
        self.assertEqual(verifier.publish_legacy(data, identity, asset, keys)['status'], 'verified')
        receipt = json.loads((data / 'cache/model-asset-index.json').read_text())['assets'][0]
        self.assertEqual((data / receipt['path']).read_bytes(), raw)
        # Even valid-looking metadata cannot be used after its content digest changes.
        archive[-1] ^= 1
        archive_path.write_bytes(archive)
        with self.assertRaisesRegex(ValueError, 'content key mismatch'):
            verifier.authenticate_root(install, product, {42}, indices, data)

    def test_unapproved_build_is_rejected_before_publication(self):
        with self.assertRaisesRegex(ValueError, 'Unexpected actual build'):
            verifier.authenticate_root(Path('unused'), {'Product': 'wow', 'Version': '12.1.1.70000'}, {42}, {}, Path('unused'))

    def test_missing_bytes_do_not_publish(self):
        data, source, target, identity, asset, keys = self.fixture(content=hashlib.md5(b'BLP2 authentic').digest())
        source.unlink()
        self.assertEqual(verifier.publish_legacy(data, identity, asset, keys)['status'], 'bytes_absent')
        self.assertFalse(target.exists())


if __name__ == '__main__':
    unittest.main()
