"""Catalog-declared DB2 inputs join code and CSV consumers during a refresh."""
from pathlib import Path
import hashlib
import json
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from refresh_db2_install import consumed_tables
from refresh_db2_data import link_file, prepare_housing_cache
from types import SimpleNamespace


def housing_tag_refresh_fixture(directory):
    root = Path(directory)
    engine = root / 'engine'
    (engine / 'data/cache').mkdir(parents=True)
    output = root / 'output'
    build = '12.1.5.70448'
    snapshot = output / 'db2' / build
    snapshot.mkdir(parents=True)
    destination = output / 'client-data'
    (destination / 'cache').mkdir(parents=True)
    receipt = {'identity': {'build_key': '88d5e8ef05ec5a8446cdec93d142d7c6', 'build': build},
               'tables': {}, 'unavailable': {}}
    exports = {
        'DataTagXHouseDecorRecord': (7570371, '5557A217', 'ID,DataTagID,HouseDecorID\n1531,66,1693\n'),
        'DataTagInfo': (6652252, 'DB46F772', 'Description_lang,ID,DataTag\nSmall,2,66\n'),
        'DataTagGroup': (6653645, 'EDA13DE2', 'ID,Description_lang\n2,Size\n'),
        'DataTagGroupXDataTag': (6653584, '62484461', 'ID,DataTagGroupID,DataTag\n4,2,66\n'),
    }
    for table, (fdid, layout, content) in exports.items():
        path = snapshot / f'{table}.csv'
        path.write_text(content)
        receipt['tables'][table] = {
            'fdid': fdid, 'layout': layout, 'rows': 1, 'unreadable_records': 0,
            'csv_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
            'db2_sha256': 'fixture-byte-receipt', 'id_column': 'ID',
        }
    baseline = engine / 'data/cache/housing.sqlite'
    with sqlite3.connect(baseline) as db:
        db.executescript('''
            CREATE TABLE housing_catalog_build (
                build_key TEXT PRIMARY KEY, build TEXT, report_sha256 TEXT, ledger_sha256 TEXT);
            CREATE TABLE housing_catalog_source (
                build_key TEXT, table_name TEXT, fdid INTEGER, status TEXT, layout_hash TEXT,
                exported_records INTEGER, encrypted_records_dropped INTEGER,
                csv_sha256 TEXT, db2_sha256 TEXT, receipt_json TEXT,
                PRIMARY KEY (build_key,table_name));
            CREATE TABLE housing_catalog_record (
                build_key TEXT, table_name TEXT, record_id INTEGER, fields_json TEXT,
                PRIMARY KEY (build_key,table_name,record_id));
        ''')
        db.execute('INSERT INTO housing_catalog_source(build_key,table_name) VALUES(?,?)',
                   (receipt['identity']['build_key'], 'DataTagXHouseDecorRecord'))
    return SimpleNamespace(engine=engine, output=output), receipt, destination, baseline


class RefreshConsumersTest(unittest.TestCase):
    def test_housing_refresh_ingests_available_tag_definitions_not_in_the_old_catalog(self):
        with tempfile.TemporaryDirectory() as directory:
            args, receipt, destination, baseline = housing_tag_refresh_fixture(directory)
            before = baseline.read_bytes()
            prepare_housing_cache(args, receipt, destination)
            with sqlite3.connect(destination / 'cache/housing.sqlite') as db:
                sources = dict(db.execute('SELECT table_name,exported_records FROM housing_catalog_source'))
                self.assertEqual(sources, {table: 1 for table in receipt['tables']})
                expected = {
                    ('DataTagInfo', 2): {'Description_lang': 'Small', 'ID': '2', 'DataTag': '66'},
                    ('DataTagGroup', 2): {'ID': '2', 'Description_lang': 'Size'},
                    ('DataTagGroupXDataTag', 4): {'ID': '4', 'DataTagGroupID': '2', 'DataTag': '66'},
                    ('DataTagXHouseDecorRecord', 1531): {'ID': '1531', 'DataTagID': '66', 'HouseDecorID': '1693'},
                }
                records = {(table, record_id): json.loads(fields) for table, record_id, fields in db.execute(
                    'SELECT table_name,record_id,fields_json FROM housing_catalog_record')}
                self.assertEqual(records, expected)
                for table, csv_sha, source_receipt in db.execute(
                        'SELECT table_name,csv_sha256,receipt_json FROM housing_catalog_source'):
                    self.assertEqual(csv_sha, receipt['tables'][table]['csv_sha256'])
                    self.assertEqual(json.loads(source_receipt), receipt['tables'][table])
            self.assertEqual(baseline.read_bytes(), before)

    def test_unavailable_tag_definition_retains_its_gap_without_invented_records(self):
        with tempfile.TemporaryDirectory() as directory:
            args, receipt, destination, _ = housing_tag_refresh_fixture(directory)
            missing = receipt['tables'].pop('DataTagInfo')
            receipt['unavailable']['DataTagInfo'] = {
                'fdid': missing['fdid'], 'reason': 'absent from authenticated local root'}
            (args.output / 'db2/12.1.5.70448/DataTagInfo.csv').unlink()
            prepare_housing_cache(args, receipt, destination)
            with sqlite3.connect(destination / 'cache/housing.sqlite') as db:
                self.assertEqual(db.execute(
                    "SELECT fdid,status,exported_records FROM housing_catalog_source WHERE table_name='DataTagInfo'").fetchall(),
                    [(6652252, 'root_unavailable', 0)])
                self.assertEqual(db.execute(
                    "SELECT COUNT(*) FROM housing_catalog_record WHERE table_name='DataTagInfo'").fetchone()[0], 0)

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
