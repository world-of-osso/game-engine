"""Targeted real-asset tests; extract the three FDIDs before running.

BLS_FIXTURES overrides data/bls-research. Missing retail assets fail explicitly.
"""
import os
from pathlib import Path
import struct
import tempfile
import unittest

import bls_dump as bls

FIXTURES = Path(os.environ.get('BLS_FIXTURES', bls.ROOT / 'data/bls-research'))


class RetailBundles(unittest.TestCase):
    def read_bundle(self, fdid):
        return bls.parse_bundle((FIXTURES / f'{fdid}.bls').read_bytes())

    def test_water_counts_and_every_program(self):
        for fdid, count, blocks in [(2977281, 6, 3), (2977223, 54, 41), (2977134, 4, 2)]:
            with self.subTest(fdid=fdid):
                header, entries, raw = self.read_bundle(fdid)
                self.assertEqual(header['permutation_count'], count)
                self.assertEqual(header['block_count'], blocks)
                self.assertEqual(header['profile'], 'DX50')
                for entry in entries:
                    program, wrapper = bls.split_program(raw[entry['offset']:entry['offset'] + entry['size']])
                    description, shader = bls.describe_program(program)
                    self.assertTrue(shader)
                    self.assertEqual(description['kind'], 'SM5')
                    self.assertTrue(description['inputs'])
                    self.assertEqual(description['outputs'][0]['semantic'], 'SV_Target')
                    self.assertNotIn('RDEF', description['chunks'])
                    self.assertEqual(wrapper['dxbc_offset'], 96)

    def test_fel_resource_slots_match_decompilation(self):
        _, entries, raw = self.read_bundle(2977134)
        textures = []
        for entry in entries:
            program, _ = bls.split_program(raw[entry['offset']:entry['offset'] + entry['size']])
            description, _ = bls.describe_program(program)
            textures.append(description['slots']['textures'])
        self.assertEqual(textures, [[0, 1, 16], [0, 1, 14, 15, 16],
                                    [0, 1, 16], [0, 1, 14, 15, 16]])

    def test_real_fel_assembly_and_written_files(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            report = bls.write_dump(FIXTURES / '2977134.bls', output,
                                    bls.ROOT / 'target/bls-disasm/debug/bls-disasm')
            self.assertEqual(len(report['programs']), 4)
            self.assertEqual(len(list(output.glob('*.dxbc'))), 4)
            assembly = (output / 'permutation-0001.asm').read_text()
            self.assertIn('ps_5_0', assembly)
            self.assertIn('t16', assembly)
            self.assertIn('t14', assembly)
            self.assertIn('sample', assembly)
            self.assertIn('ret', assembly)

    def test_truncated_and_corrupt_bundle_rejected(self):
        data = (FIXTURES / '2977281.bls').read_bytes()
        for broken in [data[:39], data[:-1], data[:204] + bytes(len(data) - 204)]:
            with self.subTest(length=len(broken)):
                with self.assertRaises((ValueError, struct.error)):
                    bls.parse_bundle(broken)


if __name__ == '__main__':
    unittest.main()
