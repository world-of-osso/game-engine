import csv
import io
import struct
import unittest
from pathlib import Path

from scripts import import_forever_skyborne as importer
from scripts import import_forever_zephras as zephras


class ForeverLiquidTests(unittest.TestCase):
    def test_string_arrays_resolve_each_element_relative_to_its_field(self):
        raw = bytearray(350)
        raw[:4] = b"WDC5"
        struct.pack_into("<6I", raw, 136, 1, 1, 8, 5, 0, 123)
        struct.pack_into("<HH7I", raw, 172, 4, 0, 1, 0, 24, 0, 0, 0, 1)
        struct.pack_into("<Q8I", raw, 204, 0, 300, 1, 5, 308, 4, 0, 0, 0)
        struct.pack_into("<HH5I", raw, 244, 0, 64, 0, 0, 0, 0, 0)
        struct.pack_into("<II", raw, 300, 8, 7)
        raw[308:313] = b"AB\0C\0"
        struct.pack_into("<I", raw, 313, 10)
        columns = [
            ("ID", "id"),
            ("Texture_0", ("string", 0, 0)),
            ("Texture_1", ("string", 0, 1)),
        ]
        rows, dropped = importer.decode_rows(bytes(raw), 123, columns, 0)
        self.assertEqual(rows, [{"ID": 10, "Texture_0": "AB", "Texture_1": "C"}])
        self.assertEqual(dropped, 0)

    def test_importer_decodes_liquids_with_retail_headers(self):
        data = Path(__file__).resolve().parents[2] / "data"
        staging = data / "cache/forever-skyborne-extract"
        for table in (
            "LiquidType",
            "LiquidObject",
            "LiquidMaterial",
            "LiquidTypeXTexture",
        ):
            fdid = importer.TABLES[table]
            raw = importer.extracted_path(staging, fdid).read_bytes()
            layout = struct.unpack_from("<I", raw, 156)[0]
            columns, id_field, _ = importer.parse_definition(
                importer.find_definition(data, table).read_text(), layout
            )
            rows, dropped = importer.decode_rows(raw, layout, columns, id_field)
            self.assertEqual(dropped, 0)
            header = (
                (data / "db2/12.1.0.69933" / f"{table}.csv")
                .read_bytes()
                .splitlines(keepends=True)[0]
            )
            encoded, missing = importer.encode_csv(header, columns, rows, table=table)
            self.assertEqual(encoded.splitlines()[0], header.strip())
            self.assertEqual(missing, [])
            exported = list(csv.DictReader(io.StringIO(encoded.decode())))
            ids = {int(row["ID"]) for row in exported}
            if table == "LiquidType":
                self.assertTrue({1251, 1279} <= ids)
            if table == "LiquidObject":
                self.assertTrue({18420, 18563, 18615, 21229} <= ids)

    def test_mh2o_texture_closure_follows_object_type_not_layer_type(self):
        from scripts import forever_liquids as liquids

        payload = bytearray(256 * 12)
        struct.pack_into("<II", payload, 0, len(payload), 1)
        payload += struct.pack("<HHff4BII", 1251, 18420, 10, 10, 0, 0, 1, 1, 0, 0)
        raw = b"O2HM" + struct.pack("<I", len(payload)) + payload
        tables = {
            "LiquidType": [{"ID": "1279", "MaterialID": "1"}],
            "LiquidMaterial": [{"ID": "1", "LVF": "0"}],
            "LiquidObject": [{"ID": "18420", "LiquidTypeID": "1279"}],
            "LiquidTypeXTexture": [
                {"ID": "1", "LiquidTypeID": "1279", "FileDataID": "555"},
                {"ID": "2", "LiquidTypeID": "1279", "FileDataID": "0"},
                {"ID": "3", "LiquidTypeID": "1251", "FileDataID": "666"},
            ],
        }
        self.assertEqual(
            liquids.asset_references(raw, tables), [(555, "textures/555.blp")]
        )
        tables["LiquidObject"] = []
        with self.assertRaisesRegex(ValueError, "LiquidObject 18420"):
            liquids.asset_references(raw, tables)
        tables["LiquidObject"] = [{"ID": "18420", "LiquidTypeID": "999"}]
        with self.assertRaisesRegex(ValueError, "LiquidType 999"):
            liquids.asset_references(raw, tables)

    def test_adt_closure_includes_liquid_textures(self):
        payload = bytearray(256 * 12)
        struct.pack_into("<II", payload, 0, len(payload), 1)
        payload += struct.pack("<HHff4BII", 1251, 0, 10, 10, 0, 0, 1, 1, 0, 0)
        raw = b"O2HM" + struct.pack("<I", len(payload)) + payload
        tables = {
            "LiquidType": [{"ID": "1251", "MaterialID": "1"}],
            "LiquidMaterial": [{"ID": "1", "LVF": "0"}],
            "LiquidObject": [],
            "LiquidTypeXTexture": [
                {"ID": "1", "LiquidTypeID": "1251", "FileDataID": "777"}
            ],
        }
        self.assertEqual(
            zephras.asset_references(raw, 1, "terrain/1.adt", {}, {}, tables),
            [(777, "textures/777.blp")],
        )
