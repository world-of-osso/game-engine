"""Observable pinned local catalog decoding and SQL application to an owned fixture."""
import importlib.util
import os
from pathlib import Path
import sqlite3
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'trading_post_import.py'
SPEC = importlib.util.spec_from_file_location('trading_post_import', SCRIPT)
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class TradingPostImportTests(unittest.TestCase):
    def test_authentic_vendor_categories_prices_and_featured_groups(self):
        source = Path(os.environ['TRADING_POST_SOURCE'])
        catalog = module.decode_catalog(source / 'db2', source / 'definitions')
        self.assertIn('perks_vendor_item', catalog)
        vendor = {row['ID']: row for row in catalog['perks_vendor_item']['records']}
        self.assertEqual((vendor[4]['ItemID'], vendor[4]['Cost']), (189869, 100))
        self.assertEqual((vendor[7]['ItemID'], vendor[7]['Cost']), (189938, 850))
        self.assertGreater(len(vendor), 1000)
        categories = {row['ID']: row for row in catalog['perks_vendor_category']['records']}
        self.assertEqual(categories[2]['DisplayName_lang'], 'Mounts')
        self.assertEqual(catalog['perks_vendor_category']['unreadable'], 1)
        groups = {row['ID']: row for row in catalog['perks_vendor_item_ui_group']['records']}
        self.assertEqual(groups[2]['Name_lang'], 'New Arrivals')
        sql, report = module.prepare_sql(catalog)
        with sqlite3.connect(':memory:') as conn:
            conn.executescript(sql)
            self.assertEqual(conn.execute("SELECT json_extract(payload,'$.Cost') FROM perks_vendor_item WHERE ID=4").fetchone(), (100,))
            self.assertEqual(conn.execute("SELECT build,layout FROM trading_post_source WHERE name='PerksVendorItem'").fetchone(), ('12.1.0.69933', '96A2B1EB'))
        self.assertEqual(report['sources']['PerksVendorItem']['readable'], len(vendor))

    def test_layout_mismatch_is_rejected(self):
        source = Path(os.environ['TRADING_POST_SOURCE'])
        with self.assertRaisesRegex(ValueError, 'layout'):
            module.read_table('PerksVendorItem', source / 'db2/4281310.db2', source / 'definitions/PerksVendorItem.dbd')


if __name__ == '__main__':
    unittest.main()
