"""Ignored RED fixtures for native metadata-owned auditor roots; no runtime/CASC."""

from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import struct
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
from closure_receipts import ReceiptClosure
from closure_seeds import (
    Catalogs,
    seed_catalogs,
    seed_displays,
    seed_item_displays,
    seed_items,
)
from test_asset_closure import chunk, ints, model


@unittest.skipUnless(
    os.environ.get("P4_RED"), "Ignored RED fixture; enable P4_RED for the bounded proof"
)
class RootProductTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.data = Path(self.temporary.name)
        self.documents = {}
        self.keys = {"wow": "a" * 32, "wow_classic_beta": "b" * 32}
        self.config = {
            "product": "wow",
            "metadata_build": "fixture",
            "customization_cache": "cache/customization-v5.sqlite",
            "maps": [],
            "characters": [],
            "npc_displays": "all",
            "game_objects": "all",
            "items": [],
            "spells": [],
        }
        self.cache(
            "creature_display.sqlite",
            "CREATE TABLE creature_displays(display_id,model_fdid,skin_fdid_0,skin_fdid_1,skin_fdid_2,skin_fdid_3,source_product);",
        )
        self.cache(
            "outfit_links-v5.sqlite",
            "CREATE TABLE display_info(id,model_res_0,model_res_1,model_mat_res_0,model_mat_res_1); CREATE TABLE model_to_fdid(model_resource_id,model_order,file_data_id); CREATE TABLE material_textures(material_resource_id,file_order,texture_fdid); CREATE TABLE display_materials(display_info_id,component_section,material_resource_id); CREATE TABLE asset_resource_sources(table_name,resource_id,source_product);",
        )

    def put(self, relative, raw):
        path = self.data / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
        return path

    def cache(self, name, sql):
        path = self.data / "cache" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with closing(sqlite3.connect(path)) as db:
            db.executescript(sql)
            db.commit()
        return path

    def publish(self, product, fdid, kind, raw):
        key = self.keys[product]
        folder = "textures" if kind == "blp" else "models"
        relative = f"products/{product}/{key}/{folder}/{fdid}.{kind}"
        row = {
            "product": product,
            "build_key": key,
            "build": "fixture-build",
            "fdid": fdid,
            "kind": kind,
            "path": relative,
            "bytes": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest(),
            "content_key": hashlib.md5(raw).hexdigest(),
        }
        self.put(relative, raw)
        document = self.documents.setdefault(
            product, {"version": 1, "assets": [], "aliases": []}
        )
        document["assets"].append(row)
        self.put(
            f"cache/model-asset-index-{product}.json", json.dumps(document).encode()
        )

    def graph(self):
        graph = ReceiptClosure(self.data, {}, "wow", "fixture")
        graph.seeds = dict(self.config)
        return graph

    def empty_world(self):
        path = self.data / "world.db"
        with closing(sqlite3.connect(path)) as db:
            db.executescript(
                "CREATE TABLE content_creature(guid,id1,id2,id3,modelid,position_x,position_y); CREATE TABLE content_creature_template_model(CreatureDisplayID); CREATE TABLE content_gameobject_template(entry,type,displayId);"
            )
        return path

    def test_map_catalog_retail_id_and_casefolded_directory_conflicts_win(self):
        retail = b"ID,Directory,MapName_lang,WdtFileDataID\n1,retail,Retail,101\n"
        forever = b"ID,Directory,MapName_lang,WdtFileDataID\n1,replaced,ID collision,201\n2,RETAIL,Directory collision,202\n3,forever,Forever,301\n"
        self.put("db2/fixture/Map.csv", retail)
        self.put("db2/active/Map.csv", retail)
        self.put("db2/1.60.1.70205/Map.csv", forever)
        for wdt, adt in [(101, 501), (301, 601)]:
            self.put(
                f"terrain/{wdt}.wdt",
                chunk("MAID", struct.pack("<8I", adt, 0, 0, 0, 0, 0, 0, 0), True),
            )
            placement = bytearray(36)
            struct.pack_into("<I", placement, 0, 10)
            struct.pack_into("<H", placement, 34, 0x40)
            self.put(f"terrain/{adt}.adt", chunk("MDDF", placement, True))
        for product in self.keys:
            self.publish(product, 10, "m2", model())
        graph = self.graph()
        config = dict(self.config, maps="all")
        seed_catalogs(graph, self.empty_world(), config)
        result = graph.run()
        self.assertEqual(
            {
                (row["requested_product"], row["fdid"])
                for row in result["assets"]
                if row["type"] == "wdt"
            },
            {("wow", 101), ("wow_classic_beta", 301)},
        )
        self.assertEqual(
            {
                (row["requested_product"], row["fdid"])
                for row in result["assets"]
                if row["type"] == "m2"
            },
            {("wow", 10), ("wow_classic_beta", 10)},
        )
        self.assertTrue(all(row["fdid"] not in {201, 202} for row in result["assets"]))

    def test_display_cache_source_owns_model_and_variation_textures(self):
        self.cache(
            "creature_display.sqlite",
            "INSERT INTO creature_displays VALUES(100,10,12,0,0,0,'wow_classic_beta'),(200,10,22,0,0,0,'wow');",
        )
        self.put(
            "db2/fixture/CreatureDisplayInfo.csv",
            b"ID,ModelID,TextureVariationFileDataID_0\n100,1,999\n200,1,999\n",
        )
        self.put("db2/fixture/CreatureModelData.csv", b"ID,FileDataID\n1,999\n")
        for product, texture in [("wow_classic_beta", 12), ("wow", 22)]:
            self.publish(product, 10, "m2", model())
            self.publish(product, texture, "blp", b"BLP2" + product.encode())
        graph = self.graph()
        seed_displays(graph, Catalogs(graph), [100, 200])
        result = graph.run()
        self.assertEqual(
            {(row["requested_product"], row["fdid"]) for row in result["assets"]},
            {
                ("wow", 10),
                ("wow", 22),
                ("wow_classic_beta", 10),
                ("wow_classic_beta", 12),
            },
        )
        self.assertEqual(result["summary"]["missing"], 0)

    def test_equipment_model_and_material_keep_independent_resource_sources(self):
        self.cache(
            "outfit_links-v5.sqlite",
            "INSERT INTO display_info VALUES(30,50,0,60,0); INSERT INTO model_to_fdid VALUES(50,0,10); INSERT INTO material_textures VALUES(60,0,12); INSERT INTO asset_resource_sources VALUES('display_info',30,'wow_classic_beta'),('model_to_fdid',50,'wow'),('material_textures',60,'wow_classic_beta');",
        )
        self.put(
            "db2/fixture/ItemDisplayInfo.csv",
            b"ID,ModelResourcesID_0,ModelMaterialResourcesID_0\n30,50,60\n",
        )
        self.put(
            "db2/fixture/ModelFileData.csv", b"FileDataID,ModelResourcesID\n999,50\n"
        )
        self.put(
            "db2/fixture/TextureFileData.csv",
            b"FileDataID,MaterialResourcesID\n998,60\n",
        )
        self.publish("wow", 10, "m2", model())
        self.publish("wow_classic_beta", 12, "blp", b"BLP2owned")
        graph = self.graph()
        seed_item_displays(graph, Catalogs(graph), [30])
        result = graph.run()
        self.assertEqual(
            {(row["requested_product"], row["fdid"]) for row in result["assets"]},
            {("wow", 10), ("wow_classic_beta", 12)},
        )
        self.assertEqual(result["summary"]["missing"], 0)

    def test_owned_item_without_definition_source_remains_an_explicit_gap(self):
        self.cache(
            "outfit_links-v5.sqlite",
            "INSERT INTO display_info VALUES(30,50,0,0,0); INSERT INTO model_to_fdid VALUES(50,0,10); INSERT INTO asset_resource_sources VALUES('model_to_fdid',50,'wow');",
        )
        self.put("db2/fixture/ItemDisplayInfo.csv", b"ID,ModelResourcesID_0\n30,50\n")
        self.put(
            "db2/fixture/ModelFileData.csv", b"FileDataID,ModelResourcesID\n10,50\n"
        )
        self.publish("wow", 10, "m2", model())
        with closing(sqlite3.connect(":memory:")) as world:
            world.executescript(
                "CREATE TABLE content_item(ID,DisplayInfoID); INSERT INTO content_item VALUES(1,30),(2,30);"
            )
            graph = self.graph()
            seed_items(graph, Catalogs(graph), world, [1, 2])
        result = graph.run()
        self.assertEqual(result["assets"], [])
        self.assertEqual(
            {
                issue["fdid"]
                for issue in result["unresolved"]
                if issue["code"] == "missing_item_definition_source"
            },
            {1, 2},
        )

    def test_explicit_owned_item_sources_do_not_use_borrowed_npc_resource_groups(self):
        for source, build, product, fdid in [
            ("Retail", "active", "wow", 10),
            ("Forever70205", "1.60.1.70205", "wow_classic_beta", 20),
        ]:
            items = f"db2/{build}" + ("/items" if source == "Forever70205" else "")
            self.put(
                f"{items}/ItemModifiedAppearance.csv",
                b"ID,ItemID,ItemAppearanceID\n1,1,2\n",
            )
            self.put(f"{items}/ItemAppearance.csv", b"ID,ItemDisplayInfoID\n2,30\n")
            self.put(
                f"db2/{build}/ItemDisplayInfo.csv", b"ID,ModelResourcesID_0\n30,50\n"
            )
            self.put(
                f"db2/{build}/ModelFileData.csv",
                f"FileDataID,ModelResourcesID\n{fdid},50\n".encode(),
            )
            self.put(
                f"db2/{build}/TextureFileData.csv", b"FileDataID,MaterialResourcesID\n"
            )
            self.put(
                f"db2/{build}/ItemDisplayInfoMaterialRes.csv",
                b"ID,ItemDisplayInfoID,MaterialResourcesID\n",
            )
            self.publish(product, fdid, "m2", model())
        # Shared NPC row points elsewhere and cannot satisfy owned item source lookup.
        self.cache(
            "outfit_links-v5.sqlite",
            "INSERT INTO display_info VALUES(30,90,0,0,0); INSERT INTO model_to_fdid VALUES(90,0,999); INSERT INTO asset_resource_sources VALUES('model_to_fdid',90,'wow');",
        )
        with closing(sqlite3.connect(":memory:")) as world:
            world.execute("CREATE TABLE content_item(ID,DisplayInfoID)")
            graph = self.graph()
            seed_items(
                graph,
                Catalogs(graph),
                world,
                [
                    {"id": 1, "definition_source": "Retail"},
                    {"id": 1, "definition_source": "Forever70205"},
                ],
            )
        result = graph.run()
        self.assertEqual(
            {(row["requested_product"], row["fdid"]) for row in result["assets"]},
            {("wow", 10), ("wow_classic_beta", 20)},
        )
        self.assertEqual(result["summary"]["missing"], 0)

    def test_missing_display_source_never_guesses_from_graph_or_csv(self):
        self.cache(
            "creature_display.sqlite",
            "INSERT INTO creature_displays VALUES(100,10,0,0,0,0,NULL);",
        )
        self.put("db2/fixture/CreatureDisplayInfo.csv", b"ID,ModelID\n100,1\n")
        self.put("db2/fixture/CreatureModelData.csv", b"ID,FileDataID\n1,10\n")
        self.publish("wow", 10, "m2", model())
        graph = self.graph()
        seed_displays(graph, Catalogs(graph), [100])
        result = graph.run()
        self.assertEqual(result["assets"], [])
        self.assertTrue(
            any(
                issue["code"] == "missing_product_source"
                for issue in result["unresolved"]
            )
        )


if __name__ == "__main__":
    unittest.main()
