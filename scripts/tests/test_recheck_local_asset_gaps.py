import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from recheck_local_asset_gaps import extract_resolved


class GapRecheckTests(unittest.TestCase):
    def run_fixture(self, warning=False, existing=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root / 'data'
            output = root / 'output'
            data.mkdir()
            executable = root / 'extract.py'
            payload = b'BLP2 authentic fixture bytes'
            executable.write_text('#!/usr/bin/env python3\nimport pathlib,sys\na=sys.argv\np=pathlib.Path(a[a.index("-o")+1]);p.mkdir(exist_ok=True)\n(p/"42.blp").write_bytes(' + repr(payload) + ')\n' + ('print("FDID 42 missing TACT keys (zero-filled): AABBCCDDEEFF0011",file=sys.stderr)\n' if warning else ''))
            executable.chmod(0o755)
            target = data / 'textures/42.blp'
            if existing is not None:
                target.parent.mkdir()
                target.write_bytes(existing)
            inventory = [{'fdid': 42, 'type': 'blp', 'locations': ['textures/42.blp']}]
            identities = {42: {'product': 'wow', 'version': '12.1.0.69933', 'build_config': 'fixture', 'fdid': 42, 'content_key': hashlib.md5(payload).hexdigest(), 'encoding_key': '00' * 16, 'size': len(payload)}}
            rows = extract_resolved(inventory, identities, executable, data, output, {}, batch_size=8)
            contents = target.read_bytes() if target.exists() else None
            receipts = list((data / 'provenance').glob('*.jsonl')) if (data / 'provenance').exists() else []
            return rows, contents, [json.loads(line) for p in receipts for line in p.read_text().splitlines()]

    def test_real_process_publishes_authenticated_payload_and_receipt(self):
        rows, contents, receipts = self.run_fixture()
        self.assertEqual(rows[0]['status'], 'readable')
        self.assertEqual(rows[0]['created'], 1)
        self.assertEqual(contents, b'BLP2 authentic fixture bytes')
        self.assertEqual(receipts[0]['product'], 'wow')

    def test_missing_key_output_is_never_published(self):
        rows, contents, receipts = self.run_fixture(warning=True)
        self.assertEqual(rows[0]['status'], 'encrypted')
        self.assertIsNone(contents)
        self.assertEqual(receipts, [])

    def test_existing_different_file_is_never_overwritten(self):
        rows, contents, receipts = self.run_fixture(existing=b'other product')
        self.assertEqual(rows[0]['status'], 'conflict')
        self.assertEqual(contents, b'other product')
        self.assertEqual(receipts, [])


if __name__ == '__main__':
    unittest.main()
