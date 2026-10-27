import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('verified_upload', Path(__file__).parents[1] / 'verified_upload.py')
upload = importlib.util.module_from_spec(spec)
spec.loader.exec_module(upload)


class VerifiedUploadTest(unittest.TestCase):
    def test_only_matching_stream_is_promoted_to_the_published_path(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rclone = root / 'rclone'
            rclone.write_text('''#!/usr/bin/env python3
import pathlib, shutil, sys
args = sys.argv[1:]
if args[0] == "rcat":
    path = pathlib.Path(args[1]); path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(sys.stdin.buffer.read())
elif args[0] == "moveto":
    target = pathlib.Path(args[2]); target.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(args[1], target)
elif args[0] == "deletefile":
    pathlib.Path(args[1]).unlink(missing_ok=True)
''')
            rclone.chmod(0o755)
            source = root / 'catalog'
            source.write_bytes(b'expected')
            entry = {'path': 'data/cache/catalog.sqlite', 'size': 8,
                     'sha256': hashlib.sha256(b'expected').hexdigest()}
            destination = root / 'remote'
            upload.upload_file(source, str(destination), entry, str(rclone))
            published = destination / entry['path']
            self.assertEqual(published.read_bytes(), b'expected')
            source.write_bytes(b'mutated!')
            with self.assertRaisesRegex(ValueError, 'data/cache/catalog.sqlite'):
                upload.upload_file(source, str(destination), entry, str(rclone))
            self.assertEqual(published.read_bytes(), b'expected')
            self.assertEqual(list((destination / '.deploy-temporary').glob('*')), [])


if __name__ == '__main__':
    unittest.main()
