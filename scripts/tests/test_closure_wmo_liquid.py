"""WMO liquid closure follows root flags/group LiquidType, not MLIQ's MOMT index."""

from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
from asset_closure import Closure


def chunk(tag, payload):
    return tag[::-1].encode() + struct.pack("<I", len(payload)) + payload


def root(group, flags):
    h = bytearray(64)
    struct.pack_into("<I", h, 4, 1)
    struct.pack_into("<H", h, 60, flags)
    return chunk("MOHD", h) + chunk("GFID", struct.pack("<I", group))


def group(liquid, flags=0, hidden=False):
    h = bytearray(68)
    struct.pack_into("<I", h, 8, flags)
    struct.pack_into("<I", h, 52, liquid)
    water = (
        struct.pack("<4i3fh", 2, 2, 1, 1, 0, 0, 0, 999)
        + b"\0" * 32
        + bytes([15 if hidden else 0])
    )
    return chunk("MOGP", h + chunk("MLIQ", water))


class WmoLiquidTests(unittest.TestCase):
    def make_graph(self, data):
        p = data / "db2/fixture"
        p.mkdir(parents=True)
        (p / "LiquidType.csv").write_text(
            "ID,MaterialID\n13,1\n14,1\n19,2\n20,4\n21,1\n"
        )
        (p / "LiquidMaterial.csv").write_text("ID\n1\n2\n4\n")
        (p / "LiquidTypeXTexture.csv").write_text(
            "ID,LiquidTypeID,FileDataID\n1,13,101\n2,14,102\n3,19,103\n4,20,104\n5,21,105\n"
        )
        return Closure(data, {}, "wow", "fixture")

    def test_shared_group_with_two_root_flag_contexts(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            (d / "models").mkdir()
            # Group zero is basic water in legacy mode, slime in DBC mode
            # (wrapping_sub(1) & 3), as core wmo_liquid::group_liquid_type.
            (d / "models/1.wmo").write_bytes(root(3, 0))
            (d / "models/2.wmo").write_bytes(root(3, 4))
            (d / "models/3.wmo").write_bytes(group(0))
            g = self.make_graph(d)
            g.add(1, "wmo", "seed")
            g.add(2, "wmo", "seed")
            r = g.run()
            self.assertIn((101, "blp"), g.assets)
            self.assertIn((104, "blp"), g.assets)
            self.assertIn((768431, "blob"), g.assets)
            self.assertFalse(
                any(i["code"] == "wmo_liquid_edges" for i in r["unresolved"])
            )

    def test_green_lava_makes_no_texture_request(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            (d / "models").mkdir()
            (d / "models/1.wmo").write_bytes(root(2, 0))
            (d / "models/2.wmo").write_bytes(group(15))
            g = self.make_graph(d)
            g.add(1, "wmo", "seed")
            r = g.run()
            self.assertFalse(any(kind in {"blp", "blob"} for _, kind in g.assets))
            self.assertFalse(
                any(i["code"] == "wmo_liquid_edges" for i in r["unresolved"])
            )

    def test_hidden_tiles_still_load_material_before_geometry(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            (d / "models").mkdir()
            (d / "models/1.wmo").write_bytes(root(2, 0))
            (d / "models/2.wmo").write_bytes(group(20, hidden=True))
            g = self.make_graph(d)
            g.add(1, "wmo", "seed")
            g.run()
            self.assertIn((105, "blp"), g.assets)

    def test_orphan_group_cannot_guess_root_flags(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            (d / "models").mkdir()
            (d / "models/1.wmo").write_bytes(group(0))
            g = self.make_graph(d)
            g.add(1, "wmo", "seed")
            r = g.run()
            self.assertTrue(
                any(
                    i["code"] == "wmo_liquid_edges" and "root" in i["reason"]
                    for i in r["unresolved"]
                )
            )


if __name__ == "__main__":
    unittest.main()
