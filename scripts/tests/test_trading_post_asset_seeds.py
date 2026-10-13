import json
import sqlite3
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage_trading_post_model_assets import seeds, summarize


def world():
    db = sqlite3.connect(':memory:')
    db.executescript('''
        CREATE TABLE perks_vendor_item (ID INTEGER PRIMARY KEY, payload TEXT);
        CREATE TABLE item_x_item_effect (ItemID INTEGER, ItemEffectID INTEGER);
        CREATE TABLE item_effect (ID INTEGER, SpellID INTEGER, TriggerType INTEGER);
        CREATE TABLE mount (ID INTEGER, SourceSpellID INTEGER);
        CREATE TABLE mount_x_display (ID INTEGER, MountID INTEGER, CreatureDisplayInfoID INTEGER, PlayerConditionID INTEGER);
        CREATE TABLE battle_pet_species (species_id INTEGER, display_id INTEGER, summon_spell_id INTEGER);
        CREATE TABLE spell_effect (SpellID INTEGER, DifficultyID INTEGER, Effect INTEGER, EffectMiscValue_0 INTEGER);
        CREATE TABLE item_modified_appearance (ID INTEGER, ItemID INTEGER, ItemAppearanceModifierID INTEGER, ItemAppearanceID INTEGER, OrderIndex INTEGER);
        CREATE TABLE item_appearance (ID INTEGER, ItemDisplayInfoID INTEGER);
        -- Mount 1: conditional display 900 is skipped for the unconditional 901.
        INSERT INTO item_x_item_effect VALUES (190000, 1), (190001, 2), (190003, 3);
        INSERT INTO item_effect VALUES (1, 5000, 6), (2, 6000, 6), (3, 7000, 0);
        INSERT INTO mount VALUES (11, 5000);
        INSERT INTO mount_x_display VALUES (1, 11, 900, 77), (2, 11, 901, 0);
        INSERT INTO battle_pet_species VALUES (3001, 4777, 6000);
        INSERT INTO spell_effect VALUES (7000, 0, 255, 42);
        INSERT INTO item_modified_appearance VALUES (500, 190002, 0, 60, 0), (501, 210, 0, 61, 0), (502, 211, 0, 62, 0);
        INSERT INTO item_appearance VALUES (60, 8001), (61, 8002), (62, 8003);
    ''')
    rows = [
        {'ID': 1, 'PerksVendorCategoryID': 2, 'ItemID': 190000, 'CreatureDisplayInfoID': 0},
        {'ID': 2, 'PerksVendorCategoryID': 3, 'ItemID': 190001, 'CreatureDisplayInfoID': 0},
        {'ID': 3, 'PerksVendorCategoryID': 1, 'ItemID': 190002, 'CreatureDisplayInfoID': 0},
        {'ID': 4, 'PerksVendorCategoryID': 8, 'ItemID': 190003, 'CreatureDisplayInfoID': 0},
        {'ID': 5, 'PerksVendorCategoryID': 3, 'ItemID': 190001, 'CreatureDisplayInfoID': 123},
        {'ID': 6, 'PerksVendorCategoryID': 5, 'ItemID': 190004, 'CreatureDisplayInfoID': 0},
    ]
    db.executemany('INSERT INTO perks_vendor_item VALUES (?, ?)', [(row['ID'], json.dumps(row)) for row in rows])
    return db


class TradingPostAssetSeeds(unittest.TestCase):
    def test_seeds_follow_server_preview_rules(self):
        sets = {42: {'set_id': 42, 'source_ids': [501, 502]}}
        self.assertEqual(seeds(world(), sets), [
            (1, 2, 'creature', [901]),
            (2, 3, 'creature', [4777]),
            (3, 1, 'item', [8001]),
            (4, 8, 'item', [8002, 8003]),
            (5, 3, 'creature', [123]),
        ])

    def test_row_needs_every_transitive_dependency_receipt(self):
        assets = {
            (10, 'm2'): {'edges': [(None, 'display 4777 -> CreatureModelData 5')]},
            (11, 'skin'): {'edges': [(10, 'skin')]},
            (20, 'm2'): {'edges': [(None, 'item display 8001 ModelResourcesID 1 (all component variants)')]},
        }
        rows = [(2, 3, 'creature', [4777]), (3, 1, 'item', [8001]), (9, 3, 'creature', [])]
        report = summarize(rows, assets, {(10, 'm2'): {}, (20, 'm2'): {}}, {(11, 'skin'): 'absent local CASC payload/root'})
        verdicts = {row['vendor_item']: (row['verified_asset_chain'], row['reasons']) for row in report['display_links']}
        self.assertEqual(verdicts[2], (False, ['11.skin: absent local CASC payload/root']))
        self.assertEqual(verdicts[3], (True, []))
        self.assertEqual(verdicts[9], (False, ['no sourced preview display']))
        self.assertEqual(report['by_category'], {'1': {'rows': 1, 'verified': 1}, '3': {'rows': 2, 'verified': 0}})


if __name__ == '__main__':
    unittest.main()
