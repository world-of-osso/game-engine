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

    def test_dbd_names_keep_existing_csv_contract_without_guessing_field_indexes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'CooldownSet.dbd').write_text(
                'COLUMNS\nint ID\nint ChrSpecialization\nint PlayerConditionID\n\n'
                'LAYOUT ABCD1234\n$noninline,id$ID<32>\n'
                'PlayerConditionID<u32>\nChrSpecialization<u32>\n')
            source = root / 'input.db2'
            source.write_bytes(fixture(0xABCD1234, [8, 71]))
            output = root / 'out.csv'
            result = subprocess.run([sys.executable, str(ROOT / 'scripts/export_db2_csv.py'),
                                     'CooldownSet', str(source), str(output), '--definitions', str(root)],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            with output.open() as stream:
                self.assertEqual(list(csv.DictReader(stream)), [
                    {'ID': '1', 'PlayerConditionID': '8', 'ChrSpecializationID': '71'}])

    def test_pallet_array_masks_each_narrow_value_before_signedness(self):
        import struct
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'PlayerCondition.dbd').write_text(
                'COLUMNS\nint ID\nint MinSkill\n\nLAYOUT ABCD1234\n'
                '$noninline,id$ID<32>\nMinSkill<u16>[2]\n')
            data = bytearray(fixture(0xABCD1234, [0]))
            start = struct.unpack_from('<I', data, 212)[0]
            palette = struct.pack('<2I', 131072, 1310882)
            data[start:start] = palette
            struct.pack_into('<I', data, 212, start + 8)
            struct.pack_into('<I', data, 224, start + 12)
            struct.pack_into('<I', data, 196, 8)
            struct.pack_into('<HH5I', data, 248, 0, 32, 8, 4, 0, 0, 2)
            source = root / 'input.db2'
            source.write_bytes(data)
            output = root / 'out.csv'
            result = subprocess.run([sys.executable, str(ROOT / 'scripts/export_db2_csv.py'),
                                     'PlayerCondition', str(source), str(output), '--definitions', str(root)],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            with output.open() as stream:
                self.assertEqual(list(csv.DictReader(stream)), [
                    {'ID': '1', 'MinSkill_0': '0', 'MinSkill_1': '162'}])

    def test_installed_player_condition_scalar_pallets_follow_scalar_block(self):
        import gzip
        import hashlib
        import json

        fixture_root = ROOT / 'scripts/tests/fixtures/playercondition-12.1.5.70391'
        receipt = json.loads((fixture_root / 'receipt.json').read_text())
        payload = gzip.decompress((fixture_root / '1045411.db2.gz').read_bytes())
        self.assertEqual(hashlib.sha256(payload).hexdigest(), receipt['raw_sha256'])
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'PlayerCondition.db2'
            source.write_bytes(payload)
            output = Path(directory) / 'PlayerCondition.csv'
            result = subprocess.run([
                sys.executable, str(ROOT / 'scripts/export_db2_csv.py'),
                'PlayerCondition', str(source), str(output),
                '--definitions', str(fixture_root)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            with output.open() as stream:
                row = next(row for row in csv.DictReader(stream) if row['ID'] == '923')
            self.assertEqual(row['WeaponSubclassMask'], '0')
            self.assertEqual(row['ContentTuningID'], '0')
            self.assertEqual(row['MovementFlags'], '0')

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
