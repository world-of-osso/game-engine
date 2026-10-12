"""Real local-CASC Retail69933 DB2 exports; TRANSMOG_DB2_DIR is required."""
import csv
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

EXPORTER = Path(__file__).resolve().parents[1] / 'export_db2_csv.py'

class TransmogDb2Tests(unittest.TestCase):
    def test_authored_transmog_table_identities_and_membership(self):
        cases = [
            ('TransmogSet', 1376213, '13', {'Name_lang': "Wild Combatant's Chain Armor", 'ParentTransmogSetID':'80'}),
            ('TransmogSetItem', 1376212, '118', {'TransmogSetID':'13', 'ItemModifiedAppearanceID':'70594'}),
            ('TransmogIllusion', 3580450, '3', {'TransmogCost':'10000', 'SpellItemEnchantmentID':'1898'}),
            ('TransmogOutfitEntry', 7320164, '2', {'Name_lang':'Outfit 1', 'Flags':'35', 'Cost':'0'}),
            ('TransmogOutfitSlotInfo', 7300046, '13', {'InventorySlotName':'MAINHANDSLOT', 'Flags':'3'}),
            ('TransmogOutfitSlotOption', 7299793, '1', {'Name_lang':'One Handed Weapon', 'TransmogOutfitSlotInfoID':'13'}),
            ('SpellItemEnchantment', 1362771, '1898', {'Name_lang':'Lifestealing', 'IconFileDataID':'136130'}),
        ]
        source = Path(os.environ['TRANSMOG_DB2_DIR'])
        with tempfile.TemporaryDirectory() as directory:
            for table, fdid, identity, expected in cases:
                with self.subTest(table=table):
                    output = Path(directory) / f'{table}.csv'
                    run = subprocess.run([sys.executable, str(EXPORTER), table, str(source / f'{fdid}.db2'), str(output)], capture_output=True, text=True)
                    self.assertEqual(run.returncode, 0, run.stderr)
                    with output.open() as stream:
                        rows = {row['ID']:row for row in csv.DictReader(stream)}
                    for field, value in expected.items():
                        self.assertEqual(rows[identity][field], value)

if __name__ == '__main__':
    unittest.main()
