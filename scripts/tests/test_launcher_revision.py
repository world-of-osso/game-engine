import gzip
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from launcher_packs import build_packs
import mapped_deploy


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

    def test_publisher_uploads_index_before_publishing_v3_root_and_keeps_v2(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / 'manifests'
            cache = root / 'packs'
            (output / 'manifest.json').parent.mkdir(parents=True)
            (output / 'manifest.json').write_text('{"files":[]}')
            (cache / 'packs').mkdir(parents=True)
            (cache / 'indexes').mkdir()
            (cache / 'manifest-v2.json').write_text('{}')
            (cache / 'manifest-v3.json').write_text(
                '{"version":3,"index":{"sha256":"' + 'a' * 64 + '"}}'
            )
            (cache / 'indexes' / ('a' * 64 + '.json.gz')).write_bytes(b'compressed')
            log = root / 'rclone.log'
            rclone = root / 'rclone'
            rclone.write_text(
                '#!/usr/bin/env python3\n'
                'import os, sys\n'
                'with open(os.environ["RCLONE_LOG"], "a") as stream:\n'
                ' stream.write("\\t".join(sys.argv[1:]) + "\\n")\n'
            )
            rclone.chmod(0o755)
            with patch.dict('os.environ', {'RCLONE_LOG': str(log)}):
                mapped_deploy.publish({}, {'files': []}, {}, output, cache, 'remote', str(rclone))
            commands = log.read_text().splitlines()
            index_upload = next(i for i, command in enumerate(commands) if '/indexes/' in command)
            root_upload = next(i for i, command in enumerate(commands) if 'remote/manifest-v3.json' in command)
            self.assertLess(index_upload, root_upload)
            self.assertIn('remote/manifest.json', commands[0])
            self.assertTrue(any('remote/manifest-v2.json' in command for command in commands))
            self.assertEqual(root_upload, len(commands) - 1)
