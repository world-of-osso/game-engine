import hashlib
import importlib.util
import random
import tempfile
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).parents[1]))
from launcher_packs import build_packs
spec = importlib.util.spec_from_file_location('verify_launcher_manifest', Path(__file__).parents[1] / 'verify_launcher_manifest.py')
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class VerifyLauncherManifestTest(unittest.TestCase):
    def test_reconstructs_multichunk_file_and_rejects_corrupt_pack(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            source.mkdir()
            data = random.Random(42).randbytes(6 * 1024 * 1024)
            (source / 'asset').write_bytes(data)
            output = root / 'output'
            v2 = build_packs(source, output)
            v1 = {'version': 'today', 'files': [{'path': 'asset', 'size': len(data),
                                                 'sha256': hashlib.sha256(data).hexdigest()}]}
            self.assertGreater(len(v2['files'][0]['chunks']), 1)
            verify.verify(v1, v2, output / 'packs')
            pack = output / 'packs' / v2['files'][0]['chunks'][0]['pack']
            corrupted = bytearray(pack.read_bytes())
            corrupted[-1] ^= 1
            pack.write_bytes(corrupted)
            with self.assertRaisesRegex(ValueError, 'pack'):
                verify.verify(v1, v2, output / 'packs')

    def test_download_size_counts_unique_new_packs(self):
        old = {'files': [{'chunks': [{'pack': 'a' * 64, 'pack_size': 20}]}]}
        new = {'files': [{'chunks': [{'pack': 'a' * 64, 'pack_size': 20},
                                    {'pack': 'b' * 64, 'pack_size': 30}]},
                         {'chunks': [{'pack': 'b' * 64, 'pack_size': 30}]}]}
        self.assertEqual(verify.additional_pack_bytes(old, new), (1, 30))

    def test_rejects_mismatched_file_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            v1 = {'files': [{'path': 'a', 'size': 1, 'sha256': 'a'}]}
            v2 = {'files': [{'path': 'a', 'size': 2, 'sha256': 'a', 'chunks': []}]}
            with self.assertRaisesRegex(ValueError, 'a'):
                verify.verify(v1, v2, Path(directory))


if __name__ == '__main__':
    unittest.main()
