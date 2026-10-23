"""Actual flat bytes and exported DB2 receipt qualification; no CASC/network."""

import hashlib
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
from closure_receipts import ReceiptClosure
from closure_seeds import Catalogs, seed_icons
from test_asset_closure import chunk


class NonModelReceiptTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.data = Path(self.temporary.name)
        self.ledger = []

    def put(self, relative, raw):
        path = self.data / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
        return path

    def graph(self):
        graph = ReceiptClosure(self.data, {}, "wow", "fixture")
        graph.seeds = {}
        return graph

    def receipt(self, relative, fdid, kind, raw, product="wow", build="12.1.5.70448"):
        row = {
            "actual_build": build,
            "build_config": "a" * 32,
            "content_key": hashlib.md5(raw).hexdigest(),
            "disposition": "verified_existing",
            "encoding_key": "b" * 32,
            "fdid": fdid,
            "path": relative,
            "product": product,
            "sha256": hashlib.sha256(raw).hexdigest(),
            "size": len(raw),
            "type": kind,
        }
        self.ledger.append(row)
        self.put(
            "provenance/closure-extract.jsonl",
            ("\n".join(json.dumps(value) for value in self.ledger) + "\n").encode(),
        )
        return row

    def flat_texture_graph(self, raw=b"BLP2legacy"):
        material = bytearray(64)
        struct.pack_into("<I", material, 12, 12)
        self.put("models/20.wmo", chunk("MOMT", material, True))
        self.put("textures/12.blp", raw)
        graph = self.graph()
        graph.add(20, "wmo", "native WMO")
        return graph

    def table_receipt(self, raw, *, product="wow", unreadable=0):
        self.put("db2/fixture/Map.csv", raw)
        document = {
            "identity": {
                "product": product,
                "build": "12.1.5.70448",
                "build_key": "a" * 32,
            },
            "tables": {
                "Map": {
                    "fdid": 1349477,
                    "content_authenticated": True,
                    "csv_sha256": hashlib.sha256(raw).hexdigest(),
                    "unreadable_records": unreadable,
                    "rows": 1,
                }
            },
        }
        self.put("db2/fixture/receipt.json", json.dumps(document).encode())
        return document

    @unittest.skipUnless(os.environ.get("P4_RED") == "1", "P4 structural RED")
    def test_flat_adt_wmo_keep_exact_paths_and_historical_receipt_identity(self):
        for fdid, kind, directory, version in [
            (10, "adt", "terrain", 18),
            (20, "wmo", "models", 17),
        ]:
            with self.subTest(kind=kind):
                raw = chunk("MVER", struct.pack("<I", version), True)
                relative = f"{directory}/{fdid}.{kind}"
                self.put(relative, raw)
                receipt = self.receipt(relative, fdid, kind, raw, build="12.1.0.69933")
                graph = self.graph()
                graph.add(fdid, kind, "native structural root")
                result = graph.run()
                asset = result["assets"][0]
                self.assertEqual(asset["identity_status"], "verified_receipt")
                self.assertEqual(asset["actual_build"], "12.1.0.69933")
                self.assertEqual(asset["verified_builds"], ["12.1.0.69933"])
                self.assertEqual(
                    asset["file_receipt_build_keys"], [receipt["build_config"]]
                )
                self.assertEqual(asset["source_product"], "wow")
                self.assertEqual(asset["present_files"][0]["path"], relative)
                self.assertFalse(asset["receipt_selected"])
                self.assertEqual(result["summary"]["unverified_identity"], 0)

    def test_structural_receipts_refuse_wrong_identity_path_and_payload(self):
        for fdid, kind, directory in [(10, "adt", "terrain"), (20, "wmo", "models")]:
            for field, value in [
                ("product", "wow_classic_beta"),
                ("fdid", 999),
                ("type", "wmo" if kind == "adt" else "adt"),
                ("path", f"products/wow/{'a' * 32}/{directory}/{fdid}.{kind}"),
                ("size", 999),
                ("sha256", "c" * 64),
                ("content_key", "c" * 32),
                ("build_config", "not-a-build-key"),
                ("disposition", "unverified"),
            ]:
                with self.subTest(kind=kind, field=field):
                    raw = chunk("MVER", struct.pack("<I", 18), True)
                    relative = f"{directory}/{fdid}.{kind}"
                    self.put(relative, raw)
                    self.ledger = []
                    row = self.receipt(relative, fdid, kind, raw)
                    row[field] = value
                    self.put(
                        "provenance/closure-extract.jsonl",
                        (json.dumps(row) + "\n").encode(),
                    )
                    graph = self.graph()
                    graph.add(fdid, kind, "native structural root")
                    result = graph.run()
                    self.assertEqual(
                        result["assets"][0]["identity_status"], "unverified"
                    )

    def test_generic_structural_eligibility_does_not_admit_other_flat_families(self):
        for kind in ["skin", "skel", "anim", "bone", "wdt", "wdl", "phys"]:
            with self.subTest(kind=kind):
                raw = chunk("MVER", struct.pack("<I", 18), True)
                directory = "terrain" if kind in {"wdt", "wdl"} else "models"
                relative = f"{directory}/30.{kind}"
                self.put(relative, raw)
                self.receipt(relative, 30, kind, raw)
                graph = self.graph()
                graph.add_flat(30, kind, "outside approved structural families")
                asset = graph.run()["assets"][0]
                self.assertEqual(asset["identity_status"], "unverified")

    def test_generic_receipt_cannot_replace_required_model_owner(self):
        raw = b"MD20model"
        self.put("models/40.m2", raw)
        self.receipt("models/40.m2", 40, "m2", raw)
        graph = self.graph()
        graph.add(40, "m2", "receipt-selected native model")
        result = graph.run()
        self.assertFalse(result["assets"][0]["present"])
        self.assertEqual(result["assets"][0]["identity_status"], "unverified")
        self.assertTrue(
            any(row["code"] == "missing_asset_receipt" for row in result["unresolved"])
        )

    def test_flat_texture_qualifies_only_its_actual_legacy_path_bytes(self):
        raw = b"BLP2legacy"
        graph = self.flat_texture_graph(raw)
        self.receipt("textures/12.blp", 12, "blp", raw)
        texture = next(row for row in graph.run()["assets"] if row["type"] == "blp")
        self.assertEqual(texture["identity_status"], "verified_receipt")
        self.assertEqual(texture["actual_build"], "12.1.5.70448")
        self.assertEqual(texture["present_files"][0]["path"], "textures/12.blp")
        self.assertFalse(texture["receipt_selected"])

    def test_flat_sound_uses_sound_receipts_without_a_model_index(self):
        raw = b"OggSactualsound"
        self.put("sounds/30.ogg", raw)
        self.receipt("sounds/30.ogg", 30, "ogg", raw)
        graph = self.graph()
        graph.add(30, "ogg", "authored SoundKit")
        sound = graph.run()["assets"][0]
        self.assertEqual(sound["identity_status"], "verified_receipt")
        self.assertEqual(sound["source_product"], "wow")
        self.assertFalse((self.data / "cache/model-asset-index.json").exists())

    def test_other_product_sound_receipt_cannot_authenticate_same_flat_bytes(self):
        raw = b"OggSsamebytes"
        self.put("sounds/30.ogg", raw)
        self.receipt("sounds/30.ogg", 30, "ogg", raw, product="wow_classic_beta")
        graph = self.graph()
        graph.add(30, "ogg", "Retail SoundKit")
        result = graph.run()
        self.assertEqual(result["assets"][0]["identity_status"], "unverified")
        self.assertTrue(
            any(
                issue["code"] == "missing_file_receipt"
                for issue in result["unresolved"]
            )
        )

    def test_sound_payload_content_key_mismatch_remains_a_gap(self):
        raw = b"OggSactualsound"
        self.put("sounds/30.ogg", raw)
        self.receipt("sounds/30.ogg", 30, "ogg", raw)
        self.ledger[0]["content_key"] = "c" * 32
        self.put(
            "provenance/closure-extract.jsonl",
            (json.dumps(self.ledger[0]) + "\n").encode(),
        )
        graph = self.graph()
        graph.add(30, "ogg", "authored SoundKit")
        result = graph.run()
        self.assertEqual(result["assets"][0]["identity_status"], "unverified")
        self.assertTrue(
            any(
                issue["code"] == "file_receipt_mismatch"
                for issue in result["unresolved"]
            )
        )

    def test_historical_ledger_rows_do_not_override_matching_actual_bytes(self):
        raw = b"BLP2legacy"
        graph = self.flat_texture_graph(raw)
        self.receipt("textures/12.blp", 12, "blp", raw, build="12.1.0.69933")
        self.receipt(
            "textures/12.blp", 12, "blp", b"BLP2different", build="12.1.5.70448"
        )
        texture = next(row for row in graph.run()["assets"] if row["type"] == "blp")
        self.assertEqual(texture["identity_status"], "verified_receipt")
        self.assertEqual(texture["actual_build"], "12.1.0.69933")

    def test_hash_only_icon_inventory_never_authenticates_product_origin(self):
        raw = b"BLP2legacy"
        graph = self.flat_texture_graph(raw)
        self.put(
            "cache/ui-icon-provenance.json",
            json.dumps(
                {
                    "icons": [
                        {
                            "fdid": 12,
                            "bytes": len(raw),
                            "sha256": hashlib.sha256(raw).hexdigest(),
                        }
                    ]
                }
            ).encode(),
        )
        result = graph.run()
        texture = next(row for row in result["assets"] if row["type"] == "blp")
        self.assertEqual(texture["identity_status"], "unverified")
        self.assertTrue(
            any(
                issue["code"] == "missing_file_receipt"
                for issue in result["unresolved"]
            )
        )

    def test_spell_icons_audit_native_flat_paths_not_model_index_paths(self):
        raw = b"ID,SpellID,DifficultyID,SpellIconFileDataID,ActiveIconFileDataID\n1,100,0,12,0\n"
        self.put("db2/fixture/SpellMisc.csv", raw)
        self.put(
            "db2/fixture/receipt.json",
            json.dumps(
                {
                    "identity": {
                        "product": "wow",
                        "build": "12.1.5.70448",
                        "build_key": "a" * 32,
                    },
                    "tables": {
                        "SpellMisc": {
                            "fdid": 42,
                            "content_authenticated": True,
                            "csv_sha256": hashlib.sha256(raw).hexdigest(),
                            "unreadable_records": 0,
                        }
                    },
                }
            ).encode(),
        )
        icon = b"BLP2icon"
        self.put("textures/12.blp", icon)
        self.receipt("textures/12.blp", 12, "blp", icon)
        graph = self.graph()
        seed_icons(graph, Catalogs(graph), [100])
        asset = graph.run()["assets"][0]
        self.assertTrue(asset["present"])
        self.assertFalse(asset["receipt_selected"])
        self.assertEqual(asset["identity_status"], "verified_receipt")

    def test_malformed_generic_build_key_cannot_authenticate_flat_bytes(self):
        raw = b"OggSactualsound"
        self.put("sounds/30.ogg", raw)
        row = self.receipt("sounds/30.ogg", 30, "ogg", raw)
        row["build_config"] = "not-a-build-key"
        self.put("provenance/closure-extract.jsonl", (json.dumps(row) + "\n").encode())
        graph = self.graph()
        graph.add(30, "ogg", "authored SoundKit")
        result = graph.run()
        self.assertEqual(result["assets"][0]["identity_status"], "unverified")
        self.assertTrue(
            any(
                issue["code"] == "file_receipt_mismatch"
                for issue in result["unresolved"]
            )
        )

    def test_malformed_db2_build_key_cannot_authenticate_export(self):
        document = self.table_receipt(b"ID,Directory\n1,azeroth\n")
        document["identity"]["build_key"] = "not-a-build-key"
        self.put("db2/fixture/receipt.json", json.dumps(document).encode())
        graph = self.graph()
        self.assertEqual(Catalogs(graph).rows("Map"), [])
        self.assertTrue(
            any(code == "metadata_receipt_mismatch" for code, _, _ in graph.unresolved)
        )

    def test_db2_without_export_receipt_is_not_silently_used(self):
        self.put("db2/fixture/Map.csv", b"ID,Directory\n1,azeroth\n")
        graph = self.graph()
        self.assertEqual(Catalogs(graph).rows("Map"), [])
        self.assertTrue(
            any(code == "missing_metadata_receipt" for code, _, _ in graph.unresolved)
        )

    def test_db2_table_csv_hash_and_product_receipt_qualify_metadata(self):
        raw = b"ID,Directory,WdtFileDataID\n1,azeroth,10\n"
        self.table_receipt(raw)
        graph = self.graph()
        self.assertEqual(Catalogs(graph).rows("Map")[0]["ID"], "1")
        entry = graph.inputs["db2/fixture/Map.csv"]
        self.assertEqual(entry["identity_status"], "verified_export")
        self.assertEqual(entry["actual_build"], "12.1.5.70448")
        self.assertEqual(entry["source_product"], "wow")

    def test_changed_db2_csv_is_not_parsed_as_the_receipted_table(self):
        self.table_receipt(b"ID,Directory\n1,azeroth\n")
        self.put("db2/fixture/Map.csv", b"ID,Directory\n9,changed\n")
        graph = self.graph()
        self.assertEqual(Catalogs(graph).rows("Map"), [])
        self.assertTrue(
            any(code == "metadata_receipt_mismatch" for code, _, _ in graph.unresolved)
        )

    def test_wrong_product_db2_receipt_is_not_a_source_fallback(self):
        self.table_receipt(b"ID,Directory\n1,azeroth\n", product="wow_classic_beta")
        graph = self.graph()
        self.assertEqual(Catalogs(graph).rows("Map"), [])
        self.assertTrue(
            any(code == "metadata_product_mismatch" for code, _, _ in graph.unresolved)
        )

    def test_unreadable_db2_records_keep_partial_export_unresolved(self):
        self.table_receipt(b"ID,Directory\n1,azeroth\n", unreadable=3)
        graph = self.graph()
        self.assertEqual(len(Catalogs(graph).rows("Map")), 1)
        self.assertTrue(
            any(
                code == "metadata_unreadable_records" for code, _, _ in graph.unresolved
            )
        )
        self.assertEqual(
            graph.inputs["db2/fixture/Map.csv"]["identity_status"],
            "authenticated_partial_export",
        )


if __name__ == "__main__":
    unittest.main()
