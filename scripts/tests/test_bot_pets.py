"""Sourced gameplay pet preparation, never a granted starting kit."""
import csv
import sqlite3
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_pets import owned_pet, tame_plan, verified_tame_target


class PetPreparationTests(unittest.TestCase):
    def test_nearest_source_spawn_requires_beast_tame_flag_and_db2_family(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            family = root / 'CreatureFamily.csv'
            with family.open('w') as stream:
                writer = csv.writer(stream)
                writer.writerow(['ID', 'Name_lang'])
                writer.writerow([1, 'Wolf'])
            path = root / 'world.db'
            with closing(sqlite3.connect(path)) as db, db:
                db.executescript('CREATE TABLE content_creature(guid INTEGER,id1 INTEGER,map INTEGER,position_x REAL,position_y REAL,position_z REAL);'
                                 'CREATE TABLE content_creature_template(entry INTEGER,name TEXT,family INTEGER,type INTEGER,type_flags INTEGER,minlevel INTEGER);')
                for row in [(49871,'Blackrock Worg',1,1,1,1),(448,'Hogger',0,7,0,1),(6498,'Devilsaur',1,1,65537,1)]:
                    db.execute('INSERT INTO content_creature_template VALUES(?,?,?,?,?,?)',row)
                for row in [(20279985,49871,0,-8978.36,-173.215,81.5874),(20280006,49871,0,-8985.33,-149.46,81.8624),
                            (9,448,0,-8971,-161,81),(10,6498,0,-8971,-161,81)]:
                    db.execute('INSERT INTO content_creature VALUES(?,?,?,?,?,?)',row)
            before = path.read_bytes()
            plan = tame_plan(path, family, 0, 32, [-8970.988,81.50232,161.3487], [1515])
            self.assertEqual(plan['entry'],49871)
            self.assertEqual(plan['spawn_guid'],20279985)
            self.assertEqual(plan['family'], {'id':1,'name':'Wolf'})
            self.assertEqual(plan['goal'],[-8978.36,81.5874,173.215])
            self.assertEqual(path.read_bytes(),before)
            with self.assertRaisesRegex(ValueError,'learned|1515'):
                tame_plan(path,family,0,32,[-8971,81,161],[])
            family.write_text('ID,Name_lang\n2,Cat\n')
            with self.assertRaisesRegex(ValueError,'family'):
                tame_plan(path,family,0,32,[-8971,81,161],[1515])

    def test_tame_binds_actual_native_identity_health_and_level(self):
        plan = {'entry':49871,'name':'Blackrock Worg','level':32}
        state = {'target':{'target':71,'target_name':'Blackrock Worg'},'spells':{'target_health':500}}
        text = ('Blackrock Worg (entity 70) level 30 health 500/500 difficulty 0 combat={}\n'
                'Blackrock Worg (entity 71) level 30 health 500/500 difficulty 0 combat={}')
        self.assertEqual(verified_tame_target(state,text,plan)['entity'],'71')
        state['spells']['target_health'] = 0
        with self.assertRaisesRegex(ValueError,'alive|health'):
            verified_tame_target(state,text,plan)

    def test_server_pet_bar_alone_does_not_prove_replicated_owned_unit(self):
        self.assertIsNone(owned_pet({'pet':{'pet':81,'local_pet':None}}))
        self.assertIsNone(owned_pet({'pet':{'pet':81,'local_pet':82}}))
        self.assertEqual(owned_pet({'pet':{'pet':81,'local_pet':81}}),'81')


if __name__ == '__main__':
    unittest.main()
