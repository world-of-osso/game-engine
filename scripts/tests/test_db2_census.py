import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from db2_census import compare_rows


class CensusTest(unittest.TestCase):
    def test_changes_distinguish_schema_expansion_numeric_rendering_and_meaning(self):
        old = {'1': {'ID': '1', 'Name': 'Before', 'Scale': '1.0'},
               '2': {'ID': '2', 'Name': 'Same', 'Scale': '0.600000024'},
               '3': {'ID': '3', 'Name': 'Gone', 'Scale': '1'}}
        new = {'1': {'ID': '1', 'Name': 'After', 'Scale': '1', 'Added': '5'},
               '2': {'ID': '2', 'Name': 'Same', 'Scale': '0.6000000238418579', 'Added': '7'},
               '4': {'ID': '4', 'Name': 'New', 'Scale': '2', 'Added': '1'}}
        diff = compare_rows(old, new)
        self.assertEqual(diff['added_ids'], [4])
        self.assertEqual(diff['removed_ids'], [3])
        self.assertEqual(diff['changed_ids'], [1])
        self.assertEqual(diff['changed_fields']['1'], ['Name'])
        self.assertEqual(diff['added_columns'], ['Added'])


if __name__ == '__main__':
    unittest.main()
