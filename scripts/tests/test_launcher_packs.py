import tempfile
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from launcher_packs import build_packs


class PackTests(unittest.TestCase):
    def test_reproducible_and_retains_old_chunk_location(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            source.mkdir()
            (source / 'a').write_bytes(b'first')
            output = root / 'output'
            first = build_packs(source, output)
            first_chunk = first['files'][0]['chunks'][0]
            self.assertEqual(build_packs(source, output), first)
            (source / 'b').write_bytes(b'second')
            second = build_packs(source, output)
            self.assertEqual(second['files'][0]['chunks'][0], first_chunk)
            for entry in second['files']:
                payload = b''.join((output / 'packs' / chunk['pack']).read_bytes()[chunk['offset']:chunk['offset'] + chunk['size']] for chunk in entry['chunks'])
                self.assertEqual(payload, (source / entry['path']).read_bytes())

    def test_large_file_insertion_reuses_content_defined_chunks(self):
        import random
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            source.mkdir()
            asset = source / 'asset'
            data = random.Random(17).randbytes(6 * 1024 * 1024)
            asset.write_bytes(data)
            output = root / 'output'
            first = build_packs(source, output)['files'][0]['chunks']
            asset.write_bytes(b'insertion' + data)
            second = build_packs(source, output)['files'][0]['chunks']
            old = {item['sha256']: item for item in first}
            shared = [item for item in second if item['sha256'] in old]
            self.assertGreaterEqual(len(shared), 1)
            self.assertTrue(all(item == old[item['sha256']] for item in shared))

    def test_corrupt_retained_pack_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            source.mkdir()
            (source / 'a').write_bytes(b'first')
            output = root / 'output'
            first = build_packs(source, output)
            (output / 'packs' / first['files'][0]['chunks'][0]['pack']).write_bytes(b'corrupt')
            with self.assertRaisesRegex(ValueError, 'corrupt'):
                build_packs(source, output)


if __name__ == '__main__':
    unittest.main()
