"""Catalog-declared DB2 inputs join code and CSV consumers during a refresh."""
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from refresh_db2_install import consumed_tables
from refresh_db2_data import link_file, prepare_housing_cache
from types import SimpleNamespace


class RefreshConsumersTest(unittest.TestCase):
    def test_relinking_an_existing_input_does_not_create_a_self_referential_link(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source.csv'
            source.write_text('ID,Name\n1,Armor\n')
            link = root / 'published.csv'
            link.symlink_to(source)
            link_file(link, link)
            self.assertEqual(link.read_text(), source.read_text())

    def test_failed_housing_refresh_preserves_the_previous_catalog_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cache = root / 'output/client-data/cache'
            cache.mkdir(parents=True)
            (root / 'engine/data').mkdir(parents=True)
            (root / 'engine/data/cache').symlink_to(cache, target_is_directory=True)
            path = cache / 'housing.sqlite'
            with sqlite3.connect(path) as db:
                db.execute('CREATE TABLE housing_catalog_source(table_name TEXT)')
                db.execute("INSERT INTO housing_catalog_source VALUES('CollectableSourceQuestSparse')")
                db.execute('CREATE TABLE housing_catalog_build(build_key TEXT,build TEXT,report TEXT,ledger TEXT)')
            before = path.read_bytes()
            args = SimpleNamespace(engine=root / 'engine', output=root / 'output')
            receipt = {'identity': {'build_key': '334a36cf5f3c32a5f7a167405487fa14', 'build': '12.1.5.70391'}, 'tables': {}, 'unavailable': {}}
            with self.assertRaises(KeyError):
                prepare_housing_cache(args, receipt, root / 'output/client-data')
            self.assertEqual(path.read_bytes(), before)

    def test_runtime_crate_inputs_join_the_consumed_table_set(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            definitions = root / 'definitions'
            definitions.mkdir()
            (definitions / 'AreaTable.dbd').write_text('COLUMNS\nint ID\n')
            runtime = root / 'server/crates/server/src'
            runtime.mkdir(parents=True)
            (runtime / 'area.rs').write_text('const TABLE: &str = "AreaTable";')
            self.assertEqual(consumed_tables(root / 'engine', root / 'server', definitions), ['AreaTable'])

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
