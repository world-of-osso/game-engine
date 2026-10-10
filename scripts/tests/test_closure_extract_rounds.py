from pathlib import Path
import struct
import sys
import subprocess
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from asset_closure import Closure
from closure_extract_rounds import fixed_point


class FixedPointTests(unittest.TestCase):
    def test_missing_expandable_descendants_are_extracted_in_later_rounds(self):
        with tempfile.TemporaryDirectory() as directory:
            data = Path(directory)
            paths = {1: 'world/map.wdt', 2: 'world/tile.adt', 3: 'texture.blp'}
            def chunk(tag, payload):
                return tag + struct.pack('<I', len(payload)) + payload
            payloads = {1: chunk(b'DIAM', struct.pack('<8I', 2, 0, 0, 0, 0, 0, 0, 0)),
                        2: chunk(b'DIDM', struct.pack('<I', 3)), 3: b'BLP2fixture'}
            def traverse():
                graph = Closure(data, paths, 'wow', 'fixture')
                graph.add(1, 'wdt', 'map root')
                return graph.run()
            def extract(manifest, round_number):
                files = 0
                for asset in manifest['assets']:
                    if not asset['present']:
                        target = data / asset['locations'][0]
                        target.parent.mkdir(parents=True, exist_ok=True)
                        target.write_bytes(payloads[asset['fdid']])
                        files += 1
                return {'files': files}
            rounds = []
            result = fixed_point(traverse(), extract, traverse,
                                 lambda m: m['summary']['missing'], rounds.append)
            self.assertEqual(result['summary']['present'], 3)
            self.assertEqual(result['summary']['missing'], 0)
            self.assertEqual([r['before'] for r in rounds], [1, 1, 1])
            self.assertEqual([r['after'] for r in rounds], [1, 1, 0])

    def test_graph_replacement_completes_with_room_for_one_manifest(self):
        code = '''
from pathlib import Path
import resource
import os
from closure_extract_rounds import fixed_point
baseline = int(Path('/proc/self/statm').read_text().split()[0]) * os.sysconf('SC_PAGE_SIZE')
limit = baseline + 40 * 1024 * 1024
resource.setrlimit(resource.RLIMIT_AS, (limit, limit))
result = fixed_point({'payload': b'x' * (24 * 1024 * 1024)},
                     lambda m, n: {'files': 1},
                     lambda: {'done': True, 'payload': b'y' * (24 * 1024 * 1024)},
                     lambda m: 0 if m.get('done') else 1, lambda r: None)
print(len(result['payload']))
'''
        result = subprocess.run([sys.executable, '-c', code],
                                cwd=Path(__file__).resolve().parents[1],
                                text=True, capture_output=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), str(24 * 1024 * 1024))

    def test_indexed_failure_stops_without_claiming_missing_bytes_present(self):
        manifest = {'summary': {'missing': 1}, 'assets': [{'fdid': 8, 'present': False}]}
        rounds = []
        result = fixed_point(manifest, lambda m, n: {'files': 0}, lambda: manifest,
                             lambda m: m['summary']['missing'], rounds.append)
        self.assertEqual(result['summary']['missing'], 1)
        self.assertEqual(len(rounds), 1)


if __name__ == '__main__':
    unittest.main()
