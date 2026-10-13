import csv
import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / 'export_db2_csv.py'
SPEC = importlib.util.spec_from_file_location('arena_export', SCRIPT)
EXPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(EXPORT)


class ArenaExportTest(unittest.TestCase):
    def test_pinned_arena_and_shuffle_rows_are_exported_verbatim(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'BattlemasterList.csv'
            source = Path(__file__).with_name('fixtures') / 'pvp-1360265.db2'
            with patch('sys.argv', [str(SCRIPT), 'BattlemasterList', str(source), str(output)]):
                EXPORT.main()
            with output.open() as stream:
                rows = {int(r['ID']): r for r in csv.DictReader(stream)}
            self.assertEqual(rows[8]['Name_lang'], 'Ruins of Lordaeron')
            self.assertEqual(rows[8]['InstanceType'], '4')
            self.assertEqual(rows[8]['MinLevel'], '10')
            self.assertEqual(rows[1065]['MaxPlayers'], '6')
            self.assertEqual(rows[1065]['GroupsAllowed'], '0')
            self.assertEqual(rows[1117]['Name_lang'], 'Cage of Carnage')


if __name__ == '__main__':
    unittest.main()
