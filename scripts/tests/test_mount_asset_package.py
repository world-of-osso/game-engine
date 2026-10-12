import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class PublicationTests(unittest.TestCase):
    def test_default_validates_without_writing_and_apply_rejects_changed_destination(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            canonical = root / 'canonical'
            package = root / 'package'
            canonical.mkdir()
            package.mkdir()
            (canonical / 'cache').mkdir()
            (canonical / 'cache/index.json').write_bytes(b'old')
            (package / 'cache').mkdir()
            (package / 'cache/index.json').write_bytes(b'new')
            spec = importlib.util.spec_from_file_location('apply_mounts', ROOT / 'scripts/apply_mount_model_assets.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            manifest = {'canonical_data': str(canonical), 'expected_index_inputs': {}, 'artifacts': [{
                'path': 'cache/index.json', 'source_sha256': module.sha(package / 'cache/index.json'),
                'bytes': 3, 'expected_before_sha256': module.sha(canonical / 'cache/index.json')}]}
            (package / 'manifest.json').write_text(json.dumps(manifest))
            module.validate(package)
            self.assertEqual((canonical / 'cache/index.json').read_bytes(), b'old')
            (canonical / 'cache/index.json').write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'before hash'):
                module.publish(package)
            self.assertEqual((canonical / 'cache/index.json').read_bytes(), b'changed')

    def test_apply_publishes_additive_asset_and_refuses_changed_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            canonical, package = root / 'canonical', root / 'package'
            canonical.mkdir()
            package.mkdir()
            (canonical / 'cache').mkdir()
            (package / 'mount.m2').write_bytes(b'authentic-test-payload')
            spec = importlib.util.spec_from_file_location('apply_mounts', ROOT / 'scripts/apply_mount_model_assets.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            manifest = {'canonical_data': str(canonical), 'expected_index_inputs': {}, 'artifacts': [{
                'path': 'mount.m2', 'source_sha256': module.sha(package / 'mount.m2'),
                'bytes': 22, 'expected_before_sha256': None}]}
            (package / 'manifest.json').write_text(json.dumps(manifest))
            (package / 'mount.m2').write_bytes(b'corrupted')
            with self.assertRaisesRegex(ValueError, 'source hash'):
                module.publish(package)
            self.assertFalse((canonical / 'mount.m2').exists())
            (package / 'mount.m2').write_bytes(b'authentic-test-payload')
            manifest['artifacts'][0]['bytes'] = len(b'authentic-test-payload')
            (package / 'manifest.json').write_text(json.dumps(manifest))
            module.publish(package)
            self.assertEqual((canonical / 'mount.m2').read_bytes(), b'authentic-test-payload')


if __name__ == '__main__':
    unittest.main()
