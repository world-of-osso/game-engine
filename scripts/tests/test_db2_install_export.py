"""DBD-selected export follows the payload layout rather than a former build."""
import csv
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from test_cache_import_exports import fixture

ROOT = Path(__file__).resolve().parents[2]


class InstallExportTest(unittest.TestCase):
    def test_new_layout_reorders_fields_and_preserves_signed_array_values(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            definitions = root / 'definitions'
            definitions.mkdir()
            (definitions / 'CharComponentTextureLayouts.dbd').write_text(
                'COLUMNS\nint ID\nint Width\nint Height\nint Offset\n\n'
                'LAYOUT ABCD1234\nBUILD 12.1.5.70340\n'
                '$noninline,id$ID<32>\nHeight<u32>\nWidth<u32>\nOffset<16>[2]\n')
            source = root / 'input.db2'
            source.write_bytes(fixture(0xABCD1234, [1024, 2048, 0x0003FFFF]))
            output = root / 'out.csv'
            result = subprocess.run([
                sys.executable, str(ROOT / 'scripts/export_db2_csv.py'),
                'CharComponentTextureLayouts', str(source), str(output),
                '--definitions', str(definitions)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            with output.open() as stream:
                self.assertEqual(list(csv.DictReader(stream)), [
                    {'ID': '1', 'Height': '1024', 'Width': '2048',
                     'Offset_0': '-1', 'Offset_1': '3'}])

    def test_unknown_layout_fails_without_writing_export(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'CharComponentTextureLayouts.dbd').write_text(
                'COLUMNS\nint ID\nLAYOUT 11223344\n$noninline,id$ID<32>\n')
            source = root / 'input.db2'
            source.write_bytes(fixture(0xABCD1234, [1024]))
            output = root / 'out.csv'
            result = subprocess.run([
                sys.executable, str(ROOT / 'scripts/export_db2_csv.py'),
                'CharComponentTextureLayouts', str(source), str(output),
                '--definitions', str(root)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
