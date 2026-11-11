import gzip
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from launcher_packs import build_packs


class LauncherRevisionTests(unittest.TestCase):
    def test_empty_bundle_publishes_small_root_and_verified_compressed_index(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'source'
            output = Path(directory) / 'output'
            source.mkdir()
            manifest = build_packs(source, output, {'files': []})
            root_bytes = (output / 'manifest-v3.json').read_bytes()
            self.assertLess(len(root_bytes), 512)
            root = json.loads(root_bytes)
            self.assertEqual(root['version'], 3)
            index = root['index']
            compressed = (output / 'indexes' / (index['sha256'] + '.json.gz')).read_bytes()
            self.assertEqual(hashlib.sha256(compressed).hexdigest(), index['sha256'])
            self.assertEqual(len(compressed), index['size'])
            body = gzip.decompress(compressed)
            self.assertEqual(len(body), index['uncompressed_size'])
            self.assertEqual(json.loads(body), manifest)
            build_packs(source, output, {'files': []})
            self.assertEqual((output / 'manifest-v3.json').read_bytes(), root_bytes)
