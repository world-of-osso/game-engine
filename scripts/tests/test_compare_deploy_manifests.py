import hashlib
import importlib.util
import struct
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('compare', Path(__file__).parents[1] / 'compare_deploy_manifests.py')
compare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compare)


def uid_cache(rows):
    result = bytearray(struct.pack('<I', len(rows)))
    for uid, path in rows:
        raw = path.encode()
        result.extend(struct.pack('<QI', uid, len(raw)))
        result.extend(raw)
    return bytes(result)


def entry(path, data):
    return {'path': path, 'size': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


class CompareManifestsTest(unittest.TestCase):
    def test_sqlite_bytes_may_differ_only_when_schema_and_rows_match(self):
        import sqlite3
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            old_root, new_root = root / 'old', root / 'new'
            for base in (old_root, new_root):
                (base / 'data/cache').mkdir(parents=True)
                (base / 'godot/.godot').mkdir(parents=True)
                (base / compare.UID_PATH).write_bytes(uid_cache([(1, 'res://one')]))
                with sqlite3.connect(base / 'data/cache/test.sqlite') as db:
                    db.execute('CREATE TABLE names (name TEXT)')
                    db.execute("INSERT INTO names VALUES ('one')")
            with sqlite3.connect(new_root / 'data/cache/test.sqlite') as db:
                db.execute('PRAGMA user_version=1')
            def manifest(base):
                return {'version': 'today', 'files': [
                    entry(compare.UID_PATH, (base / compare.UID_PATH).read_bytes()),
                    entry('data/cache/test.sqlite', (base / 'data/cache/test.sqlite').read_bytes()),
                ]}
            first, second = manifest(old_root), manifest(new_root)
            self.assertNotEqual(first['files'][1]['sha256'], second['files'][1]['sha256'])
            compare.compare_manifests(first, second, old_root / compare.UID_PATH,
                                      new_root / compare.UID_PATH, old_root, new_root)
            with sqlite3.connect(new_root / 'data/cache/test.sqlite') as db:
                db.execute("INSERT INTO names VALUES ('two')")
            with self.assertRaisesRegex(ValueError, 'test.sqlite'):
                compare.compare_manifests(first, manifest(new_root), old_root / compare.UID_PATH,
                                          new_root / compare.UID_PATH, old_root, new_root)

    def test_uid_record_order_is_ignored_but_all_other_entries_are_exact(self):
        # Two cold imports of the same old bundle produced different cache hashes.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            old = root / 'old'
            new = root / 'new'
            old.write_bytes(uid_cache([(1, 'res://one'), (2, 'res://two')]))
            new.write_bytes(uid_cache([(2, 'res://two'), (1, 'res://one')]))
            name = 'godot/.godot/uid_cache.bin'
            first = {'version': 'today', 'files': [entry(name, old.read_bytes()), entry('a', b'same')]}
            second = {'version': 'today', 'files': [entry(name, new.read_bytes()), entry('a', b'same')]}
            compare.compare_manifests(first, second, old, new)
            second['files'][1] = entry('a', b'wrong')
            with self.assertRaisesRegex(ValueError, 'a'):
                compare.compare_manifests(first, second, old, new)
            second['files'][1] = entry('a', b'same')
            new.write_bytes(uid_cache([(2, 'res://two'), (3, 'res://one')]))
            second['files'][0] = entry(name, new.read_bytes())
            with self.assertRaisesRegex(ValueError, 'uid_cache'):
                compare.compare_manifests(first, second, old, new)


if __name__ == '__main__':
    unittest.main()
