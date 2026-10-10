"""Unlisted sounds identify their format from real extracted bytes, not extension guesses."""

from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
from asset_closure import Closure
from closure_seeds import Catalogs
from closure_spell_seeds import seed_sound_kits


class SoundIdentityTests(unittest.TestCase):
    def test_local_ogg_and_mp3_header_with_visible_unknown_record(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            (d / "db2/fixture").mkdir(parents=True)
            (d / "sounds").mkdir()
            (d / "db2/fixture/SoundKitEntry.csv").write_text(
                "ID,SoundKitID,FileDataID\n1,10,100\n2,10,101\n3,10,102\n"
            )
            (d / "sounds/100.audio").write_bytes(b"OggS" + b"\0" * 28)
            (d / "sounds/101.audio").write_bytes(b"ID3" + b"\0" * 30)
            (d / "sounds/102.audio").write_bytes(b"not audio")
            g = Closure(d, {}, "wow", "fixture")
            seed_sound_kits(g, Catalogs(g), {10})
            self.assertEqual(
                sorted(g.assets), [(100, "ogg"), (101, "mp3"), (102, "audio")]
            )
            self.assertEqual(
                [fdid for c, _, fdid in g.unresolved if c == "unknown_sound_file_type"],
                [102],
            )
            self.assertIn("sounds/100.audio", g.assets[(100, "ogg")]["locations"])
            self.assertIn("sounds/spells/100.ogg", g.assets[(100, "ogg")]["locations"])


if __name__ == "__main__":
    unittest.main()
