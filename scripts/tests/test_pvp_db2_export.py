"""Behavioral exports against the pinned, locally recovered CASC records."""
import csv
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "export_db2_csv.py"
SOURCE = SCRIPT.parent / "tests/fixtures/pvp-12.1.0.69933"


class PvpExports(unittest.TestCase):
    def export(self, table, fdid):
        with tempfile.TemporaryDirectory() as tmp:
            target = Path(tmp) / "out.csv"
            subprocess.run([sys.executable, str(SCRIPT), table, str(SOURCE / f"{fdid}.db2"), str(target)], check=True)
            with target.open() as handle:
                return {int(row["ID"]): row for row in csv.DictReader(handle)}

    def test_local_currency_definitions(self):
        rows = self.export("CurrencyTypes", 1095531)
        self.assertEqual(rows[1792]["Name_lang"], "Honor")
        self.assertEqual(rows[1792]["MaxQty"], "15000")
        self.assertEqual(rows[1602]["Name_lang"], "Conquest")
        self.assertEqual(rows[1602]["MaxEarnablePerWeek"], "0")
        self.assertEqual(rows[1602]["MaxQtyWorldStateID"], "31270")

    def test_local_talents_slots_and_reward_identity(self):
        talents = self.export("PvpTalent", 1302853)
        self.assertEqual(len(talents), 407)
        self.assertEqual(talents[24]["SpecID"], "73")
        self.assertEqual(talents[24]["SpellID"], "236077")
        slots = self.export("PvpTalentSlotUnlock", 1950197)
        self.assertEqual([int(row["LevelRequired"]) for row in slots.values()], [10, 20, 30, 40])
        rewards = self.export("PrestigeLevelInfo", 1275869)
        self.assertEqual(rewards[1]["PrestigeLevel"], "5")
        self.assertEqual(rewards[1]["AwardedAchievementID"], "12893")


if __name__ == "__main__":
    unittest.main()
