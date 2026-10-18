"""Run24 retained route/positions, with explicitly synthetic coverage/native projection controls."""
import copy
import json
import sys
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from run_stockade_bot import StockadeRun
from bot_controller import PlayerController
from bot_pull_corridor import require_safe_corridor

FIXTURE = Path(__file__).parent / 'fixtures/bot_harness/native-stockade-pull-corridor.json'


class SourceControl:
    def population(self, tank):
        return {"hideable_entries": [46383], "max_radius": 14.5}

    def radius(self, creature, tank):
        return creature.get("radius", 14.5)


def unit(identity, entry, position, **fields):
    return {"id": identity, "entry": entry, "position": position, "health": 100,
            "combat": False, "hostile": True, **fields}


class PullCorridorTests(unittest.TestCase):
    def controller(self, directory):
        f = json.loads(FIXTURE.read_text())
        run = StockadeRun.__new__(StockadeRun)
        run.output = Path(directory)
        run.run = {"openings": []}
        run.opening_target = None
        run.tank = "Captain Garrick"
        run.party_actor_ids = {run.tank: str(f['tank'])}
        run.pull_sources = SourceControl()
        run.account = lambda: {"world_map_id": 34}
        run.party = lambda: []
        run.poll_combat = lambda: None
        state = {"at_ms": int(time.time() * 1000), "actor": 7, "target": f['target'],
                 "target_position": f['target_position'], "assisted": {"in_combat": False},
                 "spells": {"known": [57755], "target_health": 100},
                 "facts": {"known": [57755], **{k: {"57755": {"Ok": True}}
                           for k in ['ready', 'affordable', 'in_range']}}}
        units = [unit(f['tank'], 209057, f['route'][0], hostile=False),
                 unit(f['target'], 46382, f['target_position']),
                 unit(99, 46383, [80, -25, 1], combat=False)]
        units += [unit(g['id'], 46382, g['position']) for g in f['guards']]
        native = {"epoch": [34, 1, 1], "full_update_after_entry": True,
                  "units": units, "observed_dying_entries": []}
        def publish():
            state['pull_units'] = {"state_json": json.dumps(native)}
        publish()
        run.rotation = lambda: state
        run.snapshot = lambda target: state
        queries, casts = [], []
        run.query_route = lambda here, goal, label: queries.append((here, goal)) or f['route']
        run.approach_opening = lambda target: None
        def cast_opening(target, spell, admission=None):
            if admission:
                admission(state, target)
            casts.append((target, spell))
            return {"success": True}
        run.player = SimpleNamespace(tab_to_enemy=lambda: f['target'], cast_opening=cast_opening,
                                     pending=None)
        return run, native, state, publish, queries, casts

    def test_retained_run24_corridor_refuses_before_opening_cast(self):
        with tempfile.TemporaryDirectory() as directory:
            run, native, state, publish, queries, casts = self.controller(directory)
            with self.assertRaisesRegex(ValueError, 'corridor.*hostile'):
                run.open_pull()
            self.assertEqual(casts, [])
            self.assertIsNone(run.opening_target)

    def test_safe_separated_pull_casts_once_and_keeps_selected_target(self):
        with tempfile.TemporaryDirectory() as directory:
            run, native, state, publish, queries, casts = self.controller(directory)
            native['units'] = native['units'][:3]
            publish()
            run.open_pull()
            self.assertEqual(casts, [(state['target'], 57755)])
            self.assertEqual(run.opening_target, state['target'])

    def test_unknown_stale_or_unseen_hideable_population_refuses(self):
        for case in ['missing', 'no_update', 'unseen', 'gone', 'outside', 'stale']:
            with self.subTest(case=case), tempfile.TemporaryDirectory() as directory:
                run, native, state, publish, queries, casts = self.controller(directory)
                native['units'] = native['units'][:3]
                if case == 'no_update':
                    native['full_update_after_entry'] = False
                elif case in ('unseen', 'gone'):
                    native['units'] = native['units'][:2]
                elif case == 'outside':
                    run.query_route = lambda *args: [[0, 0, 0], [101, 0, 0]]
                publish()
                if case == 'missing':
                    del state['pull_units']
                if case == 'stale':
                    state['at_ms'] = -10000
                with patch('run_stockade_bot.time.time', return_value=1):
                    with self.assertRaises((ValueError, RuntimeError, KeyError)):
                        run.open_pull()
                self.assertEqual(casts, [])

    def test_observed_dying_template_satisfies_missing_hideable_v2(self):
        with tempfile.TemporaryDirectory() as directory:
            run, native, state, publish, queries, casts = self.controller(directory)
            native['units'] = native['units'][:2]
            native['observed_dying_entries'] = [46383]
            publish()
            run.open_pull()
            self.assertEqual(len(casts), 1)

    def test_dead_or_engaged_other_hostile_not_a_new_pack(self):
        for field in [{'health': 0}, {'combat': True}]:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                run, native, state, publish, queries, casts = self.controller(directory)
                for other in native['units'][3:]:
                    other.update(field)
                publish()
                run.open_pull()
                self.assertEqual(len(casts), 1)

    def test_segment_crossing_strict_boundary_and_vertical_distance(self):
        snapshot = {'epoch': [34, 1, 1], 'full_update_after_entry': True,
                    'observed_dying_entries': [46383], 'units': [
                        unit(1, 209057, [0, 0, 0], hostile=False),
                        unit(2, 46382, [40, 0, 0]),
                        unit(3, 46382, [20, 0, 0], radius=5)]}
        path = [[0, 0, 0], [40, 0, 0]]
        with self.assertRaisesRegex(ValueError, 'corridor.*hostile'):
            require_safe_corridor(snapshot, 1, 2, path, SourceControl())
        snapshot['units'][2]['position'] = [20, 5, 0]
        require_safe_corridor(snapshot, 1, 2, path, SourceControl())  # strict distance < radius
        snapshot['units'][2]['position'] = [20, 4.99, 0]
        with self.assertRaises(ValueError):
            require_safe_corridor(snapshot, 1, 2, path, SourceControl())
        snapshot['units'][2]['position'] = [20, 8, 0]
        require_safe_corridor(snapshot, 1, 2, path, SourceControl())

    def test_changed_tank_or_target_during_nav_query_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            run, native, state, publish, queries, casts = self.controller(directory)
            original = run.query_route
            def move_tank(*args):
                corners = original(*args)
                native['units'][0]['position'] = [110, -25.6, 3]
                publish()
                return corners
            run.query_route = move_tank
            with self.assertRaisesRegex(ValueError, 'changed'):
                run.open_pull()
            self.assertEqual(casts, [])

    def test_pending_ack_prevents_admission_or_cast(self):
        calls = []
        player = PlayerController(lambda *args: calls.append(args), lambda target: self.fail('snapshot before ACK'),
                                  None, None)
        player.pending = {"spell": 1464}
        with self.assertRaisesRegex(RuntimeError, 'outstanding'):
            player.cast_opening(42, 57755)
        self.assertEqual(calls, [])


if __name__ == '__main__':
    unittest.main()
