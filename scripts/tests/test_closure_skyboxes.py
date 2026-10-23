"""Authored map lighting roots must expose their M2 descendants offline."""

from pathlib import Path
import sqlite3
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from asset_closure import RawByteGraph
import closure_seeds


class SkyboxRootsTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.data = Path(directory.name)
        metadata = self.data / "db2/fixture"
        metadata.mkdir(parents=True)
        (metadata / "Light.csv").write_text(
            "ID,ContinentID,LightParamsID_0,LightParamsID_7\n"
            "1,2991,12,13\n2,0,14,0\n3,2991,12,0\n"
        )
        (metadata / "LightParams.csv").write_text(
            "ID,LightSkyboxID\n12,683\n13,684\n14,685\n15,0\n"
        )
        (metadata / "LightSkybox.csv").write_text(
            "ID,SkyboxFileDataID,CelestialSkyboxFileDataID\n"
            "683,7345733,0\n684,0,7000001\n685,7000002,0\n"
        )
        (self.data / "models").mkdir()
        raw = b"MD20" + bytes(124)
        (self.data / "models/7345733.m2").write_bytes(
            b"MD21" + struct.pack("<I", len(raw)) + raw
            + b"SFID" + struct.pack("<II", 4, 7345740)
        )

    def graph(self):
        return RawByteGraph(self.data, {}, "wow_classic_beta", "fixture")

    def test_selected_map_all_slots_expose_skybox_skin_and_celestial(self):
        graph = self.graph()
        closure_seeds.seed_skyboxes(graph, closure_seeds.Catalogs(graph), {2991})
        assets = {(a["fdid"], a["type"]): a for a in graph.run()["assets"]}
        self.assertEqual(
            set(assets), {(7345733, "m2"), (7345740, "skin"), (7000001, "m2")}
        )
        self.assertTrue(assets[7345733, "m2"]["present"])
        self.assertFalse(assets[7345740, "skin"]["present"])

    def test_positive_missing_joins_remain_explicit_but_zero_is_no_request(self):
        (self.data / "db2/fixture/Light.csv").write_text(
            "ID,ContinentID,LightParamsID_0,LightParamsID_7\n"
            "1,2991,999,13\n2,2991,15,0\n"
        )
        (self.data / "db2/fixture/LightParams.csv").write_text(
            "ID,LightSkyboxID\n13,998\n15,0\n"
        )
        graph = self.graph()
        closure_seeds.seed_skyboxes(graph, closure_seeds.Catalogs(graph), {2991})
        result = graph.run()
        reasons = {
            row["reason"]
            for row in result["unresolved"]
            if row["code"] == "missing_metadata_row"
        }
        self.assertEqual(reasons, {"LightParams ID=999", "LightSkybox ID=998"})
        self.assertEqual(result["assets"], [])

    def test_catalog_entrypoint_seeds_global_map_without_terrain_tiles(self):
        db = sqlite3.connect(self.data / "world.db")
        db.executescript("""
            CREATE TABLE content_creature(guid,id1,id2,id3,modelid,position_x,position_y,map);
            CREATE TABLE content_creature_template_model(CreatureID,CreatureDisplayID,Idx);
            CREATE TABLE content_gameobject_template(entry,type,displayId);
            CREATE TABLE content_item(ID,DisplayInfoID);
            CREATE TABLE spell_name(ID);
        """)
        db.close()
        graph = self.graph()
        closure_seeds.seed_catalogs(
            graph,
            self.data / "world.db",
            {
                "maps": [{"id": 2991, "directory": "skyborne", "tiles": []}],
                "characters": [],
                "npc_displays": "all",
                "game_objects": "all",
                "items": [],
                "spells": [],
                "customization_cache": "cache/customization-v5.sqlite",
            },
        )
        self.assertIn(
            (7345740, "skin"),
            {(a["fdid"], a["type"]) for a in graph.run()["assets"]},
        )


if __name__ == "__main__":
    unittest.main()
