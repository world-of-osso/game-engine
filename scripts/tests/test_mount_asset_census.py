import sys
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
from stage_mount_model_assets import census


class MountCensusTests(unittest.TestCase):
    def fixture(self):
        links = [{'MountID': '18', 'CreatureDisplayInfoID': '2404'}]
        displays = {2404: {'ModelID': '73', 'TextureVariationFileDataID_0': '999', 'ExtendedDisplayInfoID': '0'}}
        models = {73: {'FileDataID': '123'}}
        assets = {
            (123, 'm2'): {'edges': {(None, 'display 2404 -> CreatureModelData 73')}},
            (456, 'skin'): {'edges': {(123, 'SFID[0]')}},
            (999, 'blp'): {'edges': {(None, 'display 2404 TextureVariationFileDataID_0')}},
        }
        return links, displays, models, assets

    def test_root_receipt_does_not_hide_missing_skin(self):
        links, displays, models, assets = self.fixture()
        result = census(links, {18: {'Name_lang': 'Chestnut Mare'}}, displays, models,
                        assets, {(123, 'm2'), (999, 'blp')}, {(456, 'skin'): 'absent local archive payload'})
        self.assertEqual(result['mounts_with_model_receipt'], 1)
        self.assertEqual(result['mounts_with_verified_asset_chain'], 0)
        self.assertEqual(result['display_links'][0]['reasons'], ['456.skin: absent local archive payload'])

    def test_verified_chain_does_not_fabricate_missing_mount_identity(self):
        links, displays, models, assets = self.fixture()
        result = census(links, {}, displays, models, assets,
                        {(123, 'm2'), (456, 'skin'), (999, 'blp')}, {})
        self.assertEqual(result['mounts_with_verified_asset_chain'], 1)
        self.assertEqual(result['mount_rows_available'], 0)
        self.assertFalse(result['display_links'][0]['mount_row_available'])
        self.assertEqual(result['display_links'][0]['required_assets'], ['123.m2', '456.skin', '999.blp'])


if __name__ == '__main__':
    unittest.main()
