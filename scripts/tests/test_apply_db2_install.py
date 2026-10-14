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

    def test_all_backups_exist_before_first_payload_write(self):
        from unittest.mock import patch
        import apply_db2_install as publication
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = []
            for name in ['a', 'b']:
                source, destination = root / f'new-{name}', root / name
                source.write_text('new-' + name)
                destination.write_text('old-' + name)
                files.append({'source': str(source), 'destination': str(destination), 'before': digest(destination), 'after': digest(source)})
            manifest = root / 'manifest.json'
            manifest.write_text(json.dumps({'files': files, 'links': []}))
            copy = publication.shutil.copy2
            def fail_first_payload(source, destination, *args, **kwargs):
                if Path(source) == root / 'new-a':
                    raise OSError('payload write failure')
                return copy(source, destination, *args, **kwargs)
            with patch.object(publication.shutil, 'copy2', fail_first_payload), self.assertRaisesRegex(OSError, 'payload write failure'):
                apply(manifest, root / 'backup')
            self.assertEqual({path.read_text() for path in (root / 'backup/files').iterdir()}, {'old-a', 'old-b'})
            self.assertEqual((root / 'a').read_text(), 'old-a')
            self.assertEqual((root / 'b').read_text(), 'old-b')

    def test_publication_failure_rolls_back_already_published_files(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = []
            for name in ['a', 'b']:
                source, destination = root / f'new-{name}', root / name
                source.write_text('new-' + name)
                destination.write_text('old-' + name)
                files.append({'source': str(source), 'destination': str(destination), 'before': digest(destination), 'after': digest(source)})
            manifest = root / 'manifest.json'
            manifest.write_text(json.dumps({'files': files, 'links': []}))
            replace = Path.replace
            def fail_second_publish(source, destination):
                if Path(destination) == root / 'b':
                    raise OSError('second publication failure')
                return replace(source, destination)
            with patch.object(Path, 'replace', fail_second_publish), self.assertRaisesRegex(OSError, 'second publication failure'):
                apply(manifest, root / 'backup')
            self.assertEqual((root / 'a').read_text(), 'old-a')
            self.assertEqual((root / 'b').read_text(), 'old-b')

    def test_repeated_current_hardlink_apply_is_a_no_op(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, destination = root / 'source', root / 'published'
            source.write_bytes(b'authenticated bytes')
            manifest = root / 'manifest.json'
            manifest.write_text(json.dumps({'files': [{'source': str(source), 'destination': str(destination), 'before': None, 'after': digest(source), 'hardlink': True}], 'links': []}))
            apply(manifest, root / 'backup')
            expected_files = set(root.iterdir())
            inode = destination.stat().st_ino
            apply(manifest, root / 'backup')
            self.assertEqual(destination.stat().st_ino, inode)
            self.assertEqual(destination.read_bytes(), b'authenticated bytes')
            self.assertEqual(set(root.iterdir()), expected_files)

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
