"""Pinned local journal cache acceptance; no guessed encounter or item identities."""
from contextlib import closing
import importlib.util
import os
from pathlib import Path
import sqlite3
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'adventure_guide_import.py'
SPEC = importlib.util.spec_from_file_location('adventure_guide_import', SCRIPT)


class JournalImportTests(unittest.TestCase):
    def test_local_journal_retains_hierarchy_and_loot_difficulty(self):
        module = importlib.util.module_from_spec(SPEC)
        SPEC.loader.exec_module(module)
        source = Path(os.environ['ADVENTURE_GUIDE_SOURCE'])
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'journal.sqlite'
            module.import_cache(source / 'db2', Path(os.environ['ADVENTURE_GUIDE_DEFINITIONS']), output, Path(os.environ['ADVENTURE_GUIDE_ITEMS']), Path(os.environ['ADVENTURE_GUIDE_WORLD']))
            with closing(sqlite3.connect(output)) as conn:
                encounter = conn.execute('SELECT ID,Name_lang,JournalInstanceID,FirstSectionID FROM JournalEncounter WHERE ID=663').fetchone()
                self.assertEqual(encounter[1], 'Jandice Barov')
                self.assertEqual(conn.execute('SELECT Name_lang FROM JournalInstance WHERE ID=?', (encounter[2],)).fetchone()[0], 'Scholomance')
                section = conn.execute('SELECT JournalEncounterID FROM JournalEncounterSection WHERE ID=?', (encounter[3],)).fetchone()
                self.assertEqual(section[0], 663)
                self.assertGreater(conn.execute('SELECT COUNT(*) FROM JournalEncounterItem WHERE JournalEncounterID=663').fetchone()[0], 0)
                self.assertGreater(conn.execute('SELECT COUNT(*) FROM JournalItemXDifficulty d JOIN JournalEncounterItem i ON i.ID=d.JournalEncounterItemID').fetchone()[0], 1000)
                self.assertEqual(conn.execute('SELECT build FROM source').fetchone()[0], module.source_identity(source / 'db2')['build'])
                self.assertIsNotNone(conn.execute("SELECT name FROM sqlite_master WHERE name='challenge_dungeon'").fetchone(), 'Owned keystones require authentic challenge-to-Journal map data')
                self.assertEqual(conn.execute('SELECT map_id,name,time_limit_ms FROM challenge_dungeon WHERE ID=401').fetchone(), (2515, 'The Azure Vault', 2250000))
                self.assertIsNotNone(conn.execute("SELECT name FROM sqlite_master WHERE name='ItemSetSpell'").fetchone(), 'Retail set tooltip bonus thresholds must come from ItemSetSpell')
                self.assertEqual(conn.execute('SELECT SpellID,Threshold FROM ItemSetSpell WHERE ItemSetID=900 ORDER BY Threshold').fetchall(), [(70765, 2), (70762, 4)])
                self.assertEqual(conn.execute('SELECT instance_type FROM map_type WHERE ID=43').fetchone(), (1,))
                self.assertEqual(conn.execute('SELECT instance_type FROM map_type WHERE ID=409').fetchone(), (2,))
                self.assertEqual(conn.execute('SELECT COUNT(*) FROM JournalInstance j LEFT JOIN map_type m ON m.ID=j.MapID WHERE m.ID IS NULL').fetchone(), (0,))

    def test_rejects_mismatched_layout_without_replacing_cache(self):
        module = importlib.util.module_from_spec(SPEC)
        SPEC.loader.exec_module(module)
        source = Path(os.environ['ADVENTURE_GUIDE_SOURCE'])
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'journal.sqlite'
            out.write_bytes(b'old cache')
            with self.assertRaisesRegex(ValueError, 'layout'):
                module.read_table('JournalTier', source / 'db2/1237438.db2', Path(os.environ['ADVENTURE_GUIDE_DEFINITIONS']) / 'JournalTier.dbd')
            self.assertEqual(out.read_bytes(), b'old cache')


if __name__ == '__main__':
    unittest.main()
