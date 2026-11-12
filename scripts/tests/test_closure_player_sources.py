"""Player body/customization source overlay; no install, renderer or extraction."""

from contextlib import closing
from pathlib import Path
import sqlite3
import sys
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
from closure_receipts import ReceiptClosure
from closure_seeds import seed_catalogs
from test_asset_closure import model
import test_closure_root_products as fixtures

CUSTOMIZATION_SCHEMA = """
CREATE TABLE options(id,chr_model_id,order_index,requirement_id);
CREATE TABLE choices(id,option_id,order_index,requirement_id,visibility_requirement_id);
CREATE TABLE elements(choice_id,related_choice_id,material_id,skinned_model_id,has_unsupported_effects);
CREATE TABLE materials(id,material_resources_id);
CREATE TABLE texture_fdids(material_resources_id,file_data_id);
CREATE TABLE skinned_models(id,collection_fdid);
"""


class PlayerSourceTests(unittest.TestCase):
    def setUp(self):
        self.files = fixtures.RootProductTests()
        self.files.setUp()
        self.addCleanup(self.files.doCleanups)
        self.data = self.files.data
        self.config = dict(self.files.config, metadata_build="active", characters="all")
        self.world = self.files.empty_world()
        retail = {
            "ChrRaceXChrModel": "ID,ChrRacesID,ChrModelID,Sex\n1,1,1,0\n2,95,1,0\n3,95,1,1\n4,96,1,0\n5,96,1,1\n",
            "ChrModel": "ID,DisplayID,SkeletonFileDataID\n1,100,0\n",
            "CreatureDisplayInfo": "ID,ModelID\n100,1\n",
            "CreatureModelData": "ID,FileDataID\n1,10\n",
            "ChrCustomizationReq": "ID,ReqType,ClassMask,RaceMasks_0,RaceMasks_1\n1,1,0,0,1\n",
        }
        forever = {
            "ChrRaceXChrModel": "ID,ChrRacesID,ChrModelID,Sex\n1,95,218,0\n2,95,219,1\n3,96,218,0\n4,96,219,1\n5,1,218,0\n",
            "ChrModel": "ID,DisplayID,SkeletonFileDataID\n218,2180,0\n219,2190,0\n",
            "CreatureDisplayInfo": "ID,ModelID\n2180,218\n2190,219\n",
            "CreatureModelData": "ID,FileDataID\n218,20\n219,21\n",
            "ChrCustomizationReq": "ID,ReqType,ClassMask,RaceMasks_0,RaceMasks_1\n1,1,0,0,1\n",
        }
        for directory, tables in [("active", retail), ("1.60.1.70205", forever)]:
            for name, text in tables.items():
                self.files.put(f"db2/{directory}/{name}.csv", text.encode())
        self.cache(
            "cache/customization-v5.sqlite",
            CUSTOMIZATION_SCHEMA
            + """
INSERT INTO options VALUES(1,1,0,0),(8,218,0,0);
INSERT INTO choices VALUES(300,1,0,0,0),(999,8,0,0,0);
INSERT INTO elements VALUES(300,0,30,31,0),(999,0,30,31,0);
INSERT INTO materials VALUES(30,30);
INSERT INTO texture_fdids VALUES(30,30);
INSERT INTO skinned_models VALUES(31,31);
""",
        )
        self.cache(
            "db2/1.60.1.70205/cache/customization-v5.sqlite",
            CUSTOMIZATION_SCHEMA
            + """
INSERT INTO options VALUES(8,218,0,0),(9,219,0,0),(99,999,0,0);
INSERT INTO choices VALUES(400,8,0,1,0),(401,9,0,1,0),(499,99,0,0,0);
INSERT INTO elements VALUES(400,0,40,41,0),(401,0,40,41,0),(499,0,49,49,0);
INSERT INTO materials VALUES(40,40),(49,49);
INSERT INTO texture_fdids VALUES(40,40),(49,49);
INSERT INTO skinned_models VALUES(41,41),(49,49);
""",
        )
        # Merged NPC CDI cache is not the source for player body chains.
        self.files.cache(
            "creature_display.sqlite",
            "INSERT INTO creature_displays VALUES(100,10,0,0,0,0,'wow'),(2180,999,0,0,0,0,'wow'),(2190,999,0,0,0,0,'wow');",
        )
        for product, ids in [("wow", [10, 31]), ("wow_classic_beta", [20, 21, 41])]:
            for fdid in ids:
                self.files.publish(product, fdid, "m2", model())
        self.files.publish("wow", 30, "blp", b"BLP2retail")
        self.files.publish("wow_classic_beta", 40, "blp", b"BLP2forever")

    def cache(self, relative, sql):
        path = self.data / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        with closing(sqlite3.connect(path)) as db:
            db.executescript(sql)
            db.commit()
        return path

    def run_graph(self, characters="all"):
        graph = ReceiptClosure(self.data, {}, "wow", "active")
        seed_catalogs(graph, self.world, dict(self.config, characters=characters))
        return graph.run()

    def test_all_skyborne_pairs_use_forever_body_and_customizations_only(self):
        result = self.run_graph()
        actual = {(row["requested_product"], row["fdid"]) for row in result["assets"]}
        self.assertEqual(
            actual,
            {
                ("wow", 10),
                ("wow", 30),
                ("wow", 31),
                ("wow_classic_beta", 20),
                ("wow_classic_beta", 21),
                ("wow_classic_beta", 40),
                ("wow_classic_beta", 41),
            },
        )
        profiles = result["seeds"]["character_sources"]
        self.assertEqual(
            {
                (row["race"], row["sex"])
                for row in profiles
                if row["product"] == "wow_classic_beta"
            },
            {(95, 0), (95, 1), (96, 0), (96, 1)},
        )
        self.assertEqual(
            next(row["model_fdid"] for row in profiles if row["race"] == 1), 10
        )
        self.assertEqual(result["summary"]["missing"], 0)

    def test_missing_forever_body_mapping_never_uses_retail_placeholder(self):
        self.files.put(
            "db2/1.60.1.70205/ChrRaceXChrModel.csv",
            b"ID,ChrRacesID,ChrModelID,Sex\n1,1,218,0\n",
        )
        result = self.run_graph([{"race": 95, "sex": 0, "choices": "all"}])
        self.assertEqual(result["assets"], [])
        self.assertTrue(
            any(
                issue["code"] == "missing_player_source_row"
                for issue in result["unresolved"]
            )
        )

    def test_missing_forever_customization_cache_never_reads_retail_cache(self):
        (self.data / "db2/1.60.1.70205/cache/customization-v5.sqlite").unlink()
        result = self.run_graph([{"race": 95, "sex": 0, "choices": "all"}])
        actual = {(row["requested_product"], row["fdid"]) for row in result["assets"]}
        self.assertEqual(actual, {("wow_classic_beta", 20)})
        self.assertTrue(
            any(
                issue["code"] == "missing_metadata_file"
                and "1.60.1.70205/cache/customization-v5.sqlite" in issue["reason"]
                for issue in result["unresolved"]
            )
        )

    def test_default_skyborne_requirements_use_authored_playable_race_bits(self):
        result = self.run_graph(
            [{"race": 95, "sex": 0, "class": 8, "choices": "default"}]
        )
        actual = {(row["requested_product"], row["fdid"]) for row in result["assets"]}
        self.assertEqual(
            actual,
            {
                ("wow_classic_beta", 20),
                ("wow_classic_beta", 40),
                ("wow_classic_beta", 41),
            },
        )
        self.assertEqual(
            result["seeds"]["character_sources"][0]["selected_choices"], [400]
        )

    def test_starting_items_follow_native_source_and_race_mask_rules(self):
        self.files.put(
            "db2/active/ChrRaces.csv",
            b"ID,PlayableRaceBit\n8,7\n",
        )
        self.files.put(
            "db2/active/ChrRaceXChrModel.csv",
            b"ID,ChrRacesID,ChrModelID,Sex\n"
            b"1,1,1,0\n2,95,1,0\n3,95,1,1\n"
            b"4,96,1,0\n5,96,1,1\n6,8,1,0\n",
        )
        self.files.put(
            "db2/active/ItemModifiedAppearance.csv",
            b"ID,ItemID,ItemAppearanceID\n1,157655,2\n2,157657,3\n3,157658,4\n",
        )
        self.files.put(
            "db2/active/ItemAppearance.csv",
            b"ID,ItemDisplayInfoID\n2,82927\n3,82929\n4,82930\n",
        )
        self.files.put(
            "db2/1.60.1.70205/items/ItemModifiedAppearance.csv",
            b"ID,ItemID,ItemAppearanceID\n1,157656,2\n",
        )
        self.files.put(
            "db2/1.60.1.70205/items/ItemAppearance.csv",
            b"ID,ItemDisplayInfoID\n2,82928\n",
        )
        for product_path, displays, models in [
            (
                "db2/active",
                [(82927, 50), (82929, 52), (82930, 53)],
                [(464822, 50), (464824, 52), (464825, 53)],
            ),
            (
                "db2/1.60.1.70205",
                [(82928, 51)],
                [(464823, 51)],
            ),
        ]:
            display_rows = "".join(
                f"{display},{resource},0\n" for display, resource in displays
            )
            model_rows = "".join(
                f"{fdid},{resource}\n" for fdid, resource in models
            )
            display_csv = (
                "ID,ModelResourcesID_1,ModelMaterialResourcesID_1\n"
                f"{display_rows}"
            )
            model_csv = f"FileDataID,ModelResourcesID\n{model_rows}"
            self.files.put(
                f"{product_path}/ItemDisplayInfo.csv", display_csv.encode()
            )
            self.files.put(
                f"{product_path}/ModelFileData.csv", model_csv.encode()
            )
            for table, header in [
                ("TextureFileData", "FileDataID,MaterialResourcesID\n"),
                (
                    "ItemDisplayInfoMaterialRes",
                    "ID,ItemDisplayInfoID,MaterialResourcesID\n",
                ),
                (
                    "ItemDisplayInfoModelMatRes",
                    "ID,ItemDisplayInfoID,MaterialResourcesID\n",
                ),
            ]:
                self.files.put(f"{product_path}/{table}.csv", header.encode())

        with closing(sqlite3.connect(self.world)) as db:
            db.executescript(
                "CREATE TABLE content_item(ID INTEGER PRIMARY KEY);"
                "INSERT INTO content_item VALUES(157655),(157656),(157657),(157658);"
                "CREATE TABLE skyborne_content_item(ID INTEGER PRIMARY KEY);"
                "INSERT INTO skyborne_content_item VALUES(157656),(157658);"
                "CREATE TABLE character_loadout("
                "ID INTEGER PRIMARY KEY,ChrClassID,Purpose,ModID,RaceMasks_0,RaceMasks_1);"
                "INSERT INTO character_loadout VALUES"
                "(1,7,9,75,128,0),(2,1,9,75,0,1),(3,7,9,75,128,0),(4,7,9,75,64,0);"
                "CREATE TABLE character_loadout_item("
                "ID INTEGER PRIMARY KEY,CharacterLoadoutID,ItemID);"
                "INSERT INTO character_loadout_item VALUES"
                "(1,1,157655),(2,2,157656),(3,3,157658),(4,4,157658);"
                "CREATE TABLE db2_row_source("
                "table_name,row_id,build,file_data_id,layout_hash,sha256);"
                "INSERT INTO db2_row_source VALUES"
                "('character_loadout',1,'12.1.5.70448',1,'',''),"
                "('character_loadout',2,'1.60.1.70205',2,'',''),"
                "('character_loadout',3,'1.60.1.70205',3,'',''),"
                "('character_loadout',4,'12.1.5.70448',4,'','');"
                "CREATE TABLE tdb_playercreateinfo_item("
                "race,class,itemid,amount);"
                "INSERT INTO tdb_playercreateinfo_item VALUES"
                "(8,7,157657,1),(95,1,157656,-1);"
            )
        for product, fdid in [
            ("wow", 464822),
            ("wow_classic_beta", 464823),
            ("wow", 464824),
            ("wow", 464825),
        ]:
            raw = model()
            self.files.publish(product, fdid, "m2", raw)
            self.files.put(f"models/{fdid}.m2", raw)

        characters = [
            {"race": 8, "sex": 0, "class": 7, "level": 1, "choices": "default"},
            {"race": 95, "sex": 0, "class": 1, "level": 1, "choices": "default"},
        ]
        config = dict(self.config, characters=characters, items="starting")

        def result_for(graph):
            seed_catalogs(graph, self.world, config)
            return graph.run()

        receipt_graph = ReceiptClosure(self.data, {}, "wow", "active")
        receipt_result = result_for(receipt_graph)
        receipt_assets = {
            (row["requested_product"], row["fdid"])
            for row in receipt_result["assets"]
        }
        self.assertIn(("wow", 464822), receipt_assets)
        self.assertIn(("wow", 464824), receipt_assets)
        self.assertIn(("wow_classic_beta", 464823), receipt_assets)
        self.assertNotIn(("wow_classic_beta", 464822), receipt_assets)
        self.assertNotIn(("wow", 464823), receipt_assets)
        self.assertNotIn(("wow", 464825), receipt_assets)



if __name__ == "__main__":
    unittest.main()
