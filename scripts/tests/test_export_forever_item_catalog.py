"""Behavioral exporter proof against repository-owned 70205 fixtures."""

import csv
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from scripts.tests.forever_fixture import csv_wdc5, FIXTURE_LAYOUT

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = Path(__file__).parent / "fixtures/forever-70205/items"


class ForeverItemExportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "fixture-source"
        self.source.mkdir()
        for path in (FIXTURE / "source").glob("*.gz"):
            (self.source / path.stem).write_bytes(gzip.decompress(path.read_bytes()))
        self.definitions = self.root / "fixture-definitions"
        shutil.copytree(FIXTURE / "definitions", self.definitions)
        self.server = self.root / "server"
        shutil.copytree(FIXTURE / "server", self.server)
        schema = self.server / "scripts/db2_schema.sql.gz"
        schema.with_suffix("").write_bytes(gzip.decompress(schema.read_bytes()))
        self.pins = self.prepare_scaling()
        self.output = self.root / "db2/1.60.1.70205/items"
        self.retail = self.root / "db2/12.1.0.69933/items"
        self.retail.mkdir(parents=True)
        (self.retail / "ItemSparse.csv").write_text(
            "ID,Display_lang\n2947,Retail sentinel\n"
        )
        self.before = (self.retail / "ItemSparse.csv").read_bytes()

    def prepare_scaling(self):
        sys.path.insert(0, str(ROOT / "scripts"))
        self.addCleanup(sys.path.remove, str(ROOT / "scripts"))
        import export_forever_item_catalog as exporter

        original = json.loads((FIXTURE / "original-manifest.json").read_text())
        provenance_path = self.definitions / "schema-provenance.json"
        provenance = json.loads(provenance_path.read_text())
        pins = {"scaling": {}, "schemas": {}}
        for name, (fdid, _, _) in exporter.SCALING_SOURCES.items():
            content = (FIXTURE / "scaling" / f"{name}.csv").read_bytes()
            self.assertEqual(
                hashlib.sha256(content).hexdigest(),
                original["outputs"][f"{name}.csv"]["sha256"],
            )
            columns = next(csv.reader(content.decode().splitlines()))
            floats = set(columns) - {"ID", "ItemLevel"}
            strings = {"DisplayName_lang"} if name == "ItemSubClass" else set()
            if strings:
                floats = set()
            raw, definition = csv_wdc5(
                content, floats, strings,
                row_ids=original["outputs"][f"{name}.csv"]["row_ids"],
            )
            (self.source / f"{fdid}.db2").write_bytes(raw)
            (self.definitions / f"{name}.dbd").write_bytes(definition)
            digest = hashlib.sha256(definition).hexdigest()
            pins["scaling"][name] = [
                fdid, FIXTURE_LAYOUT, hashlib.sha256(raw).hexdigest(),
            ]
            pins["schemas"][name] = digest
            provenance[name] = {
                "sha256": digest,
                "source": "repo fixture reconstructed from verified 70205 CSV",
            }
        provenance_path.write_text(json.dumps(provenance))
        path = self.root / "fixture-pins.json"
        path.write_text(json.dumps(pins))
        return path

    def export(self, source=None, definitions=None):
        return subprocess.run(
            [
                sys.executable,
                "-c",
                "import sys, json; sys.path.insert(0, sys.argv.pop(1)); "
                "import export_forever_item_catalog as exporter; "
                "pins = json.load(open(sys.argv.pop(1))); "
                "exporter.SCALING_SOURCES = pins['scaling']; "
                "exporter.SCHEMA_SHA256 = {**exporter.SCHEMA_SHA256, **pins['schemas']}; "
                "raise SystemExit(exporter.main())",
                str(ROOT / "scripts"),
                str(self.pins),
                "--server-root",
                str(self.server),
                "--source-dir",
                str(source or self.source),
                "--definitions",
                str(definitions or self.definitions),
                "--output",
                str(self.output),
            ],
            text=True,
            capture_output=True,
            check=False,
        )

    def rows(self, name):
        with (self.output / f"{name}.csv").open(newline="") as stream:
            return list(csv.DictReader(stream))

    def test_exports_exact_kit_authored_values_and_preserves_retail(self):
        result = self.export()
        self.assertEqual(result.returncode, 0, result.stderr)
        items = {int(r["ID"]): r for r in self.rows("Item")}
        sparse = {int(r["ID"]): r for r in self.rows("ItemSparse")}
        self.assertEqual(
            set(items),
            {
                35,
                117,
                159,
                876,
                2092,
                2101,
                2504,
                2512,
                2947,
                3661,
                4536,
                4540,
                6948,
                271655,
                271658,
                271659,
                271661,
                271662,
                271663,
                271665,
                271666,
                271668,
                271669,
                271671,
                271672,
                271673,
                271674,
                271675,
                280399,
                280400,
            },
        )
        self.assertEqual(set(items), set(sparse))
        self.assertEqual(items[2947]["ClassID"], "2")
        self.assertEqual(items[2101]["ClassID"], "11")
        self.assertEqual(items[2512]["ClassID"], "6")
        self.assertEqual(sparse[2947]["Display_lang"], "Small Throwing Knife")
        self.assertEqual(sparse[2101]["Display_lang"], "Light Quiver")
        self.assertEqual(sparse[2512]["Display_lang"], "Rough Arrow")
        self.assertEqual(sparse[2947]["ItemDelay"], "2000")
        self.assertEqual(sparse[2947]["ItemLevel"], "3")
        self.assertEqual(sparse[2947]["StatModifier_bonusStat_0"], "-1")
        self.assertAlmostEqual(float(sparse[2947]["DmgVariance"]), 0.6000000238418579)
        self.assertEqual(sparse[2512]["ItemLevel"], "5")
        self.assertEqual(sparse[2512]["ItemDelay"], "3000")
        self.assertEqual(sparse[2101]["ContainerSlots"], "6")
        self.assertEqual(sparse[2512]["Stackable"], "200")
        self.assertEqual(
            list(sparse[2947]),
            "ID Display_lang OverallQualityID Stackable SellPrice Bonding RequiredLevel InventoryType ItemLevel MaxCount Description_lang ContainerSlots ExpansionID ItemDelay DmgVariance Flags_1".split()
            + [f"StatModifier_bonusStat_{i}" for i in range(10)]
            + [f"StatPercentEditor_{i}" for i in range(10)],
        )
        self.assertEqual((self.retail / "ItemSparse.csv").read_bytes(), self.before)
        appearances = {int(r["ID"]) for r in self.rows("ItemAppearance")}
        modified = self.rows("ItemModifiedAppearance")
        self.assertTrue(all(int(r["ItemID"]) in items for r in modified))
        self.assertEqual(
            appearances,
            {
                int(r["ItemAppearanceID"])
                for r in modified
                if int(r["ItemAppearanceID"])
            },
        )
        subclasses = {
            (int(r["ClassID"]), int(r["SubClassID"])) for r in self.rows("ItemSubClass")
        }
        self.assertEqual(
            subclasses,
            {(int(r["ClassID"]), int(r["SubclassID"])) for r in items.values()},
        )

    def test_exports_authored_display_info_link(self):
        result = self.export()
        self.assertEqual(result.returncode, 0, result.stderr)
        appearances = {int(row["ID"]): row for row in self.rows("ItemAppearance")}
        self.assertEqual(appearances[63224].get("ItemDisplayInfoID"), "472")
        self.assertEqual(appearances[57187].get("ItemDisplayInfoID"), "21328")

    def test_complete_scaling_authored_values_and_deterministic_provenance(self):
        result = self.export()
        self.assertEqual(result.returncode, 0, result.stderr)
        for name in (
            "ItemArmorQuality",
            "ItemArmorTotal",
            "ItemArmorShield",
            "ItemDamageOneHand",
            "ItemDamageOneHandCaster",
            "ItemDamageTwoHand",
            "ItemDamageTwoHandCaster",
        ):
            self.assertEqual(len(self.rows(name)), 100, name)
        total = {int(r["ItemLevel"]): r for r in self.rows("ItemArmorTotal")}
        self.assertEqual(total[1]["Cloth"], "18.5200005")
        self.assertEqual(total[2]["Leather"], "137.970001")
        quality = {int(r["ID"]): r for r in self.rows("ItemArmorQuality")}
        self.assertEqual(quality[1]["Qualitymod_0"], "0.899999976")
        points = {int(r["ID"]): r for r in self.rows("RandPropPoints")}
        self.assertEqual([points[2][f"GoodF_{i}"] for i in range(5)], ["1"] * 5)
        self.assertEqual(len(self.rows("RandPropPoints")), 300)
        self.assertEqual(len(self.rows("ArmorLocation")), 23)
        damage = {int(r["ItemLevel"]): r for r in self.rows("ItemDamageOneHand")}
        self.assertEqual(damage[1]["Quality_0"], "0.761455715")
        self.assertEqual(damage[2]["Quality_1"], "1.01635063")
        shield = {int(r["ItemLevel"]): r for r in self.rows("ItemArmorShield")}
        self.assertEqual(shield[1]["Quality_1"], "10")
        self.assertEqual(shield[2]["Quality_1"], "18")
        for path in (FIXTURE / "scaling").glob("*.csv"):
            self.assertEqual(
                (self.output / path.name).read_bytes(), path.read_bytes(), path.name,
            )
        manifest = json.loads((self.output / "manifest.json").read_text())
        self.assertEqual(manifest["build"], "1.60.1.70205")
        self.assertEqual(len(manifest["selected_item_ids"]), 30)
        self.assertEqual(manifest["selected_levels"], [1, 2])
        self.assertEqual(manifest["authored_item_levels"], [1, 2, 3, 5])
        pin = manifest["sources"]["ItemSparse"]
        self.assertEqual(pin["fdid"], 1572924)
        self.assertEqual(pin["layout"], "6FCC3191")
        self.assertEqual(
            pin["raw_sha256"],
            "15fde0ce086da0e642011b1f99b49977b6dfa56e72cf4edc941401dddf139f6d",
        )
        self.assertEqual(
            pin["schema_sha256"],
            "ff87dafe875376c0ee5b6951ef4668aaae53a862d190cd9d3e6b708a9deafe9b",
        )
        for name, entry in manifest["outputs"].items():
            self.assertEqual(
                hashlib.sha256((self.output / name).read_bytes()).hexdigest(),
                entry["sha256"],
            )
        first = {p.name: p.read_bytes() for p in self.output.iterdir()}
        shutil.rmtree(self.output)
        result = self.export()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(first, {p.name: p.read_bytes() for p in self.output.iterdir()})

    def test_corrupt_source_rejected_before_publication_no_retail_fallback(self):
        source = self.root / "source"
        shutil.copytree(self.source, source)
        path = source / "1572924.db2"
        raw = bytearray(path.read_bytes())
        raw[-1] ^= 1
        path.write_bytes(raw)
        result = self.export(source)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SHA", result.stderr)
        self.assertFalse(self.output.exists())
        self.assertEqual((self.retail / "ItemSparse.csv").read_bytes(), self.before)

    def test_schema_corruption_and_missing_source_do_not_publish(self):
        definitions = self.root / "definitions"
        shutil.copytree(self.definitions, definitions)
        with (definitions / "ItemArmorTotal.dbd").open("a") as stream:
            stream.write("\nCORRUPTED\n")
        result = self.export(definitions=definitions)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SHA", result.stderr)
        self.assertFalse(self.output.exists())
        source = self.root / "source"
        shutil.copytree(self.source, source)
        (source / "841626.db2").unlink()
        result = self.export(source)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("841626", result.stderr)
        self.assertFalse(self.output.exists())

    def test_missing_critical_field_fails_without_default_value(self):
        sys.path.insert(0, str(ROOT / "scripts"))
        self.addCleanup(sys.path.remove, str(ROOT / "scripts"))
        import export_forever_item_catalog as exporter

        with self.assertRaisesRegex(
            ValueError, "missing critical fields.*IconFileDataID"
        ):
            exporter.encode_contract(
                "Item", [{"ID": 2947, "ClassID": 2, "SubclassID": 16, "SheatheType": 0}]
            )

    def test_staging_write_failure_leaves_no_published_or_partial_directory(self):
        from unittest.mock import patch

        sys.path.insert(0, str(ROOT / "scripts"))
        self.addCleanup(sys.path.remove, str(ROOT / "scripts"))
        import export_forever_item_catalog as exporter

        original = Path.write_bytes

        def fail_manifest(path, content):
            if path.name == "manifest.json":
                raise OSError("fixture disk failure")
            return original(path, content)

        with patch.object(Path, "write_bytes", fail_manifest):
            with self.assertRaisesRegex(OSError, "fixture disk failure"):
                exporter.publish_catalog(
                    self.output, {"Item.csv": b"ID\\n2947\\n", "manifest.json": b"{}"}
                )
        self.assertFalse(self.output.exists())
        self.assertEqual(list(self.output.parent.iterdir()), [])

    def test_retail_output_path_is_rejected(self):
        self.output = self.retail
        result = self.export()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Retail paths forbidden", result.stderr)
        self.assertEqual((self.retail / "ItemSparse.csv").read_bytes(), self.before)

    def test_existing_output_is_not_partially_replaced(self):
        self.output.mkdir(parents=True)
        (self.output / "manifest.json").write_text("sentinel")
        result = self.export()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(list(self.output.iterdir()), [self.output / "manifest.json"])
        self.assertEqual((self.output / "manifest.json").read_text(), "sentinel")


if __name__ == "__main__":
    unittest.main()
