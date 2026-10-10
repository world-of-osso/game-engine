import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from closure_extract import publish, read_receipts, summarize_receipts


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.data = self.root / 'data'
        self.data.mkdir()
        self.stage = self.root / 'file'
        self.payload = b'MD21 authentic fixture bytes'
        self.stage.write_bytes(self.payload)
        self.index = self.data / 'provenance/closure-extract.jsonl'
        self.identity = {'product': 'wow', 'actual_build': '12.1.0.69933',
                         'build_config': 'a' * 32, 'fdid': 17,
                         'content_key': hashlib.md5(self.payload).hexdigest(),
                         'encoding_key': 'b' * 32, 'size': len(self.payload), 'type': 'm2'}

    def test_publish_runtime_aliases_and_resume_receipts(self):
        paths = ['models/17.m2', 'models/character/test/test.m2']
        result = publish(self.stage, self.data, paths, self.identity, self.index)
        self.assertEqual(result['created'], 2)
        self.assertEqual((self.data / paths[1]).read_bytes(), self.payload)
        receipts = read_receipts(self.index)
        self.assertEqual(len(receipts), 2)
        self.assertEqual(receipts[paths[0]]['sha256'], hashlib.sha256(self.payload).hexdigest())
        before = self.index.read_bytes()
        result = publish(self.stage, self.data, paths, self.identity, self.index)
        self.assertEqual(result['created'], 0)
        self.assertEqual(self.index.read_bytes(), before)
        self.assertEqual(summarize_receipts(receipts.values())['m2']['files'], 2)

    def test_existing_different_bytes_are_never_overwritten_or_receipted(self):
        target = self.data / 'models/17.m2'
        target.parent.mkdir()
        target.write_bytes(b'older product')
        result = publish(self.stage, self.data, ['models/17.m2'], self.identity, self.index)
        self.assertEqual(target.read_bytes(), b'older product')
        self.assertEqual(result['conflicts'], ['models/17.m2'])
        self.assertEqual(read_receipts(self.index), {})

    def test_zero_filled_or_wrong_size_output_never_published(self):
        self.stage.write_bytes(b'\0' * len(self.payload))
        with self.assertRaisesRegex(ValueError, 'content key'):
            publish(self.stage, self.data, ['models/17.m2'], self.identity, self.index)
        self.assertFalse((self.data / 'models/17.m2').exists())
        self.stage.write_bytes(b'short')
        with self.assertRaisesRegex(ValueError, 'size'):
            publish(self.stage, self.data, ['models/17.m2'], self.identity, self.index)

    def test_restart_after_publication_before_receipt_recovers_verified_file(self):
        target = self.data / 'models/17.m2'
        target.parent.mkdir()
        target.write_bytes(self.payload)
        result = publish(self.stage, self.data, ['models/17.m2'], self.identity, self.index)
        self.assertEqual(result['created'], 0)
        self.assertEqual(read_receipts(self.index)['models/17.m2']['disposition'], 'verified_existing')

    def test_path_escape_refused(self):
        with self.assertRaisesRegex(ValueError, 'path'):
            publish(self.stage, self.data, ['../escape.m2'], self.identity, self.index)
        self.assertFalse((self.root / 'escape.m2').exists())

    def test_changed_receipted_file_is_not_trusted(self):
        publish(self.stage, self.data, ['models/17.m2'], self.identity, self.index)
        (self.data / 'models/17.m2').write_bytes(b'changed')
        result = publish(self.stage, self.data, ['models/17.m2'], self.identity, self.index)
        self.assertEqual(result['conflicts'], ['models/17.m2'])


if __name__ == '__main__':
    unittest.main()
