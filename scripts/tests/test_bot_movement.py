import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run_bot_roster


class BotMovementTests(unittest.TestCase):
    def test_normal_movement_approaches_without_running_through_dummy(self):
        here = [-8985.0, 79.55, 179.0]
        target = [-8969.405, 81.58, 159.34814]
        step = run_bot_roster.approach_step(here, target)
        self.assertEqual(step["seconds"], .4)
        self.assertAlmostEqual(step["yaw_degrees"], 141.56576516, places=3)
        self.assertIsNone(run_bot_roster.approach_step([0, 5, 0], [0, 5, 2]))
        step = run_bot_roster.approach_step([0, 5, 0], [0, 5, 3])
        self.assertAlmostEqual(step["seconds"] * 7, .5)


if __name__ == "__main__":
    unittest.main()
