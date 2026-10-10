import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from closure_extract_batch import extract_batch, pending_assets


class BatchTests(unittest.TestCase):
    def test_native_process_failure_and_receipted_resume(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root / 'data'
            data.mkdir()
            output = root / 'output'
            output.mkdir()
            fixture = root / 'extractor.py'
            fixture.write_text('''#!/usr/bin/env python3
import pathlib,sys
out=pathlib.Path(sys.argv[-1]);out.mkdir(parents=True,exist_ok=True)
print('CASC resolver initialized using wow cache fixture-cache',file=sys.stderr)
for fdid in sys.argv[1:-2]:
 if fdid == '18': print('Failed FDID 18: missing TACT key',file=sys.stderr)
 else: (out / (fdid + '.dat')).write_bytes(b'authentic')
sys.exit(1)
''')
            fixture.chmod(0o755)
            identity = {'product': 'wow', 'actual_build': '12.1.0.69933',
                        'build_config': 'a' * 32, 'content_key': hashlib.md5(b'authentic').hexdigest(),
                        'encoding_key': 'b' * 32, 'size': 9, 'type': 'm2'}
            assets = [{'fdid': n, 'type': 'm2', 'locations': [f'models/{n}.m2']} for n in [17, 18]]
            identities = {n: dict(identity, fdid=n) for n in [17, 18]}
            receipts = {}
            result = extract_batch(assets, identities, fixture, data, output, 1, 'fixture-cache', receipts, {})
            self.assertEqual(result['files'], 1)
            self.assertEqual((data / 'models/17.m2').read_bytes(), b'authentic')
            self.assertIn('missing TACT key', result['failures'][18])
            pending = pending_assets(assets, data, receipts, {18})
            self.assertEqual(pending, [])
            (data / 'models/17.m2').write_bytes(b'corrupt')
            self.assertEqual([a['fdid'] for a in pending_assets(assets, data, receipts, {18})], [17])

    def test_receipts_do_not_hide_new_runtime_alias(self):
        with tempfile.TemporaryDirectory() as directory:
            data = Path(directory)
            (data / '17.m2').write_bytes(b'authentic')
            asset = {'fdid': 17, 'type': 'm2', 'locations': ['17.m2', 'new/17.m2']}
            receipt = {'sha256': hashlib.sha256(b'authentic').hexdigest()}
            self.assertEqual(pending_assets([asset], data, {'17.m2': receipt}, set()), [asset])


if __name__ == '__main__':
    unittest.main()
