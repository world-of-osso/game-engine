"""Path identities follow the runtime's persisted-local / SQLite import rule."""
from contextlib import closing
from pathlib import Path
import sqlite3
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
from asset_closure import Closure, read_listfile
from test_asset_closure import chunk, ints, model


class PathTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.data = Path(self.tmp.name)

    def put(self, relative, payload):
        target = self.data / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(payload)
        return target

    def test_named_reference_uses_last_imported_identity_not_largest_fdid(self):
        graph = Closure(self.data, {9: 'world/test.m2', 2: 'world/test.m2'}, 'wow', 'fixture')
        self.put('models/2.m2', model())
        graph.named('WORLD\\TEST.M2', 'm2', 'named placement', None)
        result = graph.run()
        self.assertEqual([a['fdid'] for a in result['assets']], [2])
        self.assertFalse(result['unresolved'])
        self.assertTrue(any(r['code'] == 'ambiguous_path' and '2' in r['evidence'] for r in result['resolved']))

    def test_listfile_bindings_match_runtime_sqlite_unique_path_import(self):
        rows = [(7, 'world/a.m2'), (3, 'world/a.m2'), (3, 'world/b.m2'), (9, 'world/c.m2'), (2, 'world/c.m2')]
        path = self.put('community-listfile.csv', ''.join(f'{fdid};{name}\n' for fdid, name in rows).encode())
        with closing(sqlite3.connect(':memory:')) as db:
            db.executescript('CREATE TABLE listfile_entries(fdid INTEGER PRIMARY KEY,path TEXT NOT NULL,lower_path TEXT NOT NULL); CREATE UNIQUE INDEX paths ON listfile_entries(lower_path);')
            for fdid, name in rows:
                db.execute('INSERT OR REPLACE INTO listfile_entries VALUES (?,?,?)', (fdid, name, name.lower()))
            expected = {name: fdid for fdid, name in db.execute('SELECT fdid,lower_path FROM listfile_entries')}
        # This exercises both kinds of displacement: duplicate path, then repeated FDID.
        paths, bindings = read_listfile(path)
        self.assertEqual(bindings, expected)
        self.assertEqual(paths[7], 'world/a.m2')  # retain candidate census, not a false path binding

    def test_runtime_ascii_casefold_does_not_merge_non_ascii_names(self):
        path = self.put('community-listfile.csv', '1;world/Éclair.m2\n9;world/éclair.m2\n'.encode())
        paths, bindings = read_listfile(path)
        self.assertEqual(bindings, {'world/Éclair.m2': 1, 'world/éclair.m2': 9})
        self.put('models/1.m2', model())
        graph = Closure(self.data, paths, 'wow', 'fixture', runtime_paths=bindings)
        graph.named('WORLD/Éclair.M2', 'm2', 'case-sensitive non-ASCII name', None)
        self.assertEqual([a['fdid'] for a in graph.run()['assets']], [1])

    def test_persisted_local_row_wins_and_its_input_is_fingerprinted(self):
        cache = self.data / 'local.sqlite'
        with closing(sqlite3.connect(cache)) as db:
            db.execute('CREATE TABLE local_listfile_entries(fdid INTEGER PRIMARY KEY,path TEXT,lower_path TEXT UNIQUE)')
            db.execute('INSERT INTO local_listfile_entries VALUES(17,?,?)', ('world/test.m2', 'world/test.m2'))
            db.commit()
        graph = Closure(self.data, {9: 'world/test.m2', 2: 'world/test.m2'}, 'wow', 'fixture', path_cache=cache)
        self.put('models/17.m2', model())
        graph.named('world/test.m2', 'm2', 'named placement', None)
        result = graph.run()
        self.assertEqual([a['fdid'] for a in result['assets']], [17])
        self.assertFalse(result['unresolved'])
        self.assertEqual([row['path'] for row in result['inputs']], ['local.sqlite'])
        self.assertTrue(any('local-listfile' in r['evidence'] for r in result['resolved']))

    def test_declared_map_wdt_fdid_wins_over_named_duplicates(self):
        from closure_seeds import seed_terrain
        self.put('terrain/10.wdt', chunk('MVER', ints(18), True))
        graph = Closure(self.data, {10: 'world/maps/fixture/fixture.wdt', 9: 'world/maps/fixture/fixture.wdt'}, 'wow', 'fixture')
        seed_terrain(graph, {'maps': [{'id': 1, 'directory': 'fixture', 'tiles': 'all', 'wdt_fdid': 10}]})
        result = graph.run()
        self.assertEqual([a['fdid'] for a in result['assets']], [10])
        self.assertFalse(result['unresolved'])

    def test_explicit_adt_fdid_never_chooses_a_named_duplicate(self):
        row = bytearray(36)
        struct.pack_into('<I', row, 0, 4)
        struct.pack_into('<H', row, 34, 0x40)
        self.put('terrain/1.adt', chunk('MDDF', row, True))
        self.put('models/4.m2', model())
        graph = Closure(self.data, {4: 'world/test.m2', 9: 'world/test.m2'}, 'wow', 'fixture')
        graph.add(1, 'adt', 'test')
        result = graph.run()
        self.assertEqual([a['fdid'] for a in result['assets']], [1, 4])
        self.assertFalse(result['unresolved'])

    def test_m2_txid_and_adt_mdid_override_stale_named_texture_arrays(self):
        name = b'textures/old.blp\0'
        raw = bytearray(0x148 + len(name))
        raw[:4] = b'MD20'
        struct.pack_into('<II', raw, 0x50, 1, 0x138)
        struct.pack_into('<IIII', raw, 0x138, 0, 0, len(name), 0x148)
        raw[0x148:] = name
        self.put('models/2.m2', chunk('MD21', raw) + chunk('TXID', ints(17)))
        self.put('textures/17.blp', b'BLP2current')
        paths = {8: 'textures/old.blp', 9: 'textures/old.blp'}
        graph = Closure(self.data, paths, 'wow', 'fixture')
        graph.add(2, 'm2', 'test')
        result = graph.run()
        self.assertEqual([a['fdid'] for a in result['assets']], [2, 17])
        self.assertFalse(result['unresolved'])
        self.put('terrain/1.adt', chunk('MDID', ints(17), True) + chunk('MTEX', name, True))
        graph = Closure(self.data, paths, 'wow', 'fixture')
        graph.add(1, 'adt', 'test')
        result = graph.run()
        self.assertEqual([a['fdid'] for a in result['assets']], [1, 17])
        self.assertFalse(result['unresolved'])

    def test_displaced_path_without_runtime_binding_remains_a_visible_gap(self):
        graph = Closure(self.data, {7: 'world/a.m2', 3: 'world/b.m2'}, 'wow', 'fixture', runtime_paths={'world/b.m2': 3})
        graph.named('world/a.m2', 'm2', 'displaced placement', None)
        result = graph.run()
        self.assertEqual(result['assets'], [])
        self.assertTrue(any(row['code'] == 'unmapped_path' for row in result['unresolved']))


if __name__ == '__main__':
    unittest.main()
