"""Source plans and native target-bound provider, not a hardcoded class cycle."""
import json
import sqlite3
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path
from types import SimpleNamespace

from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_stockade import admitted_copy, authored_priority, encounter_states, parse_npc, parse_party, stockade_exit_plan, stockade_plan, target_observation_ready


class StockadeOpeningTests(unittest.TestCase):
    def test_idle_pull_tabs_and_acknowledges_throw_before_any_movement(self):
        from run_stockade_bot import StockadeRun
        calls = []
        controller = StockadeRun.__new__(StockadeRun)
        controller.opening_target = None
        controller.run = {"clear_elapsed_ms": None, "bosses": [
            {"state": "NotStarted", "goal": [103, -19, 0]}], "openings": []}
        controller.tank = "Captain Garrick"
        controller.poll_combat = lambda: None
        controller.record_credit = lambda: None
        controller.party = lambda: [{"health": 100, "max_health": 100}]
        controller.account = lambda: {"local_player_position": "(56,-19,0)"}
        controller.snapshot = lambda target: {"at_ms": 1000, "spells": {"known": [57755]},
            "facts": {"known": [57755], **{field: {"57755": {"Ok": True}}
                for field in ("in_range", "ready", "affordable")}}}
        controller.move = lambda *args: self.fail("Moved before ranged opening")
        controller.player = SimpleNamespace(
            assist=lambda tank: None, tab_to_enemy=lambda: 41,
            cast_opening=lambda target, spell: calls.append((target, spell)) or {"success": True})
        with tempfile.TemporaryDirectory() as root:
            controller.output = Path(root)
            with patch('run_stockade_bot.time.sleep'):
                self.assertFalse(controller.step())
            receipt = json.loads((Path(root) / 'opening-0001.json').read_text())
            self.assertTrue(receipt['outcome']['success'])
        self.assertEqual(calls, [(41, 57755)])
        self.assertEqual(controller.opening_target, 41)


    def test_native_range_stops_strict_approach_before_waiting_on_cooldown(self):
        from run_stockade_bot import StockadeRun
        controller = StockadeRun.__new__(StockadeRun)
        states = []
        for ranged, ready in [(False, True), (True, False), (True, True)]:
            state = {"at_ms": 1000, "spells": {"known": [57755]},
                "facts": {"known": [57755], **{field: {"57755": {"Ok": True}}
                    for field in ("in_range", "ready", "affordable")}}}
            state["facts"]["in_range"]["57755"] = {"Ok": ranged}
            state["facts"]["ready"]["57755"] = {"Ok": ready}
            state["target_position"] = [93, -25, 0]
            states.append(state)
        iterator = iter(states)
        controller.snapshot = lambda target: next(iterator)
        controller.party = lambda: []
        controller.poll_combat = lambda: None
        controller.account = lambda: {"local_player_position": "(56,-19,0)"}
        moved = []
        controller.move = lambda here, goal, label: moved.append((here, goal, label))
        with tempfile.TemporaryDirectory() as root:
            controller.output = Path(root)
            with patch('run_stockade_bot.time.sleep'):
                controller.approach_opening(41)
        self.assertEqual(moved, [([56, -19, 0], [93, -25, 0], "native-ranged-approach")])


class StockadeSourceTests(unittest.TestCase):
    def test_actual_dead_assist_target_needs_no_authored_evaluation_and_never_casts(self):
        fixture = Path(__file__).parent / 'fixtures/bot_harness/native-stockade-dead-target.json'
        state = json.loads(fixture.read_text())
        self.assertEqual(state['spells']['target_health'], 0)
        self.assertNotIn('evaluation_json', state['assisted'])
        self.assertTrue(target_observation_ready(state, state['target']))
        self.assertIsNone(authored_priority(state, state['target']))
        state['spells']['target_health'] = 20
        self.assertFalse(target_observation_ready(state, state['target']))
        state['assisted']['evaluation_json'] = json.dumps({'target': state['target']})
        self.assertTrue(target_observation_ready(state, state['target']))
        self.assertFalse(target_observation_ready(state, state['target'] + 1))

    def test_exit_uses_authored_trigger_shape_and_world_safe_location_not_reposition(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / 'exit.sqlite'
            with closing(sqlite3.connect(path)) as db:
                db.executescript('CREATE TABLE area_trigger(ID INTEGER,ContinentID INTEGER,Pos_0 REAL,Pos_1 REAL,Pos_2 REAL,ShapeType INTEGER,Box_length REAL,Box_width REAL,Box_height REAL,Box_yaw REAL);'
                                 'CREATE TABLE tdb_areatrigger_teleport(ID INTEGER,PortLocID INTEGER);'
                                 'CREATE TABLE tdb_world_safe_locs(ID INTEGER,MapID INTEGER,LocX REAL,LocY REAL,LocZ REAL);')
                db.execute('INSERT INTO area_trigger VALUES(503,34,48.0937233,0.933266521,-13.7769003,1,6,12,11.3136997,0)')
                db.execute('INSERT INTO tdb_areatrigger_teleport VALUES(503,3618)')
                db.execute('INSERT INTO tdb_world_safe_locs VALUES(3618,0,-8762,848,87)');db.commit()
            plan = stockade_exit_plan(path)
            self.assertEqual(plan['trigger_id'], 503)
            self.assertEqual(plan['goal'], [48.0937233,-13.7769003,-0.933266521])
            self.assertEqual(plan['destination_map'], 0)
            from run_stockade_bot import StockadeRun
            controller = StockadeRun.__new__(StockadeRun)
            controller.args = SimpleNamespace(world_db=path, character='Bwarfury')
            controller.output = Path(root)
            (Path(root) / 'native-account-state.json').write_text('{}')
            # Actual terminal map34 observation has nav-surface Y below trigger center.
            base = {'screen': 'InWorld', 'selected_character_name': 'Bwarfury', 'local_player_health': 8256}
            observations = iter([{**base, 'world_map_id': 34,
                                  'local_player_position': '(50.49923, -16.96527, -0.846576)'},
                                 {**base, 'world_map_id': 0,
                                  'local_player_position': '(-8766.11,87.9952,-845.499)'}])
            controller.account = lambda: next(observations)
            from bot_controller import PlayerController
            def unexpected_movement(*words):
                self.fail(f"Movement submitted after native entry into authored trigger volume: {words}")
            controller.player = PlayerController(unexpected_movement, None,
                lambda *args: [[48.09372, -15.718611, -0.933267]], None)
            with patch('run_stockade_bot.time.sleep'):
                controller.exit_instance()
            confirmation = json.loads((Path(root) / 'exit-confirmation.json').read_text())
            self.assertEqual(confirmation['account']['world_map_id'], 0)
            with closing(sqlite3.connect(path)) as db:
                db.execute('DELETE FROM tdb_areatrigger_teleport');db.commit()
            with self.assertRaisesRegex(ValueError, 'exit|trigger'):
                stockade_exit_plan(path)

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
