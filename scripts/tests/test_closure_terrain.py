"""Terrain closure: real chunk streams and pinned metadata joins."""
from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
from asset_closure import Closure
from test_asset_closure import chunk, ints, model


class TerrainTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.data = Path(self.tmp.name)
        self.graph = Closure(self.data, {}, 'wow', 'fixture')

    def put(self, path, payload):
        target = self.data / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(payload)

    def table(self, name, text):
        self.put(f'db2/fixture/{name}.csv', text.encode())

    def run_adt(self, payload):
        self.graph = Closure(self.data, {}, 'wow', 'fixture')
        self.put('terrain/1.adt', payload)
        self.graph.add(1, 'adt', 'test')
        return self.graph.run()

    def test_all_mcnk_layers_join_ground_doodads_and_transitive_textures(self):
        self.table('GroundEffectTexture', 'ID,DoodadID_0,DoodadID_1,DoodadID_2,DoodadID_3\n10,20,0,0,0\n11,21,0,0,0\n')
        self.table('GroundEffectDoodad', 'ID,ModelFileID\n20,2\n21,3\n')
        self.put('models/2.m2', model(chunk('TXID', ints(8))))
        self.put('models/3.m2', model(chunk('TXID', ints(9))))
        self.put('textures/8.blp', b'BLP2a')
        self.put('textures/9.blp', b'BLP2b')
        payload = b''.join(chunk('MCNK', chunk('MCLY', ints(0, 0, 0, effect), True), True) for effect in [10, 11])
        result = self.run_adt(payload)
        self.assertEqual({a['fdid'] for a in result['assets']}, {1, 2, 3, 8, 9})
        self.assertFalse(result['unresolved'])

    def test_root_geometry_and_empty_object_mcnks_have_no_external_edges(self):
        result = self.run_adt(chunk('MHDR', bytes(64), True) + chunk('MCNK', bytes(128) + chunk('MCVT', bytes(580), True), True) + chunk('MCNK', b'', True))
        self.assertFalse(result['unresolved'])
        self.assertTrue(any(r['code'] == 'terrain_auxiliary_edges' and r['status'] == 'not_needed' for r in result['resolved']))

    def test_liquid_objects_join_texture_fdids_and_shader_global_blob(self):
        self.table('LiquidObject', 'ID,LiquidTypeID\n50,3\n')
        self.table('LiquidType', 'ID,MaterialID\n3,2\n')
        self.table('LiquidMaterial', 'ID,LVF\n2,0\n')
        self.table('LiquidTypeXTexture', 'ID,LiquidTypeID,FileDataID,OrderIndex,Type\n1,3,8,0,0\n2,3,0,1,0\n')
        water = bytearray(256 * 12)
        struct.pack_into('<III', water, 0, len(water), 1, 0)
        water += struct.pack('<HHffBBBBII', 1, 50, 0, 0, 0, 0, 1, 1, 0, 0)
        result = self.run_adt(chunk('MH2O', water, True))
        self.assertEqual({(a['fdid'], a['type']) for a in result['assets']}, {(1, 'adt'), (8, 'blp'), (768431, 'blob')})
        blob = next(a for a in result['assets'] if a['type'] == 'blob')
        self.assertEqual(blob['locations'], ['textures/768431.blob'])
        self.assertFalse(any(r['code'] == 'terrain_auxiliary_edges' for r in result['unresolved']))

    def test_empty_liquid_is_not_needed_but_missing_join_stays_unresolved(self):
        result = self.run_adt(chunk('MH2O', bytes(256 * 12), True))
        self.assertFalse(result['unresolved'])
        self.assertTrue(any(r['status'] == 'not_needed' for r in result['resolved']))
        self.table('GroundEffectTexture', 'ID,DoodadID_0\n')
        self.table('GroundEffectDoodad', 'ID,ModelFileID\n')
        result = self.run_adt(chunk('MCNK', chunk('MCLY', ints(0, 0, 0, 99), True), True))
        self.assertTrue(any(r['code'] == 'terrain_auxiliary_edges' and '99' in r['reason'] for r in result['unresolved']))

    def test_mdid_mhid_are_ids_mtxf_is_flags_not_a_texture(self):
        result = self.run_adt(chunk('MDID', ints(8), True) + chunk('MHID', ints(9), True) + chunk('MTXF', ints(0x10), True))
        self.assertEqual({a['fdid'] for a in result['assets']}, {1, 8, 9})
        self.assertTrue(any('MTXF' in r['evidence'] for r in result['resolved']))

    def test_malformed_liquid_offsets_and_unknown_nested_chunks_are_visible(self):
        water = bytearray(256 * 12)
        struct.pack_into('<III', water, 0, len(water) + 20, 1, 0)
        result = self.run_adt(chunk('MH2O', water, True))
        self.assertTrue(any(r['code'] == 'parse_error' for r in result['unresolved']))
        result = self.run_adt(chunk('MCNK', chunk('ZZZZ', ints(8), True), True))
        self.assertTrue(any(r['code'] == 'terrain_auxiliary_edges' for r in result['unresolved']))

    def test_wdt_maid_preserves_all_eight_explicit_id_columns(self):
        self.put('terrain/1.wdt', chunk('MAID', ints(2, 3, 4, 5, 6, 7, 8, 9), True))
        self.graph.add(1, 'wdt', 'test')
        result = self.graph.run()
        self.assertEqual({(a['fdid'], a['type']) for a in result['assets']}, {(1, 'wdt'), (2, 'adt'), (3, 'adt'), (4, 'adt'), (5, 'adt'), (6, 'adt'), (7, 'blp'), (8, 'blp'), (9, 'blp')})


if __name__ == '__main__':
    unittest.main()
