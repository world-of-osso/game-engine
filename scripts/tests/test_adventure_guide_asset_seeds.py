import sqlite3
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from stage_adventure_guide_model_assets import seeds, summarize


def journal():
    db = sqlite3.connect(':memory:')
    db.executescript('''
        CREATE TABLE JournalEncounterCreature (ID INTEGER, JournalEncounterID INTEGER,
            CreatureDisplayInfoID INTEGER, OrderIndex INTEGER);
        -- Ozruk112: the Guide shows the lowest OrderIndex creature, not the lowest ID.
        INSERT INTO JournalEncounterCreature VALUES (20, 112, 36476, 1), (21, 112, 36475, 0);
        INSERT INTO JournalEncounterCreature VALUES (30, 1519, 13031, 0);
        -- Display 0 has no model; the encounter is reported, never seeded.
        INSERT INTO JournalEncounterCreature VALUES (40, 2000, 0, 0);
    ''')
    return db


class AdventureGuideAssetSeeds(unittest.TestCase):
    def test_seeds_follow_guide_preview_order_and_every_positive_display(self):
        self.assertEqual(seeds(journal()), [
            (112, 36475, [36475, 36476]),
            (1519, 13031, [13031]),
            (2000, 0, []),
        ])

    def test_encounter_needs_every_transitive_dependency_of_its_preview(self):
        assets = {
            (10, 'm2'): {'edges': [(None, 'display 36475 -> CreatureModelData 5')]},
            (11, 'skin'): {'edges': [(10, 'skin')]},
            (12, 'm2'): {'edges': [(None, 'display 36476 -> CreatureModelData 6')]},
            (20, 'm2'): {'edges': [(None, 'display 13031 -> CreatureModelData 7')]},
        }
        rows = seeds(journal())
        report = summarize(rows, assets, {(10, 'm2'): {}, (12, 'm2'): {}, (20, 'm2'): {}},
                           {(11, 'skin'): 'absent local CASC payload/root'})
        verdicts = {row['encounter']: (row['verified_preview_chain'], row['reasons']) for row in report['encounters']}
        self.assertEqual(verdicts[112], (False, ['11.skin: absent local CASC payload/root']))
        self.assertEqual(verdicts[1519], (True, []))
        self.assertEqual(verdicts[2000], (False, ['no sourced preview display']))
        self.assertEqual((report['displays'], report['verified_displays']), (3, 2))
        self.assertEqual((report['encounters_total'], report['verified_encounters']), (3, 1))


if __name__ == '__main__':
    unittest.main()
