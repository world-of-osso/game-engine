"""Source plans and native target-bound provider, not a hardcoded class cycle."""
import json
import sqlite3
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_stockade import admitted_copy, authored_priority, encounter_states, parse_npc, parse_party, stockade_plan


class StockadeSourceTests(unittest.TestCase):
    def test_native_provider_preserves_authored_selection_and_refuses_unknown_rule(self):
        state = {"target": 41, "assisted": {"available": True, "failure_reason": "", "next_spell": 1464,
                 "evaluation_json": json.dumps({"target": 41, "selection": {"Ok": {"Spell": 1464}}})}}
        self.assertEqual(authored_priority(state, 41)["spell"], 1464)
        state["assisted"].update(failure_reason="rule21566 condition3 unverified")
        with self.assertRaisesRegex(ValueError, "unverified"):
            authored_priority(state, 41)
        state["assisted"].update(failure_reason="")
        with self.assertRaisesRegex(ValueError, "target"):
            authored_priority(state, 42)

    def test_native_party_requires_live_health_and_keeps_role_and_position(self):
        text = ('group_roster: 2\nBwarfury leader=true role=damage online=true subgroup=0 level=32 hp=8208/8208 death=Alive pos=56.7,-19.3,-0.6 debuffs=[]\n'
                'Captain Garrick leader=false role=tank online=true subgroup=0 level=32 hp=10584/10584 power=Mana:4430/4430 death=Alive pos=54.0,-19.3,-0.6 debuffs=[]')
        party = parse_party(text)
        self.assertEqual(party[1]["name"], "Captain Garrick")
        self.assertEqual(party[1]["position"], [54.0, -19.3, -0.6])
        self.assertEqual(party[1]["health"], 10584)
        self.assertEqual(party[1]["power"], ["Mana", 4430, 4430])
        with self.assertRaisesRegex(ValueError, "live|health"):
            parse_party('group_roster: 1\nCaptain Garrick leader=false role=tank online=true subgroup=0 level=32')

    def test_authoritative_copy_is_bound_to_character_and_normal_map_not_a_fixed_one(self):
        text = ('Created instance 1 of map 34 at difficulty 1 for InstanceEntrant { character_id: 69, group_id: None }\n'
                'Created instance 2 of map 34 at difficulty 1 for InstanceEntrant { character_id: 70, group_id: None }')
        self.assertEqual(admitted_copy(text, 70), 2)
        with self.assertRaisesRegex(ValueError, 'copy|instance'):
            admitted_copy(text, 71)
        with self.assertRaisesRegex(ValueError, 'copy|instance'):
            admitted_copy(text + '\nCreated instance 3 of map 34 at difficulty 1 for InstanceEntrant { character_id: 70, group_id: None }', 70)

    def test_live_npc_identity_and_encounter_credit_never_cross_instance_copies(self):
        text = ('Randolph Moloch (entity 4294959692) level 30 health 12/13137 difficulty 1 '
                'combat={"creature_armor":155.0,"stats":null,"threat":null,"victim":null}')
        npc = parse_npc(text, 46383)
        self.assertEqual(npc['entity'], '4294959692')
        self.assertEqual(npc['entry'], 46383)
        self.assertEqual(npc['health'], 12)
        with self.assertRaisesRegex(ValueError, 'unique|NPC'):
            parse_npc(text + '\n' + text, 46383)
        log = ('Instance InMap { map_id: 34, instance_id: 1 }: boss 0 (encounter 1146) InProgress -> Done\n'
               'Instance InMap { map_id: 34, instance_id: 2 }: boss 0 (encounter 1146) NotStarted -> InProgress\n'
               'Instance InMap { map_id: 34, instance_id: 2 }: boss 1 (encounter 1145) InProgress -> Done')
        self.assertEqual(encounter_states(log, 2), {1146: 'InProgress', 1145: 'Done'})

    def test_stockade_plan_joins_authored_encounters_spawns_and_entrance_read_only(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "source.sqlite"
            with closing(sqlite3.connect(path)) as db:
                db.executescript('CREATE TABLE dungeon_encounter(ID INTEGER,Name_lang TEXT,MapID INTEGER,DifficultyID INTEGER);'
                                 'CREATE TABLE content_creature_template(entry INTEGER,name TEXT);'
                                 'CREATE TABLE content_creature(guid INTEGER,id1 INTEGER,map INTEGER,position_x REAL,position_y REAL,position_z REAL);'
                                 'CREATE TABLE tdb_world_safe_locs(ID INTEGER,MapID INTEGER,LocX REAL,LocY REAL,LocZ REAL);')
                db.execute('INSERT INTO tdb_world_safe_locs VALUES(3599,34,56.6821,0.62376,-19.2691)')
                for entry, encounter, name, point in [(46383,1146,'Randolph Moloch',(172.463,-2.24164,-25.5229)),
                                                     (46264,1145,'Lord Overheat',(99.4527,-116.802,-35.1063)),
                                                     (46254,1144,'Hogger',(158.016,116.564,-35.1063))]:
                    db.execute('INSERT INTO dungeon_encounter VALUES(?,?,34,1)',(encounter,name))
                    db.execute('INSERT INTO content_creature_template VALUES(?,?)',(entry,name))
                    db.execute('INSERT INTO content_creature VALUES(?,?,34,?,?,?)',(entry,entry,*point))
                db.commit()
            original = path.read_bytes()
            plan = stockade_plan(path)
            self.assertEqual([row['entry'] for row in plan['bosses']], [46383,46264,46254])
            self.assertEqual(plan['bosses'][0]['goal'], [172.463,-25.5229,2.24164])
            self.assertEqual(plan['entrance'], [56.6821,-19.2691,-0.62376])
            self.assertEqual(path.read_bytes(), original)
            with closing(sqlite3.connect(path)) as db:
                db.execute('DELETE FROM content_creature WHERE id1=46254');db.commit()
            with self.assertRaisesRegex(ValueError, 'three|boss'):
                stockade_plan(path)


if __name__ == '__main__':
    unittest.main()
