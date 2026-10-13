import csv
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]

class GlueAtlasSlices(unittest.TestCase):
    def test_export_local_dialog_and_camp_slice_margins(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'slices.csv'
            result = subprocess.run([sys.executable, str(ROOT / 'scripts/export_db2_csv.py'), 'UiTextureAtlasElementSliceData', str(ROOT / 'scripts/tests/fixtures/ui-atlas-slices-12.1.5.70077.db2'), str(target)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            with target.open() as stream:
                rows = {int(row['UiTextureAtlasElementID']): row for row in csv.DictReader(stream)}
            self.assertEqual([int(rows[30482][key]) for key in ['Left', 'Top', 'Right', 'Bottom', 'SliceMode']], [32, 32, 32, 32, 0])
            self.assertEqual([int(rows[29533][key]) for key in ['Left', 'Top', 'Right', 'Bottom', 'SliceMode']], [12, 12, 12, 12, 0])

if __name__ == '__main__':
    unittest.main()
