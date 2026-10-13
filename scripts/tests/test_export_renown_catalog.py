import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from export_renown_catalog import build_catalog


def faction(id, name, renown_currency, threshold=2500, race_mask=-1, expansion=9):
    return {'ID': id, 'Name_lang': name, 'Description_lang': '', 'Expansion': expansion,
            'RenownCurrencyID': renown_currency, 'ReputationClassMask_0': -1,
            'ReputationRaceMasks0_0': race_mask, 'ReputationRaceMasks0_1': race_mask,
            'ReputationMax_0': threshold}


def reward(id, level, ui_order, name):
    return {'ID': id, 'CovenantID': 12, 'Level': level, 'UiOrder': ui_order, 'Name_lang': name,
            'Description_lang': '', 'ToastDescription_lang': '', 'Flags': 8, 'Icon': 3012071,
            'ItemID': 0, 'SpellID': 0, 'MountID': 0, 'TransmogID': 0, 'TransmogSetID': 0,
            'CharTitlesID': 0, 'TransmogIllusionID': 0, 'QuestID': 0, 'PlayerConditionID': 0}


# Pinned Retail 12.1.0.69933 rows (Covenant12 Maruuk Centaur, Covenant3 Night Fae, empty Covenant43).
COVENANTS = [
    {'ID': 3, 'Name_lang': 'Night Fae', 'FactionID': 2465, 'CurrencyTypesID': 1831, 'RequiredPlayerConditionID': 0},
    {'ID': 12, 'Name_lang': 'Maruuk Centaur', 'FactionID': 2503, 'CurrencyTypesID': 2002, 'RequiredPlayerConditionID': 104408},
    {'ID': 43, 'Name_lang': '', 'FactionID': 0, 'CurrencyTypesID': 0, 'RequiredPlayerConditionID': 0},
]
CURRENCIES = {2002: {'ID': 2002, 'MaxQty': 25}, 1831: {'ID': 1831, 'MaxQty': 80}}
CONDITIONS = {104408: {'ID': 104408, 'Failure_description_lang':
                       "Complete the quest Maruukai with the centaur caravan in the Ohn'ahran Plains."}}


class RenownCatalog(unittest.TestCase):
    def test_joins_renown_covenants_to_faction_currency_and_ordered_rewards(self):
        factions = {2503: faction(2503, 'Maruuk Centaur', 2002), 2465: faction(2465, 'The Night Fae', 0)}
        rewards = [reward(622, 2, 0, 'Centaur Hunting Trophies'), reward(621, 1, 0, 'Among the Maruuk Centaur')]
        catalog = build_catalog(COVENANTS, factions, CURRENCIES, rewards, CONDITIONS)
        self.assertEqual([row['id'] for row in catalog], [12])
        maruuk = catalog[0]
        self.assertEqual((maruuk['faction_id'], maruuk['name'], maruuk['expansion'], maruuk['max_level'],
                          maruuk['level_threshold'], maruuk['player_condition_id']),
                         (2503, 'Maruuk Centaur', 9, 25, 2500, 104408))
        self.assertEqual(maruuk['unlock_description'],
                         "Complete the quest Maruukai with the centaur caravan in the Ohn'ahran Plains.")
        self.assertEqual([(row['level'], row['name']) for row in maruuk['rewards']],
                         [(1, 'Among the Maruuk Centaur'), (2, 'Centaur Hunting Trophies')])

    def test_unreadable_condition_text_stays_absent(self):
        factions = {2503: faction(2503, 'Maruuk Centaur', 2002)}
        catalog = build_catalog(COVENANTS, factions, CURRENCIES, [], {})
        self.assertIsNone(catalog[0]['unlock_description'])

    def test_race_specific_threshold_is_an_error_not_a_guess(self):
        factions = {2503: faction(2503, 'Maruuk Centaur', 2002, race_mask=1)}
        with self.assertRaisesRegex(ValueError, '2503'):
            build_catalog(COVENANTS, factions, CURRENCIES, [], CONDITIONS)


if __name__ == '__main__':
    unittest.main()
