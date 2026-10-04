import importlib.util
import struct
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "zephras", Path(__file__).parents[1] / "import_forever_zephras.py"
)
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


def chunk(tag, payload):
    return tag + struct.pack("<I", len(payload)) + payload


class ClosureTests(unittest.TestCase):
    def test_maid_uses_only_active_tiles_and_runtime_companions(self):
        main = bytearray(4096 * 8)
        maid = bytearray(4096 * 32)
        struct.pack_into("<I", main, (29 * 64 + 26) * 8, 1)
        struct.pack_into("<8I", maid, (29 * 64 + 26) * 32, *range(100, 108))
        struct.pack_into("<8I", maid, 0, *range(200, 208))
        data = chunk(b"NIAM", main) + chunk(b"DIAM", maid)
        self.assertEqual(
            module.terrain_references(data),
            [
                (100, "terrain/100.adt"),
                (101, "terrain/101.adt"),
                (103, "terrain/103.adt"),
            ],
        )

    def test_m2_skin_alias_and_texture_animation_closure(self):
        data = (
            chunk(b"MD21", bytes(304))
            + chunk(b"SFID", struct.pack("<2I", 21, 22))
            + chunk(b"TXID", struct.pack("<2I", 30, 0))
            + chunk(b"AFID", struct.pack("<HHI", 0, 0, 40))
            + chunk(b"SKID", struct.pack("<I", 50))
        )
        refs = module.asset_references(data, 10, "models/10.m2", {}, {})
        self.assertIn((21, "models/1000.skin"), refs)
        self.assertIn((30, "textures/30.blp"), refs)
        self.assertIn((40, "models/40.anim"), refs)
        self.assertIn((50, "models/10.skel"), refs)

    def test_wmo_groups_materials_and_doodad_models(self):
        header = bytearray(64)
        struct.pack_into("<I", header, 4, 1)
        material = bytearray(64)
        struct.pack_into("<I", material, 12, 81)
        data = (
            chunk(b"REVM", struct.pack("<I", 17))
            + chunk(b"DHOM", header)
            + chunk(b"DIFG", struct.pack("<I", 71))
            + chunk(b"TMOM", material)
            + chunk(b"IDOM", struct.pack("<I", 91))
        )
        refs = module.asset_references(data, 61, "models/61.wmo", {}, {})
        self.assertIn((71, "models/71.wmo"), refs)
        self.assertIn((81, "textures/81.blp"), refs)
        self.assertIn((91, "models/91.m2"), refs)

    def test_wmo_shader_23_collects_all_nine_runtime_textures(self):
        header = bytearray(64)
        material = bytearray(64)
        struct.pack_into("<I", material, 4, 23)
        struct.pack_into("<I", material, 12, 80)
        struct.pack_into("<I", material, 24, 81)
        struct.pack_into("<I", material, 36, 82)
        struct.pack_into("<6I", material, 40, *range(83, 89))
        data = chunk(b"DHOM", header) + chunk(b"TMOM", material)
        refs = module.asset_references(data, 60, "models/60.wmo", {}, {})
        self.assertEqual(
            set(refs), {(fdid, f"textures/{fdid}.blp") for fdid in range(80, 89)}
        )

    def test_wrong_magic_is_not_published(self):
        with self.assertRaises(ValueError):
            module.validate_asset(b"WDC5garbage", "models/10.m2")


if __name__ == "__main__":
    unittest.main()
