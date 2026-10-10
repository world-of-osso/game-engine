"""Concrete offline closure fixtures; no install, extractor, or network required."""
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import sys
import sqlite3
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "asset_closure", Path(__file__).parents[1] / "asset_closure.py"
)


def chunk(tag, payload, reverse=False):
    return (tag[::-1] if reverse else tag).encode() + struct.pack("<I", len(payload)) + payload


def ints(*values):
    return struct.pack("<" + "I" * len(values), *values)


def model(*chunks):
    return chunk("MD21", b"MD20" + bytes(0x134)) + b"".join(chunks)


class ClosureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tool = importlib.util.module_from_spec(SPEC)
        SPEC.loader.exec_module(cls.tool)

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.data = Path(self.tmp.name)
        self.paths = {1: "world/maps/azeroth/azeroth_32_48_obj0.adt",
                      2: "world/a.m2", 3: "world/abbey.wmo", 4: "world/abbey_000.wmo",
                      5: "world/a00.skin", 6: "world/a.skel", 7: "world/a.anim",
                      8: "textures/a.blp", 9: "world/b.m2"}

    def put(self, path, data):
        dest = self.data / path
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(data)

    def closure(self, fdid=1, kind="adt"):
        tool = self.tool.Closure(self.data, self.paths, product="wow", metadata_build="fixture")
        tool.add(fdid, kind, "fixture seed")
        return tool.run()

    def test_binary_chains_reach_fixed_point_and_preserve_all_edges(self):
        mddf = bytearray(36)
        struct.pack_into("<I", mddf, 0, 2)
        struct.pack_into("<H", mddf, 34, 0x40)
        modf = bytearray(64)
        struct.pack_into("<I", modf, 0, 3)
        struct.pack_into("<H", modf, 56, 0x8)
        self.put("terrain/1.adt", chunk("MDDF", mddf, True) + chunk("MODF", modf, True))
        self.put("models/2.m2", model(chunk("SFID", ints(5)), chunk("SKID", ints(6)), chunk("TXID", ints(8))))
        self.put("models/200.skin", b"SKINfixture")
        self.put("models/2.skel", chunk("AFID", struct.pack("<HHI", 1, 0, 7)))
        self.put("models/7.anim", b"AFSBfixture")
        mat = bytearray(64)
        struct.pack_into("<I", mat, 12, 8)
        self.put("models/3.wmo", chunk("GFID", ints(4), True) + chunk("MOMT", mat, True) + chunk("MODI", ints(9), True))
        self.put("models/4.wmo", chunk("MVER", ints(17), True))
        self.put("models/9.m2", model(chunk("TXID", ints(8))))
        self.put("textures/8.blp", b"BLP2fixture")
        first = self.closure()
        self.assertEqual([a["fdid"] for a in first["assets"]], list(range(1, 10)))
        texture = next(a for a in first["assets"] if a["fdid"] == 8)
        self.assertEqual(len(texture["edges"]), 3)
        self.assertEqual(texture["sha256"], hashlib.sha256(b"BLP2fixture").hexdigest())
        self.assertEqual(first["summary"]["missing"], 0)
        self.assertEqual(first, self.closure())
        self.assertIsNone(texture["actual_build"])
        self.assertEqual(texture["identity_status"], "unverified")

    def test_missing_model_is_retained_and_marks_unexpanded_boundary(self):
        self.put("models/2.m2", model(chunk("SKID", ints(6))))
        result = self.closure(2, "m2")
        self.assertEqual(result["summary"]["missing"], 1)
        self.assertTrue(any(u["code"] == "missing_dependency_bytes" and u["fdid"] == 6 for u in result["unresolved"]))

    def test_named_adt_and_wmo_dependencies(self):
        mddf = bytearray(36)
        self.put("terrain/1.adt", chunk("MMDX", b"world/a.m2\0", True) + chunk("MMID", ints(0), True) + chunk("MDDF", mddf, True))
        self.put("models/2.m2", model())
        result = self.closure()
        self.assertEqual([a["fdid"] for a in result["assets"]], [1, 2])

    def test_malformed_chunk_and_unknown_named_edge_are_not_dropped(self):
        self.put("models/2.m2", b"MD21" + ints(100) + b"short")
        result = self.closure(2, "m2")
        self.assertTrue(any(u["code"] == "parse_error" for u in result["unresolved"]))
        self.put("terrain/1.adt", chunk("MTEX", b"textures/unknown.blp\0", True))
        result = self.closure()
        self.assertTrue(any(u["code"] == "unmapped_path" for u in result["unresolved"]))

    def test_skeleton_parent_cycle_terminates(self):
        self.put("models/6.skel", chunk("SKPD", ints(0, 0, 7, 0)))
        self.put("models/7.skel", chunk("SKPD", ints(0, 0, 6, 0)))
        result = self.closure(6, "skel")
        self.assertEqual([a["fdid"] for a in result["assets"]], [6, 7])

    def test_catalog_joins_display_item_and_spell_icons(self):
        sys.path.insert(0, str(Path(__file__).parents[1]))
        self.addCleanup(sys.path.pop, 0)
        import closure_seeds
        for name, text in {
            "CreatureDisplayInfo": "ID,ModelID,TextureVariationFileDataID_0\n10,20,8\n",
            "CreatureModelData": "ID,FileDataID\n20,2\n",
            "ItemDisplayInfo": "ID,ModelResourcesID_0,ModelMaterialResourcesID_0\n30,40,50\n",
            "ModelFileData": "FileDataID,ModelResourcesID\n9,40\n",
            "TextureFileData": "FileDataID,MaterialResourcesID\n8,50\n",
            "ItemDisplayInfoMaterialRes": "ID,ItemDisplayInfoID,MaterialResourcesID\n1,30,50\n",
            "SpellMisc": "ID,SpellID,DifficultyID,SpellIconFileDataID,ActiveIconFileDataID\n1,100,0,8,0\n",
        }.items():
            self.put(f"db2/fixture/{name}.csv", text.encode())
        graph = self.tool.Closure(self.data, self.paths, "wow", "fixture")
        catalogs = closure_seeds.Catalogs(graph)
        closure_seeds.seed_displays(graph, catalogs, [10])
        db = sqlite3.connect(":memory:")
        self.addCleanup(db.close)
        db.execute("create table content_item (ID integer, DisplayInfoID integer)")
        db.execute("insert into content_item values (200,30)")
        closure_seeds.seed_items(graph, catalogs, db, [200])
        closure_seeds.seed_icons(graph, catalogs, [100])
        result = graph.run()
        self.assertEqual([a["fdid"] for a in result["assets"]], [2, 8, 9])
        self.assertEqual(len(next(a for a in result["assets"] if a["fdid"] == 8)["edges"]), 3)
        self.assertFalse(any(u["code"] == "missing_metadata_row" for u in result["unresolved"]))

    def test_catalog_size_census_labels_missing_bytes_as_estimates(self):
        self.put("textures/8.blp", b"BLP2fixture")
        result = self.tool.estimate_catalog(self.data, self.paths)
        self.assertEqual(result["classes"]["blp"]["listfile_files"], 1)
        self.assertEqual(result["classes"]["blp"]["local_files"], 1)
        self.assertEqual(result["classes"]["blp"]["local_bytes"], 11)
        self.assertEqual(result["classes"]["m2"]["missing_file_estimate"], 2)
        self.assertIsNone(result["classes"]["m2"]["missing_bytes_estimate"])

    def test_wmo_extended_shader_materials_are_dependencies(self):
        material = bytearray(64)
        struct.pack_into("<I", material, 4, 20)
        struct.pack_into("<I", material, 60, 8)
        self.put("models/3.wmo", chunk("MOMT", material, True))
        result = self.closure(3, "wmo")
        self.assertEqual([a["fdid"] for a in result["assets"]], [3, 8])

    def test_different_alias_bytes_are_not_accepted(self):
        self.put("models/2.m2", model(chunk("SFID", ints(5))))
        self.put("models/200.skin", b"SKINone")
        self.put("models/5.skin", b"SKINtwo")
        result = self.closure(2, "m2")
        self.assertTrue(any(u["code"] == "conflicting_local_bytes" for u in result["unresolved"]))


if __name__ == "__main__":
    unittest.main()
