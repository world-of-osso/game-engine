"""Full-catalog roots use real fixture bytes and catalogs, not a census."""
import importlib.util
from pathlib import Path
import sqlite3
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
from asset_closure import Closure
from closure_seeds import seed_terrain, seed_catalogs


def chunk(tag, payload):
    return tag[::-1].encode() + struct.pack('<I', len(payload)) + payload


class FullRootsTests(unittest.TestCase):
    def test_wdt_only_map_reaches_its_authored_tile_without_named_root(self):
        with tempfile.TemporaryDirectory() as tmp:
            data = Path(tmp)
            (data / 'terrain').mkdir()
            maid = struct.pack('<8I', 20, 0, 0, 0, 0, 21, 0, 0)
            (data / 'terrain/10.wdt').write_bytes(chunk('MAID', maid))
            (data / 'terrain/20.adt').write_bytes(chunk('MVER', struct.pack('<I', 18)))
            graph = Closure(data, {10: 'world/maps/newmap/newmap.wdt'}, 'wow', 'fixture')
            seed_terrain(graph, {'maps': [{'id': 1, 'directory': 'newmap', 'tiles': 'all'}]})
            result = graph.run()
            self.assertEqual([(a['fdid'], a['type']) for a in result['assets']], [(10, 'wdt'), (20, 'adt'), (21, 'blp')])
            self.assertEqual(result['summary']['present'], 2)
            self.assertEqual(result['summary']['missing'], 1)

    def test_full_npcs_include_spawns_outside_named_tiles(self):
        with tempfile.TemporaryDirectory() as tmp:
            data = Path(tmp)
            db = sqlite3.connect(data / 'world.db')
            db.executescript('''
                CREATE TABLE content_creature(guid,id1,id2,id3,modelid,position_x,position_y,map);
                INSERT INTO content_creature VALUES(7,3,0,0,90,100,200,999);
                CREATE TABLE content_creature_template_model(CreatureID,CreatureDisplayID,Idx);
                INSERT INTO content_creature_template_model VALUES(3,91,0);
                CREATE TABLE content_item(ID,DisplayInfoID);
                CREATE TABLE spell_name(ID);
            ''')
            db.commit()
            db.close()
            graph = Closure(data, {}, 'wow', 'fixture')
            seed_catalogs(graph, data / 'world.db', {'maps': 'all', 'characters': 'all', 'npc_displays': 'all', 'items': 'all', 'spells': 'all'})
            self.assertEqual(graph.seeds['world_selection']['displays'], [90, 91])
            self.assertEqual([r['guid'] for r in graph.seeds['world_selection']['spawns']], [7])


if __name__ == '__main__':
    unittest.main()
