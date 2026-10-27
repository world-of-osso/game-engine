import tempfile
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from launcher_packs import build_packs


class PackTests(unittest.TestCase):
    def test_native_chunk_index_matches_golden(self):
        import json
        import subprocess
        import hashlib

        golden = json.loads((Path(__file__).parent / 'fixtures/launcher-packs-golden.json').read_text())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'data/textures').mkdir(parents=True)
            (root / 'small').write_bytes(b'launcher golden fixture\n')
            (root / 'data/textures/random').write_bytes(hashlib.shake_256(b'launcher pack golden').digest(6 * 1024 * 1024))
            (root / 'data/textures/max').write_bytes(bytes(5 * 1024 * 1024))
            binary = Path(__file__).resolve().parents[2] / 'target/release/launcher-pack-chunks'
            paths = [entry['path'] for entry in golden['manifest']['files']]
            result = subprocess.run([str(binary), str(root)], input=json.dumps(paths), text=True, capture_output=True, check=True)
            files = json.loads(result.stdout)
            for actual, expected in zip(files, golden['manifest']['files'], strict=True):
                self.assertEqual((actual['size'], actual['sha256']), (expected['size'], expected['sha256']))
                self.assertEqual([(chunk['size'], chunk['sha256']) for chunk in actual['chunks']], [(chunk['size'], chunk['sha256']) for chunk in expected['chunks']])

    def test_golden_pack_output(self):
        import hashlib
        import json

        golden = json.loads((Path(__file__).parent / 'fixtures/launcher-packs-golden.json').read_text())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            (source / 'data/textures').mkdir(parents=True)
            (source / 'small').write_bytes(b'launcher golden fixture\n')
            (source / 'data/textures/random').write_bytes(hashlib.shake_256(b'launcher pack golden').digest(6 * 1024 * 1024))
            (source / 'data/textures/max').write_bytes(bytes(5 * 1024 * 1024))
            output = root / 'output'
            build_packs(source, output)
            for name, key in [('chunk-index.json', 'index'), ('manifest-v2.json', 'manifest')]:
                self.assertEqual((output / name).read_bytes(), json.dumps(golden[key], sort_keys=True, separators=(',', ':')).encode())
            packs = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in (output / 'packs').iterdir()}
            self.assertEqual(packs, golden['packs'])

    def test_reproducible_and_retains_old_chunk_location(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            source.mkdir()
            (source / 'a').write_bytes(b'first')
            output = root / 'output'
            first = build_packs(source, output)
            self.assertEqual(first['format_version'], 1)
            first_chunk = first['files'][0]['chunks'][0]
            self.assertEqual(build_packs(source, output), first)
            self.assertEqual(build_packs(source, root / 'other-output'), first)
            (source / 'b').write_bytes(b'second')
            second = build_packs(source, output)
            self.assertEqual(second['files'][0]['chunks'][0], first_chunk)
            for entry in second['files']:
                payload = b''.join((output / 'packs' / chunk['pack']).read_bytes()[chunk['offset']:chunk['offset'] + chunk['size']] for chunk in entry['chunks'])
                self.assertEqual(payload, (source / entry['path']).read_bytes())

    def test_old_manifest_still_reconstructs_after_changed_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            source.mkdir()
            asset = source / 'a'
            asset.write_bytes(b'old bytes')
            output = root / 'output'
            old = build_packs(source, output)
            asset.write_bytes(b'new bytes')
            build_packs(source, output)
            chunk = old['files'][0]['chunks'][0]
            data = (output / 'packs' / chunk['pack']).read_bytes()
            self.assertEqual(data[chunk['offset']:chunk['offset'] + chunk['size']], b'old bytes')

    def test_manifest_selects_exact_files_and_checks_staged_digest(self):
        import hashlib
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            source.mkdir()
            (source / 'a').write_bytes(b'first')
            (source / 'unlisted').write_bytes(b'ignored')
            listed = {'files': [{'path': 'a', 'size': 5, 'sha256': hashlib.sha256(b'first').hexdigest()}]}
            result = build_packs(source, root / 'output', listed)
            self.assertEqual([entry['path'] for entry in result['files']], ['a'])
            (source / 'a').write_bytes(b'wrong')
            with self.assertRaisesRegex(ValueError, 'staged'):
                build_packs(source, root / 'output', listed)

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
