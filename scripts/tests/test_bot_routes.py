"""Real query-process output is the route contract; no source-text assertions."""
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_controller import PlayerController
from bot_routes import NavmeshRoute


class RouteQueryTests(unittest.TestCase):
    def test_query_preserves_actual_corners_and_raw_failure_receipts(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            query = NavmeshRoute([sys.executable, '-c', 'print("0,0,0\\n7,0,2\\n21,0,0")'],
                                 output / 'navmesh', output)
            self.assertEqual(query([0, 0, 0], [21, 0, 0], 'authored-leg'),
                             [[0., 0., 0.], [7., 0., 2.], [21., 0., 0.]])
            query = NavmeshRoute([sys.executable, '-c', 'import sys;print("unbaked",file=sys.stderr);sys.exit(4)'],
                                 output / 'navmesh', output)
            with self.assertRaisesRegex(RuntimeError, 'unbaked'):
                query([0, 0, 0], [21, 0, 0], 'party-approach')
            receipts = [json.loads(p.read_text()) for p in output.glob('route-*.json')]
            self.assertEqual(sorted(r['exit'] for r in receipts), [0, 4])

    def test_empty_or_malformed_query_output_blocks(self):
        with tempfile.TemporaryDirectory() as directory:
            for text in ['', 'nan,0,0', '1,2']:
                query = NavmeshRoute([sys.executable, '-c', f'print({text!r})'], Path(directory), Path(directory))
                with self.assertRaisesRegex(ValueError, 'route'):
                    query([0, 0, 0], [21, 0, 0], 'authored-leg')

    def test_routed_controller_reobserves_and_services_loot_during_busy_ticks(self):
        here = [0., 0., 0.]
        delivered = []
        steps = []
        class Bridge:
            def request(self, kind, payload):
                delivered.append(payload['name'])
                return payload
        def command(*argv):
            if argv[0] == 'dump-ui-tree':
                return '' if delivered else 'GroupLootFrame1GreedButton [Button] visible x=10 y=20 w=30 h=40'
            here[0] += float(argv[3]) * 7
            steps.append(list(here))
            return 'movement acknowledged'
        controller = PlayerController(command, None, lambda start, goal, label: [goal], Bridge())
        ticks = [True, False, False]
        with patch('bot_controller.time.sleep'):
            controller.walk_route([[7., 0., 0.]], lambda: list(here), lambda: ticks.pop(0), timeout=5)
        self.assertEqual(delivered, ['GroupLootFrame1GreedButton'])
        self.assertEqual(len(steps), 2)
        self.assertAlmostEqual(here[0], 7.)


if __name__ == '__main__':
    unittest.main()
