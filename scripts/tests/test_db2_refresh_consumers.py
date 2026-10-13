"""Catalog-declared DB2 inputs join code and CSV consumers during a refresh."""
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from refresh_db2_install import consumed_tables


class RefreshConsumersTest(unittest.TestCase):
    def test_housing_catalog_inputs_are_exported_even_without_csv_or_code_literals(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            engine, server, definitions = root / 'engine', root / 'server', root / 'definitions'
            (engine / 'data/cache').mkdir(parents=True)
            definitions.mkdir()
            (definitions / 'CollectableSourceQuestSparse.dbd').write_text('COLUMNS\nint ID\n')
            connection = sqlite3.connect(engine / 'data/cache/housing.sqlite')
            connection.execute('CREATE TABLE housing_catalog_source(table_name TEXT)')
            connection.execute('INSERT INTO housing_catalog_source VALUES(?)', ('CollectableSourceQuestSparse',))
            connection.commit()
            connection.close()
            self.assertEqual(consumed_tables(engine, server, definitions), ['CollectableSourceQuestSparse'])


if __name__ == '__main__':
    unittest.main()
