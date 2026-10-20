"""Azure205 admission and encounter evidence must not match diagnostic map34/Normal."""
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_stockade import admitted_copy, encounter_states
from run_stockade_bot import StockadeRun


class AzureEvidenceTests(unittest.TestCase):
    def test_admission_uses_requested_map_and_difficulty(self):
        text = ("Created instance 7 of map 34 at difficulty 1 for InstanceEntrant { character_id: 42,\n"
                "Created instance 8 of map 2515 at difficulty 205 for InstanceEntrant { character_id: 42,\n"
                "Created instance 9 of map 2515 at difficulty 1 for InstanceEntrant { character_id: 42,\n")
        self.assertEqual(admitted_copy(text, 42, 2515, 205), 8)
        with self.assertRaises(ValueError):
            admitted_copy(text, 43, 2515, 205)

    def test_credits_remain_copy_and_map_scoped(self):
        text = ("Instance InMap { map_id: 34, instance_id: 8 }: boss 186644 (encounter 2582) InProgress -> Done\n"
                "Instance InMap { map_id: 2515, instance_id: 9 }: boss 186644 (encounter 2582) InProgress -> Done\n"
                "Instance InMap { map_id: 2515, instance_id: 8 }: boss 186644 (encounter 2582) NotStarted -> InProgress\n")
        self.assertEqual(encounter_states(text, 8, 2515), {2582: 'InProgress'})

    def test_readiness_requires_azure_owner_not_stockade(self):
        runner = StockadeRun.__new__(StockadeRun)
        runner.args = SimpleNamespace(character='Azfury', spec=72)
        runner.plan = {'map': 2515, 'difficulty': 205}
        account = {'selected_character_name': 'Azfury', 'screen': 'InWorld',
                   'world_map_id': 34, 'local_player_health': 100}
        state = {'spells': {'catalog_ready': True, 'spec': 72}}
        self.assertFalse(runner.owner_ready(account, state))
        account['world_map_id'] = 2515
        self.assertTrue(runner.owner_ready(account, state))


if __name__ == '__main__':
    unittest.main()
