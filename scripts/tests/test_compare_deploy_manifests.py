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
