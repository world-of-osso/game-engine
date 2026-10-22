"""File-backed auditor cases; tempting flat/other-product bytes are not authority."""

from contextlib import closing
import hashlib
import json
from pathlib import Path
import sqlite3
import struct
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
from test_asset_closure import chunk, ints, model


class ReceiptClosureTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.data = Path(self.temporary.name)
        self.documents = {}
        self.keys = {"wow": "a" * 32, "wow_classic_beta": "b" * 32}
        self.builds = {"wow": "12.1.5.70448", "wow_classic_beta": "1.60.1.70450"}

    def put(self, relative, raw):
        path = self.data / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
        if relative.startswith("db2/") and relative.endswith(".csv"):
            receipt = path.parent / "receipt.json"
            document = (
                json.loads(receipt.read_text())
                if receipt.exists()
                else {
                    "identity": {
                        "product": "wow",
                        "build": "fixture-export",
                        "build_key": self.keys["wow"],
                    },
                    "tables": {},
                }
            )
            document["tables"][path.stem] = {
                "fdid": 1,
                "csv_sha256": hashlib.sha256(raw).hexdigest(),
                "content_authenticated": True,
                "unreadable_records": 0,
            }
            receipt.write_text(json.dumps(document))
        return path

    def publish(self, product, fdid, kind, raw, key=None, alias=False):
        key = key or self.keys[product]
        directory = "textures" if kind == "blp" else "models"
        relative = f"products/{product}/{key}/{directory}/{fdid}.{kind}"
        row = {
            "product": product,
            "build_key": key,
            "build": self.builds[product],
            "fdid": fdid,
            "kind": kind,
            "path": relative,
            "bytes": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest(),
            "content_key": hashlib.md5(raw).hexdigest(),
        }
        self.put(relative, raw)
        document = self.documents.setdefault(
            product, {"version": 1, "assets": [], "aliases": [], "chains": []}
        )
        document["aliases" if alias else "assets"].append(row)
        self.put(
            f"cache/model-asset-index-{product}.json", json.dumps(document).encode()
        )
        return row

    def auditor(self, product="wow"):
        from closure_receipts import ReceiptClosure

        return ReceiptClosure(self.data, {}, product, "fixture")

    def test_cli_discovers_children_when_only_scoped_model_exists(self):
        self.put("community-listfile.csv", b"")
        self.publish(
            "wow", 10, "m2", model(chunk("SFID", ints(11)), chunk("TXID", ints(12)))
        )
        self.publish("wow", 11, "skin", b"SKINfixture")
        self.publish("wow", 12, "blp", b"BLP2fixture")
        self.put("db2/fixture/ItemDisplayInfo.csv", b"ID,ModelResourcesID_0\n40,50\n")
        self.put(
            "db2/fixture/ModelFileData.csv", b"FileDataID,ModelResourcesID\n10,50\n"
        )
        self.put("db2/fixture/TextureFileData.csv", b"FileDataID,MaterialResourcesID\n")
        self.put(
            "db2/fixture/ItemDisplayInfoMaterialRes.csv",
            b"ID,ItemDisplayInfoID,MaterialResourcesID\n",
        )
        self.put(
            "db2/fixture/ItemDisplayInfoModelMatRes.csv",
            b"ID,ItemDisplayInfoID,MaterialResourcesID\n",
        )
        self.put("db2/fixture/ChrRaceXChrModel.csv", b"ID,ChrRacesID,Sex,ChrModelID\n")
        self.put("db2/fixture/ChrModel.csv", b"ID,DisplayID,SkeletonFileDataID\n")
        self.put("db2/fixture/CreatureDisplayInfo.csv", b"ID,ModelID\n")
        self.put("db2/fixture/CreatureModelData.csv", b"ID,FileDataID\n")
        self.put("db2/fixture/SpellMisc.csv", b"ID,SpellID,DifficultyID\n")
        world = self.data / "world.db"
        with closing(sqlite3.connect(world)) as db:
            db.executescript(
                "CREATE TABLE content_item(ID,DisplayInfoID); INSERT INTO content_item VALUES(1,40); CREATE TABLE content_creature_template_model(CreatureDisplayID); CREATE TABLE content_creature(guid,id1,id2,id3,modelid,position_x,position_y); CREATE TABLE content_gameobject_template(entry,type,displayId);"
            )
            db.commit()
        self.put(
            "db2/fixture/ItemModifiedAppearance.csv",
            b"ID,ItemID,ItemAppearanceID\n1,1,2\n",
        )
        self.put("db2/fixture/ItemAppearance.csv", b"ID,ItemDisplayInfoID\n2,40\n")
        (self.data / "db2/active").symlink_to("fixture", target_is_directory=True)
        config = {
            "product": "wow",
            "metadata_build": "fixture",
            "customization_cache": "cache/customization-v5.sqlite",
            "maps": [],
            "characters": [],
            "npc_displays": "all",
            "game_objects": "all",
            "items": [{"id": 1, "definition_source": "Retail"}],
            "spells": [],
        }
        self.put("config.json", json.dumps(config).encode())
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPTS / "asset_closure.py"),
                "--data",
                str(self.data),
                "--world-db",
                str(world),
                "--config",
                str(self.data / "config.json"),
                "--output",
                str(self.data / "manifest.json"),
            ],
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(
            result.returncode, 1, result.stderr
        )  # Explicit cache-policy gaps remain.
        manifest = json.loads((self.data / "manifest.json").read_text())
        self.assertEqual(
            [
                (row["requested_product"], row["fdid"], row["type"], row["present"])
                for row in manifest["assets"]
            ],
            [
                ("wow", 10, "m2", True),
                ("wow", 11, "skin", True),
                ("wow", 12, "blp", True),
            ],
        )
        self.assertEqual(manifest["summary"]["missing"], 0)
        self.assertEqual(manifest["assets"][0]["actual_build"], "12.1.5.70448")

    def test_same_fdid_products_keep_distinct_children_in_one_graph(self):
        for product, skin, texture in [("wow", 11, 12), ("wow_classic_beta", 21, 22)]:
            self.publish(
                product,
                10,
                "m2",
                model(chunk("SFID", ints(skin)), chunk("TXID", ints(texture))),
            )
            self.publish(product, skin, "skin", b"SKIN" + product.encode())
            self.publish(product, texture, "blp", b"BLP2" + product.encode())
        self.put("models/10.m2", model(chunk("TXID", ints(999))))
        graph = self.auditor()
        graph.add(10, "m2", "Retail body", product="wow")
        graph.add(10, "m2", "Forever actor", product="wow_classic_beta")
        manifest = graph.run()
        observed = {
            (row["requested_product"], row["fdid"], row["type"])
            for row in manifest["assets"]
        }
        self.assertEqual(
            observed,
            {
                ("wow", 10, "m2"),
                ("wow", 11, "skin"),
                ("wow", 12, "blp"),
                ("wow_classic_beta", 10, "m2"),
                ("wow_classic_beta", 21, "skin"),
                ("wow_classic_beta", 22, "blp"),
            },
        )
        self.assertEqual(manifest["summary"]["missing"], 0)

    def test_authentic_579120_collision_keeps_each_products_primary_skin(self):
        sources = [
            (
                "wow",
                "88d5e8ef05ec5a8446cdec93d142d7c6",
                "2f0242f06b19409a382e418c8a92f0c383536075fb2e1d25dce7f3bdc6d4b19f",
                "e751505c024d0372ff2dab9ce18330e52cf90e0bae511451487f96231c50cdfc",
            ),
            (
                "wow_classic_beta",
                "84bea4233331e390fda8724c6f91ffa5",
                "6254ea3ea766575c88fa899208e2f5b4558b47413c27d178a31dba28cbba55df",
                "c44314528670cf2507ec44acdf92d2195e5378bb966eef6462c7bccba75085f6",
            ),
        ]
        expected = {}
        for product, key, model_hash, skin_hash in sources:
            source = SCRIPTS.parent / "data/products" / product / key / "models"
            for fdid, kind, wanted in [
                (579120, "m2", model_hash),
                (579121, "skin", skin_hash),
            ]:
                raw = (source / f"{fdid}.{kind}").read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), wanted)
                self.publish(product, fdid, kind, raw, key=key)
                expected[product, fdid] = wanted
        # This exact wrong-product flat companion caused the native Vertex101 failure.
        retail_skin = (
            self.data / f"products/wow/{sources[0][1]}/models/579121.skin"
        ).read_bytes()
        self.put("models/579121.skin", retail_skin)
        graph = self.auditor()
        for product, _, _, _ in sources:
            graph.add(579120, "m2", "authentic product collision", product=product)
        manifest = graph.run()
        selected = {
            (row["requested_product"], row["fdid"]): row
            for row in manifest["assets"]
            if row["fdid"] in {579120, 579121}
        }
        self.assertEqual(set(selected), set(expected))
        for identity, wanted in expected.items():
            self.assertEqual(selected[identity]["sha256"], wanted)
            self.assertEqual(selected[identity]["identity_status"], "verified_receipt")
        # Other authored LODs/textures were deliberately not supplied: never call this full closure.
        self.assertGreater(manifest["summary"]["missing"], 0)

    def test_companion_requires_the_model_namespace_not_another_receipt_build(self):
        self.publish("wow", 10, "m2", model(chunk("SFID", ints(11))))
        self.publish("wow", 11, "skin", b"SKINsame", key="c" * 32)
        self.put("models/11.skin", b"SKINtempting")
        result = self.auditor()
        result.add(10, "m2", "body")
        manifest = result.run()
        skin = next(row for row in manifest["assets"] if row["type"] == "skin")
        self.assertFalse(skin["present"])
        self.assertEqual(
            skin["locations"], [f"products/wow/{self.keys['wow']}/models/11.skin"]
        )
        self.assertTrue(
            any(
                row["code"] == "missing_scoped_receipt"
                for row in manifest["unresolved"]
            )
        )
        self.publish("wow", 11, "skin", b"SKINsame", alias=True)
        repaired = self.auditor()
        repaired.add(10, "m2", "body")
        self.assertEqual(repaired.run()["summary"]["missing"], 0)

    def test_missing_product_receipt_never_uses_flat_or_other_product(self):
        self.publish("wow", 10, "m2", model())
        self.put("models/10.m2", model())
        graph = self.auditor("wow_classic_beta")
        graph.add(10, "m2", "Forever actor")
        manifest = graph.run()
        self.assertFalse(manifest["assets"][0]["present"])
        self.assertTrue(
            any(
                row["code"] == "missing_asset_receipt" for row in manifest["unresolved"]
            )
        )

    def test_corrupt_receipt_payload_stops_descendant_expansion(self):
        row = self.publish("wow", 10, "m2", model(chunk("TXID", ints(12))))
        self.put(row["path"], model(chunk("TXID", ints(99))))
        graph = self.auditor()
        graph.add(10, "m2", "body")
        manifest = graph.run()
        self.assertEqual([row["fdid"] for row in manifest["assets"]], [10])
        self.assertTrue(
            any(
                row["code"] == "receipt_payload_mismatch"
                for row in manifest["unresolved"]
            )
        )

    def test_flat_wmo_material_is_not_falsely_replaced_by_scoped_current_texture(self):
        material = bytearray(64)
        struct.pack_into("<I", material, 12, 12)
        self.put("models/20.wmo", chunk("MOMT", material, True))
        self.put("textures/12.blp", b"BLP2legacy")
        self.publish("wow", 12, "blp", b"BLP2current")
        graph = self.auditor()
        graph.add(20, "wmo", "native flat WMO")
        manifest = graph.run()
        texture = next(row for row in manifest["assets"] if row["fdid"] == 12)
        self.assertEqual(texture["present_files"][0]["path"], "textures/12.blp")
        self.assertIsNone(texture["actual_build"])
        self.assertEqual(texture["identity_status"], "unverified")

    def test_unreceipted_model_texture_cannot_be_satisfied_by_flat_wmo_texture(self):
        material = bytearray(64)
        struct.pack_into("<I", material, 12, 12)
        self.put("models/20.wmo", chunk("MOMT", material, True))
        self.put("textures/12.blp", b"BLP2legacy")
        graph = self.auditor()
        graph.add(12, "blp", "independently sourced replacement texture")
        graph.add(20, "wmo", "native flat WMO")
        textures = [row for row in graph.run()["assets"] if row["type"] == "blp"]
        self.assertEqual(len(textures), 2)
        self.assertEqual(sorted(row["present"] for row in textures), [False, True])
        self.assertTrue(all(row["identity_status"] == "unverified" for row in textures))

    def test_same_product_shared_skin_retains_each_owning_model_namespace(self):
        for fdid, key in [(10, "a" * 32), (20, "c" * 32)]:
            self.publish("wow", fdid, "m2", model(chunk("SFID", ints(11))), key=key)
        self.publish("wow", 11, "skin", b"SKINsame")
        self.publish("wow", 11, "skin", b"SKINsame", key="c" * 32, alias=True)
        graph = self.auditor()
        graph.add(10, "m2", "first body")
        graph.add(20, "m2", "second body")
        manifest = graph.run()
        skins = [row for row in manifest["assets"] if row["type"] == "skin"]
        self.assertEqual(len(skins), 2)
        self.assertEqual(
            {row["asset_build_key"] for row in skins}, {"a" * 32, "c" * 32}
        )
        self.assertEqual(manifest["summary"]["missing"], 0)

    def test_changed_payload_cannot_retain_verified_identity_on_graph_recheck(self):
        row = self.publish("wow", 10, "m2", model())
        graph = self.auditor()
        graph.add(10, "m2", "body")
        self.assertEqual(
            graph.run()["assets"][0]["identity_status"], "verified_receipt"
        )
        self.put(row["path"], model(chunk("TXID", ints(99))))
        rechecked = graph.run()
        self.assertEqual(rechecked["assets"][0]["identity_status"], "unverified")
        self.assertIsNone(rechecked["assets"][0]["actual_build"])
        self.assertTrue(
            any(
                issue["code"] == "receipt_payload_mismatch"
                for issue in rechecked["unresolved"]
            )
        )

    def test_alias_bytes_must_match_the_same_product_receipt_owner(self):
        self.publish("wow", 10, "m2", model(chunk("SFID", ints(11))))
        self.publish("wow", 11, "skin", b"SKINauthentic", key="c" * 32)
        self.publish("wow", 11, "skin", b"SKINdifferent", alias=True)
        graph = self.auditor()
        graph.add(10, "m2", "body")
        manifest = graph.run()
        skin = next(row for row in manifest["assets"] if row["type"] == "skin")
        self.assertEqual(skin["identity_status"], "unverified")
        self.assertTrue(
            any(
                issue["code"] == "alias_receipt_mismatch"
                for issue in manifest["unresolved"]
            )
        )

    def test_unused_modelnamed_alias_without_owner_fields_does_not_poison_index(self):
        product = "wow_classic_beta"
        self.keys[product] = "f6e309c700cea095978aeb5d85210df4"
        self.builds[product] = "1.60.1.70338"
        self.publish(product, 123939, "m2", model(chunk("SFID", ints(475713))))
        owner = self.publish(product, 475713, "skin", b"SKINlegacy")
        alias = {
            key: owner[key]
            for key in ["product", "build_key", "fdid", "kind", "bytes", "sha256"]
        }
        alias.update(
            path=f"products/{product}/{self.keys[product]}/models/12393900.skin",
            owner_model_fdid=123939,
            reference="SFID[0]",
        )
        self.documents[product]["aliases"].append(alias)
        self.put(
            f"cache/model-asset-index-{product}.json",
            json.dumps(self.documents[product]).encode(),
        )
        graph = self.auditor(product)
        graph.add(123939, "m2", "recorded production alias ordering")
        result = graph.run()
        skin = next(row for row in result["assets"] if row["type"] == "skin")
        self.assertTrue(skin["present"])
        self.assertEqual(skin["actual_build"], "1.60.1.70338")
        self.assertEqual(skin["locations"], [owner["path"]])
        self.assertFalse((self.data / alias["path"]).exists())

    def test_selected_numeric_alias_derives_build_and_md5_from_unique_owner(self):
        self.publish("wow", 10, "m2", model(chunk("SFID", ints(11))))
        owner = self.publish("wow", 11, "skin", b"SKINsame", key="c" * 32)
        alias = {
            key: owner[key] for key in ["product", "fdid", "kind", "bytes", "sha256"]
        }
        alias.update(
            build_key=self.keys["wow"],
            path=f"products/wow/{self.keys['wow']}/models/11.skin",
            owner_model_fdid=10,
            reference="SFID[0]",
        )
        self.put(alias["path"], b"SKINsame")
        self.documents["wow"]["aliases"].append(alias)
        self.put(
            "cache/model-asset-index-wow.json",
            json.dumps(self.documents["wow"]).encode(),
        )
        graph = self.auditor()
        graph.add(10, "m2", "selected build-qualified alias")
        skin = next(row for row in graph.run()["assets"] if row["type"] == "skin")
        self.assertEqual(skin["identity_status"], "verified_receipt")
        self.assertEqual(skin["actual_build"], owner["build"])
        self.assertEqual(skin["sha256"], owner["sha256"])

    def test_duplicate_receipts_are_rejected_before_selecting_bytes(self):
        self.publish("wow", 10, "m2", model())
        self.publish("wow", 10, "m2", model())
        with self.assertRaisesRegex(ValueError, "Duplicate asset receipt"):
            self.auditor()

    def test_receipt_path_cannot_escape_its_product_namespace(self):
        self.publish("wow", 10, "m2", model())
        self.documents["wow"]["assets"][0]["path"] = "models/10.m2"
        self.put(
            "cache/model-asset-index-wow.json",
            json.dumps(self.documents["wow"]).encode(),
        )
        with self.assertRaisesRegex(ValueError, "namespace"):
            self.auditor()


if __name__ == "__main__":
    unittest.main()
