import hashlib
import sqlite3
import struct
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

from scripts import import_forever_skyborne as importer


class ForeverNpcDisplayTests(unittest.TestCase):
    def tables(self):
        return {
            "CreatureDisplayInfo": [
                {
                    "ID": 10,
                    "ModelID": 1,
                    "ExtendedDisplayInfoID": 0,
                    "CreatureModelScale": 1,
                    "TextureVariationFileDataID_0": 300,
                },
                {
                    "ID": 136968,
                    "ModelID": 1,
                    "ExtendedDisplayInfoID": 20,
                    "CreatureModelScale": 1,
                    "TextureVariationFileDataID_0": 301,
                },
                {
                    "ID": 139694,
                    "ModelID": 1,
                    "ExtendedDisplayInfoID": 21,
                    "CreatureModelScale": 1,
                },
            ],
            "CreatureModelData": [{"ID": 1, "FileDataID": 7478494, "ModelScale": 1}],
            "CreatureDisplayInfoExtra": [
                {
                    "ID": 20,
                    "DisplayRaceID": 95,
                    "DisplaySexID": 1,
                    "DisplayClassID": 1,
                    "Flags": 0,
                    "BakeMaterialResourcesID": 400,
                    "HDBakeMaterialResourcesID": 401,
                },
                {
                    "ID": 21,
                    "DisplayRaceID": 95,
                    "DisplaySexID": 1,
                    "DisplayClassID": 1,
                    "Flags": 0,
                    "BakeMaterialResourcesID": 402,
                    "HDBakeMaterialResourcesID": 0,
                },
            ],
            "CreatureDisplayInfoOption": [
                {
                    "CreatureDisplayInfoExtraID": 20,
                    "ChrCustomizationOptionID": 1,
                    "ChrCustomizationChoiceID": 50,
                }
            ],
            "CreatureDisplayInfoGeosetData": [
                {"CreatureDisplayInfoID": 136968, "GeosetIndex": 2, "GeosetValue": 3}
            ],
            "NPCModelItemSlotDisplayInfo": [
                {"NpcModelID": 20, "ItemDisplayInfoID": 60, "ItemSlot": 0}
            ],
            "ItemDisplayInfo": [
                {"ID": 60, "ModelResourcesID_0": 500, "ModelMaterialResourcesID_0": 403}
            ],
            "ItemDisplayInfoMaterialRes": [
                {"ItemDisplayInfoID": 60, "MaterialResourcesID": 404}
            ],
            "ModelFileData": [{"ModelResourcesID": 500, "FileDataID": 700}],
            "TextureFileData": [
                {"MaterialResourcesID": material, "FileDataID": texture, "UsageType": 0}
                for material, texture in [
                    (400, 800),
                    (401, 801),
                    (402, 802),
                    (403, 803),
                    (404, 804),
                    (405, 805),
                ]
            ],
            "ChrCustomizationElement": [
                {
                    "ChrCustomizationChoiceID": 50,
                    "ChrCustomizationMaterialID": 90,
                    "ChrCustomizationSkinnedModelID": 91,
                }
            ],
            "ChrCustomizationMaterial": [{"ID": 90, "MaterialResourcesID": 405}],
            "ChrCustomizationSkinnedModel": [{"ID": 91, "CollectionsFileDataID": 701}],
        }

    def test_npc_assets_follow_display_body_bake_customization_and_items(self):
        assets, failures = importer.npc_asset_roots(
            self.tables(),
            {10, 136968, 139694},
            {10},
            {7478494: "character/skyborne/female/skybornefemale.m2"},
        )
        self.assertEqual(set(assets), {136968, 139694})
        self.assertEqual(failures, {})
        self.assertEqual(
            assets[136968],
            {
                (7478494, "m2"),
                (301, "blp"),
                (800, "blp"),
                (803, "blp"),
                (804, "blp"),
                (805, "blp"),
                (700, "m2"),
                (701, "m2"),
            },
        )
        self.assertEqual(assets[139694], {(7478494, "m2"), (802, "blp")})

    def test_declared_extra_is_not_an_ordinary_display_when_table_missing(self):
        tables = self.tables()
        del tables["CreatureDisplayInfoExtra"]
        assets, failures = importer.npc_asset_roots(
            tables, {136968}, set(), {7478494: "skybornefemale.m2"}
        )
        self.assertTrue({(7478494, "m2"), (301, "blp")} <= assets[136968])
        self.assertIn("CreatureDisplayInfoExtra", failures[136968][0])

    def test_missing_model_and_requested_display_fail_explicitly(self):
        tables = self.tables()
        tables["CreatureModelData"] = []
        _, failures = importer.npc_asset_roots(tables, {136968, 999999}, set(), {})
        self.assertIn("ModelID 1", failures[136968][0])
        self.assertIn("CreatureDisplayInfo", failures[999999][0])

    def test_appearance_rows_keep_actual_model_bake_and_display_geosets(self):
        rows = importer.npc.npc_appearance_rows(
            self.tables(), {136968, 139694}, {7478494: "skybornefemale.m2"}
        )
        self.assertEqual(rows[0], [(136968, 95, 1, 1, 800), (139694, 95, 1, 1, 802)])
        self.assertEqual(rows[1], [(136968, 50)])
        self.assertEqual(rows[2], [(136968, 2, 3)])
        self.assertEqual(rows[3], [(136968, 1), (139694, 1)])

    def test_published_catalogs_preserve_retail_rows_and_distinct_forever_chain(self):
        with tempfile.TemporaryDirectory() as directory:
            data = Path(directory)
            (data / "cache").mkdir()
            with (
                closing(
                    sqlite3.connect(data / "cache/creature_display.sqlite")
                ) as conn,
                conn,
            ):
                conn.execute(
                    "create table creature_displays(display_id integer primary key, model_fdid integer, a integer, b integer, c integer, scale integer)"
                )
                conn.execute(
                    "insert into creature_displays values(10,1011653,11,12,13,1250)"
                )
            importer.npc.publish_display_rows(
                data, self.tables(), {10, 136968, 139694}, {10}
            )
            with (
                closing(
                    sqlite3.connect(data / "cache/creature_display.sqlite")
                ) as conn,
                conn,
            ):
                self.assertEqual(
                    conn.execute(
                        "select * from creature_displays where display_id=10"
                    ).fetchone(),
                    (10, 1011653, 11, 12, 13, 1250),
                )
                self.assertEqual(
                    conn.execute(
                        "select model_fdid from creature_displays where display_id=136968"
                    ).fetchone(),
                    (7478494,),
                )
            rows = importer.npc.npc_appearance_rows(
                self.tables(), {136968}, {7478494: "skybornefemale.m2"}
            )
            from scripts import import_npc_appearance

            import_npc_appearance.write_database(
                data / "cache/npc_appearance.sqlite",
                ([(10, 1, 0, 1, 123)], [(10, 2)], [], [(10, 1)]),
            )
            importer.npc.publish_appearance_rows(data, rows)
            importer.npc.publish_appearance_rows(data, rows)
            with closing(sqlite3.connect(data / "cache/npc_appearance.sqlite")) as conn:
                self.assertEqual(
                    conn.execute(
                        "select * from appearances where display_id=10"
                    ).fetchone(),
                    (10, 1, 0, 1, 123),
                )
                self.assertEqual(
                    conn.execute(
                        "select * from appearances where display_id=136968"
                    ).fetchone(),
                    (136968, 95, 1, 1, 800),
                )
                self.assertEqual(
                    conn.execute(
                        "select count(*) from choices where display_id=136968"
                    ).fetchone(),
                    (1,),
                )

    def test_asset_closure_reports_missing_child_not_just_available_parent(self):
        result = importer.npc.summarize_asset_closures(
            {136968: {(1, "m2")}},
            {(1, "m2"): {(2, "skin"), (3, "blp")}},
            {(1, "m2"), (2, "skin"), (3, "blp")},
            {(3, "blp")},
        )
        self.assertEqual(
            result[136968],
            {
                "resolved": 2,
                "failed": 1,
                "assets": ["1.m2", "2.skin", "3.blp"],
                "missing": ["3.blp"],
            },
        )

    def test_probe_reuse_never_accepts_wrong_current_root_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            probe, staging = directory / "probe", directory / "staging"
            probe.mkdir()
            staging.mkdir()
            (probe / "7478494.dat").write_bytes(b"MD21")
            with closing(sqlite3.connect(":memory:")) as connection, connection:
                connection.execute(
                    "create table resolution(fdid integer, content_key blob)"
                )
                connection.execute(
                    "insert into resolution values (?, ?)",
                    (7478494, hashlib.md5(b"MD21").digest()),
                )
                importer.stage_verified_probes(probe, staging, {7478494}, connection)
                self.assertEqual((staging / "7478494.dat").read_bytes(), b"MD21")
                (probe / "7478494.dat").write_bytes(b"wrong")
                with self.assertRaisesRegex(ValueError, "content key mismatch"):
                    importer.stage_verified_probes(
                        probe, staging, {7478494}, connection
                    )

    def test_encrypted_record_ids_are_reported_not_reconstructed(self):
        raw = bytearray(308)
        struct.pack_into("<I", raw, 144, 4)
        struct.pack_into("<I", raw, 200, 2)
        struct.pack_into("<Q8I", raw, 204, 0, 300, 1, 0, 304, 0, 0, 0, 0)
        struct.pack_into("<Q8I", raw, 244, 123, 304, 1, 0, 308, 0, 0, 0, 0)
        struct.pack_into("<2I", raw, 284, 1, 12345)
        self.assertEqual(importer.encrypted_record_ids(bytes(raw)), [12345])


if __name__ == "__main__":
    unittest.main()
