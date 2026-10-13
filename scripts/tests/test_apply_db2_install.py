import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from apply_db2_install import apply


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class ApplyTest(unittest.TestCase):
    def test_backup_and_fingerprint_diff_precede_publication(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            old = root / 'engine/data/Table.csv'
            old.parent.mkdir(parents=True)
            old.write_text('ID,Name\n1,Before\n')
            new = root / 'package/Table.csv'
            new.parent.mkdir()
            new.write_text('ID,Name\n1,After\n')
            manifest = root / 'manifest.json'
            manifest.write_text(json.dumps({'files': [{'source': str(new), 'destination': str(old),
                                                      'before': digest(old), 'after': digest(new)}],
                                             'links': [{'path': str(root / 'engine/data/db2/active'),
                                                        'before': None, 'after': '12.1.5.70340'}]}))
            backup = root / 'backup'
            apply(manifest, backup)
            self.assertEqual(old.read_text(), new.read_text())
            self.assertEqual(next((backup / 'files').iterdir()).read_text(), 'ID,Name\n1,Before\n')
            self.assertEqual((root / 'engine/data/db2/active').readlink(), Path('12.1.5.70340'))
            diff = json.loads((backup / 'fingerprint-diff.json').read_text())
            self.assertTrue(diff['files'][0]['matches'])

    def test_source_or_destination_drift_blocks_every_write(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            destination = root / 'current.csv'
            source = root / 'new.csv'
            destination.write_text('original')
            source.write_text('new')
            manifest = root / 'manifest.json'
            manifest.write_text(json.dumps({'files': [{'source': str(source), 'destination': str(destination),
                                                      'before': 'stale-fingerprint', 'after': digest(source)}], 'links': []}))
            with self.assertRaisesRegex(ValueError, 'fingerprint drift'):
                apply(manifest, root / 'backup')
            self.assertEqual(destination.read_text(), 'original')


if __name__ == '__main__':
    unittest.main()
