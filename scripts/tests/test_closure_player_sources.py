"""Ignored RED player overlay fixtures; no install, renderer or extraction."""

from contextlib import closing
import os
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


@unittest.skipUnless(
    os.environ.get("P4_RED"), "Ignored RED player source overlay; enable P4_RED"
)
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


if __name__ == "__main__":
    unittest.main()
