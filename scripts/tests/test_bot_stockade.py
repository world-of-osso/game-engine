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
        controller.snapshot = lambda target: {"at_ms": 1000, "target": target,
            "assisted": {"in_combat": False}, "visibility": {"target": target, "visible": True},
            "spells": {"known": [57755], "target_health": 100},
            "facts": {"known": [57755], **{field: {"57755": {"Ok": True}}
                for field in ("in_range", "ready", "affordable")}}}
        controller.move = lambda *args: self.fail("Moved before ranged opening")
        controller.rotation = lambda: controller.snapshot(41)
        controller.player = SimpleNamespace(
            observe_cast_outcome=lambda state: True,
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
            state = {"at_ms": 1000, "target": 41, "assisted": {"in_combat": False},
                "visibility": {"target": 41, "visible": True},
                "spells": {"known": [57755], "target_health": 100},
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


class StockadeDeadPreparationTests(unittest.TestCase):
    def test_captured_alive_to_dead_preparation_abandons_before_corpse_movement(self):
        from run_stockade_bot import StockadeRun
        path = Path(__file__).parent / 'fixtures/bot_harness/native-stockade-prep-target-death.json'
        fixture = json.loads(path.read_text())
        state = fixture["alive"]
        controller = StockadeRun.__new__(StockadeRun)
        controller.args = SimpleNamespace(spec=72)
        controller.run = {"openings": []}
        controller.opening_target = None
        controller.rotation = lambda: state
        controller.party = lambda: []
        controller.poll_combat = lambda: None
        controller.account = lambda: {"local_player_position": "(108,-26,-12)"}
        controller.player = SimpleNamespace(tab_to_enemy=lambda: 4294739425,
            cast_opening=lambda *args: self.fail("Cast at dead preparation target"))
        movements = []
        def move(*args):
            nonlocal state
            self.assertGreater(state["spells"]["target_health"], 0, "Moved after confirmed target death")
            movements.append(args)
            state = fixture["dead"]
        controller.move = move
        with tempfile.TemporaryDirectory() as root:
            controller.output = Path(root)
            with patch('run_stockade_bot.time.sleep'):
                controller.open_pull()
            receipt = json.loads((Path(root) / 'opening-0001.json').read_text())
            self.assertEqual(receipt["abandoned"], "received selected target health zero before cast")
            self.assertNotIn("outcome", receipt)
        self.assertEqual(len(movements), 1)
        self.assertIsNone(controller.opening_target)


class StockadeCombatOpeningTests(unittest.TestCase):
    def captured_controller(self, output):
        from run_stockade_bot import StockadeRun
        path = Path(__file__).parent / 'fixtures/bot_harness/native-stockade-engaged-opening.json'
        state = json.loads(path.read_text())
        controller = StockadeRun.__new__(StockadeRun)
        controller.output = output
        controller.run = {"openings": []}
        controller.opening_target = None
        controller.rotation = lambda: state
        controller.snapshot = lambda target: state
        controller.party = lambda: []
        controller.poll_combat = lambda: None
        controller.account = lambda: {"local_player_position": "(108.4171,-26.5229,-12.46661)"}
        controller.player = SimpleNamespace(
            tab_to_enemy=lambda: self.fail("New Tab while received owner is in combat"),
            cast_opening=lambda *args: self.fail("New cast while received owner is in combat"))
        controller.move = lambda *args: self.fail("Moved during engaged opening")
        return controller, state

    def test_captured_engaged_vicious_thug_does_not_start_an_opening(self):
        with tempfile.TemporaryDirectory() as root:
            controller, state = self.captured_controller(Path(root))
            self.assertEqual(state["target"], 4294739427)
            self.assertEqual(state["provenance"]["target_already_engaged_ms_before_opening"], 3764)
            controller.open_pull()
            self.assertEqual(controller.run["openings"], [])
            self.assertIsNone(controller.opening_target)

    def test_combat_start_during_preparation_abandons_without_cast_or_forced_target(self):
        with tempfile.TemporaryDirectory() as root:
            controller, state = self.captured_controller(Path(root))
            state["assisted"]["in_combat"] = False
            controller.player.tab_to_enemy = lambda: int(state["target"])
            movements = []
            def move(*args):
                movements.append(args)
                state["assisted"]["in_combat"] = True
                state["visibility"]["visible"] = True
                for field in ("in_range", "ready", "affordable"):
                    state["facts"][field]["57755"] = {"Ok": True}
            controller.move = move
            with patch('run_stockade_bot.time.sleep'):
                controller.open_pull()
            self.assertEqual(len(movements), 1)
            receipt = json.loads((Path(root) / 'opening-0001.json').read_text())
            self.assertEqual(receipt["abandoned"], "received owner CombatStatus true before cast")
            self.assertNotIn("outcome", receipt)
            self.assertIsNone(controller.opening_target)

    def test_missing_owner_combat_status_blocks_new_opening(self):
        with tempfile.TemporaryDirectory() as root:
            controller, state = self.captured_controller(Path(root))
            del state["assisted"]["in_combat"]
            with self.assertRaisesRegex(ValueError, "combat status"):
                controller.open_pull()


class StockadeMeleeEndpointTests(unittest.TestCase):
    def endpoint_controller(self):
        from bot_controller import PlayerController
        from run_stockade_bot import StockadeRun
        path = Path(__file__).parent / 'fixtures/bot_harness/native-stockade-melee-endpoint.json'
        fixture = json.loads(path.read_text())
        controller = StockadeRun.__new__(StockadeRun)
        controller.run = {"clear_elapsed_ms": None}
        controller.opening_target = None
        controller.tank = "Captain Garrick"
        controller.blocked_steps = 0
        controller.poll_combat = lambda: None
        controller.record_credit = lambda: None
        controller.party = lambda: [{"health": 80, "max_health": 100}]
        controller.rotation = lambda: fixture["rotation"]
        controller.snapshot = lambda target: fixture["rotation"]
        controller.account = lambda: {"local_player_position": fixture["player_position"]}
        inputs = []
        controller.player = PlayerController(lambda *args: inputs.append(args), controller.snapshot,
            lambda *args: self.fail("Queried a route after native endpoint completion"), None)
        controller.player.assist = lambda tank: int(fixture["rotation"]["target"])
        return controller, fixture, inputs

    def test_actual_two_point_five_yard_endpoint_releases_native_usable_execute(self):
        controller, fixture, inputs = self.endpoint_controller()
        with patch('run_stockade_bot.time.sleep'):
            self.assertFalse(controller.step())
        self.assertEqual(inputs, [("spell", "cast", "--spell", "5308", "--target",
                                   str(int(fixture["rotation"]["target"])))])
        self.assertEqual(controller.player.pending["spell"], 5308)

    def test_reached_endpoint_still_obeys_native_spell_range(self):
        controller, fixture, inputs = self.endpoint_controller()
        fixture["rotation"]["facts"]["in_range"]["5308"] = {"Ok": False}
        with patch('run_stockade_bot.time.sleep'):
            controller.step()
        self.assertEqual(inputs, [])
        self.assertIsNone(controller.player.pending)

    def test_physical_route_step_waits_for_fresh_observation_before_cast(self):
        controller, fixture, inputs = self.endpoint_controller()
        fixture["player_position"] = "(80,-25.60623,-.839278)"
        controller.player.route = lambda here, goal, label: [goal]
        with patch('run_stockade_bot.time.sleep'):
            controller.step()
        self.assertEqual(len(inputs), 1)
        self.assertEqual(inputs[0][:2], ("movement", "forward"))
        self.assertIsNone(controller.player.pending)


class StockadeReceiptLifecycleTests(unittest.TestCase):
    def switched_target_controller(self):
        from bot_controller import PlayerController
        from run_stockade_bot import StockadeRun
        path = Path(__file__).parent / 'fixtures/bot_harness/native-stockade-kill-switch-receipt.json'
        fixture = json.loads(path.read_text())
        controller = StockadeRun.__new__(StockadeRun)
        controller.run = {"clear_elapsed_ms": None}
        controller.opening_target = None
        controller.tank = "Captain Garrick"
        controller.poll_combat = lambda: None
        controller.record_credit = lambda: None
        controller.party = lambda: [{"health": 80, "max_health": 100}]
        controller.rotation = lambda: fixture["after"]
        controller.snapshot = lambda target: fixture["after"]
        controller.account = lambda: {"local_player_position": "(0,0,0)"}
        controller.player = PlayerController(lambda *args: None, controller.snapshot, None, None)
        controller.player.assist = lambda tank: int(fixture["after"]["target"])
        controller.player.submit_cast(fixture["before"], fixture["before"]["target"], 5308)
        return controller, fixture

    def test_execute_kill_target_switch_consumes_real_receipt_before_movement(self):
        controller, fixture = self.switched_target_controller()
        moves = []
        def move(*args):
            self.assertIsNone(controller.player.pending, "Kill receipt not consumed before target-switch movement")
            moves.append(args)
        controller.move = move
        with patch('run_stockade_bot.time.sleep'):
            self.assertFalse(controller.step())
        self.assertEqual(len(moves), 1)
        self.assertTrue(controller.player.outcomes[-1]["success"])
        self.assertEqual(controller.player.outcomes[-1]["spell"], 5308)

    def test_target_switch_without_receipt_waits_before_movement_or_new_input(self):
        controller, fixture = self.switched_target_controller()
        fixture["after"]["spells"] = fixture["before"]["spells"]
        moves = []
        controller.move = lambda *args: moves.append(args)
        with patch('run_stockade_bot.time.sleep'):
            self.assertFalse(controller.step())
        self.assertEqual(moves, [])
        self.assertIsNotNone(controller.player.pending)
        self.assertEqual(controller.player.outcomes, [])

    def test_missing_kill_receipt_times_out_without_reset_or_movement(self):
        controller, fixture = self.switched_target_controller()
        fixture["after"]["spells"] = fixture["before"]["spells"]
        controller.move = lambda *args: self.fail("Movement after missing outcome deadline")
        deadline = controller.player.pending["deadline"]
        with patch('run_stockade_bot.time.monotonic', return_value=deadline + 1):
            with self.assertRaisesRegex(TimeoutError, "outcome not observed"):
                controller.step()
        self.assertIsNotNone(controller.player.pending)


class StockadeVisibilityTests(unittest.TestCase):
    def poacher_controller(self, output):
        from run_stockade_bot import StockadeRun
        fixture = Path(__file__).parent / 'fixtures/bot_harness/native-stockade-poacher-blocked.json'
        state = json.loads(fixture.read_text())
        state["assisted"] = {"in_combat": False}
        state["spells"]["target_health"] = 22527
        controller = StockadeRun.__new__(StockadeRun)
        controller.output = output
        controller.run = {"openings": []}
        controller.opening_target = None
        controller.party = lambda: []
        controller.poll_combat = lambda: None
        controller.snapshot = lambda target: state
        controller.rotation = lambda: state
        controller.account = lambda: {"local_player_position": "(98.53893,-25.60623,.300785)"}
        controller.player = SimpleNamespace(tab_to_enemy=lambda: int(state["target"]))
        return controller, state

    def test_riverpaw_poacher_in_range_moves_before_one_cast_only_when_visible(self):
        with tempfile.TemporaryDirectory() as root:
            controller, state = self.poacher_controller(Path(root))
            inputs = []
            def move(here, goal, label):
                inputs.append(("move", here, goal))
                state["visibility"]["visible"] = True
            def cast(target, spell):
                inputs.append(("cast", target, spell))
                return {"success": True}
            controller.move = move
            controller.player.cast_opening = cast
            with patch('run_stockade_bot.time.sleep'):
                controller.open_pull()
            self.assertEqual(inputs, [
                ("move", [98.53893, -25.60623, .300785], state["target_position"]),
                ("cast", 4294739508, 57755)])

    def test_blocked_poacher_path_limit_stops_without_cast(self):
        with tempfile.TemporaryDirectory() as root:
            controller, state = self.poacher_controller(Path(root))
            positions = iter(["(0,0,0)", "(61,0,0)"])
            controller.account = lambda: {"local_player_position": next(positions)}
            controller.move = lambda *args: None
            controller.player.cast_opening = lambda *args: self.fail("Cast while visibility false")
            with patch('run_stockade_bot.time.sleep'):
                with self.assertRaisesRegex(RuntimeError, "opening.*path|path.*limit"):
                    controller.open_pull()

    def test_missing_stale_or_failed_visibility_never_casts_or_moves(self):
        for visibility in [{}, {"target": 1, "visible": True},
                           {"target": 4294739508, "error": "missing eye table"}]:
            with self.subTest(visibility=visibility), tempfile.TemporaryDirectory() as root:
                controller, state = self.poacher_controller(Path(root))
                state["visibility"] = visibility
                controller.move = lambda *args: self.fail("Moved without target visibility")
                controller.player.cast_opening = lambda *args: self.fail("Cast without target visibility")
                with self.assertRaisesRegex(ValueError, "visibility"):
                    controller.open_pull()

    def test_blocked_poacher_time_limit_stops_without_cast(self):
        with tempfile.TemporaryDirectory() as root:
            controller, state = self.poacher_controller(Path(root))
            controller.move = lambda *args: None
            controller.player.cast_opening = lambda *args: self.fail("Cast while visibility false")
            with patch('run_stockade_bot.time.sleep'), patch('run_stockade_bot.time.monotonic', side_effect=[0, 0, 61]):
                with self.assertRaisesRegex(TimeoutError, "visibility"):
                    controller.open_pull()


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
