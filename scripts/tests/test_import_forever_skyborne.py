import csv
import io
import struct
import unittest

from scripts import export_db2_csv as export


def string_fixture(encrypted=False):
    data = bytearray(400)
    data[:4] = b"WDC5"
    struct.pack_into("<6I", data, 136, 2, 1, 4, 5, 0, 123)
    struct.pack_into("<HH7I", data, 172, 4, 0, 1, 0, 24, 0, 0, 0, 2)
    struct.pack_into("<Q8I", data, 204, 0, 320, 1, 3, 324, 4, 0, 0, 0)
    struct.pack_into("<Q8I", data, 244, int(encrypted), 350, 1, 2, 354, 4, 0, 0, 0)
    struct.pack_into("<HH5I", data, 288, 0, 32, 0, 0, 0, 0, 0)
    # Global record 0: -8 + 8 = string logical offset 0.
    # Global record 1: -4 + 7 = string logical offset 3.
    struct.pack_into("<I", data, 320, 8)
    data[324:327] = b"AB\0"
    struct.pack_into("<I", data, 327, 10)
    struct.pack_into("<I", data, 350, 7)
    data[354:356] = b"C\0"
    struct.pack_into("<I", data, 356, 11)
    return bytes(data)


class StringTests(unittest.TestCase):
    def test_global_string_offsets(self):
        data = string_fixture()
        rows, dropped, fields, _ = export.read_wdc5(data, 123)
        self.assertEqual(dropped, 0)
        self.assertEqual(
            [
                export.read_string(data, row[2], fields[0], row[0][0])
                for row in rows.values()
            ],
            ["AB", "C"],
        )

    def test_null_string_offset_is_empty(self):
        data = string_fixture()
        _, _, fields, _ = export.read_wdc5(data, 123)
        self.assertEqual(export.read_string(data, 320, fields[0], 0), "")

    def test_zero_filled_encrypted_records_are_dropped(self):
        data = bytearray(string_fixture(True))
        data[350:354] = bytes(4)
        rows, dropped, _, _ = export.read_wdc5(bytes(data), 123)
        self.assertEqual(list(rows), [10])
        self.assertEqual(dropped, 1)

    def test_encrypted_string_block_rejected(self):
        data = string_fixture(True)
        rows, _, fields, _ = export.read_wdc5(data, 123)
        with self.assertRaisesRegex(ValueError, "encrypted"):
            export.read_string(data, rows[11][2], fields[0], rows[11][0][0])


class ImportTests(unittest.TestCase):
    def test_local_golden_races_and_zephras_identity(self):
        from pathlib import Path
        from scripts import import_forever_skyborne as importer

        data = Path(__file__).resolve().parents[2] / "data"
        staging = data / "cache/forever-skyborne-extract"
        if not (staging / "1305311.db2").is_file():
            self.skipTest("local CASC fixtures unavailable")
        decoded = {}
        for table in ("ChrRaces", "Map"):
            raw = importer.extracted_path(staging, importer.TABLES[table]).read_bytes()
            layout = struct.unpack_from("<I", raw, 156)[0]
            columns, id_field, _ = importer.parse_definition(
                importer.find_definition(data, table).read_text(), layout
            )
            rows, _ = importer.decode_rows(raw, layout, columns, id_field)
            decoded[table] = {int(row["ID"]): row for row in rows}
        with (data / "forever-1.60.1.70205/metadata/ChrRaces-95-96.csv").open(
            newline=""
        ) as handle:
            for golden in csv.DictReader(handle):
                actual = decoded["ChrRaces"][int(golden["ID"])]
                self.assertEqual({key: str(actual[key]) for key in golden}, golden)
        zephras = decoded["Map"][2991]
        self.assertEqual(
            (zephras["Directory"], zephras["MapName_lang"], zephras["WdtFileDataID"]),
            ("2991", "Zephras Isle", 7198644),
        )
        self.assertEqual(decoded["Map"][0]["Directory"], "Azeroth")

    def test_dbd_inline_id_arrays_and_relation(self):
        from scripts import import_forever_skyborne as importer

        dbd = """COLUMNS
int ID
int Values
int Parent
string Name

LAYOUT 0000007B
BUILD 1.60.1.70205
$id$ID<32>
Values<u16>[2]
Name
$noninline,relation$Parent<32>
"""
        columns, id_field, explicit = importer.parse_definition(dbd, 123)
        self.assertEqual(id_field, 0)
        self.assertTrue(explicit)
        self.assertEqual(
            columns,
            [
                ("ID", "id"),
                ("Values_0", ("u16", 1, 0)),
                ("Values_1", ("u16", 1, 1)),
                ("Name", ("string", 2)),
                ("Parent", "parent"),
            ],
        )

    def test_version_one_bone_chunks(self):
        from scripts import import_forever_skyborne as importer

        raw = struct.pack("<I4sI2H", 1, b"BIDA", 4, 58, 59)
        importer.validate_magic(raw, "bone")
        with self.assertRaisesRegex(ValueError, "truncated"):
            importer.validate_magic(raw[:-1], "bone")

    def test_raw_afid_animation_requires_content_identity(self):
        import hashlib
        from scripts import import_forever_skyborne as importer

        raw = struct.pack("<4I", 0, 34, 67, 100)
        importer.validate_magic(raw, "anim", hashlib.md5(raw).hexdigest())
        with self.assertRaisesRegex(ValueError, "content"):
            importer.validate_magic(raw, "anim", "0" * 32)
        with self.assertRaisesRegex(ValueError, "content"):
            importer.validate_magic(raw, "anim")

    def test_chunk_closure(self):
        from scripts import import_forever_skyborne as importer

        def chunk(tag, body):
            return tag + struct.pack("<I", len(body)) + body

        data = chunk(b"MD21", b"header") + chunk(b"SFID", struct.pack("<II", 20, 21))
        data += chunk(b"AFID", struct.pack("<HHI", 1, 0, 30))
        data += chunk(b"BFID", struct.pack("<I", 40)) + chunk(
            b"TXID", struct.pack("<II", 0, 50)
        )
        self.assertEqual(
            importer.asset_references(data),
            {"skin": [20, 21], "anim": [30], "bone": [40], "blp": [50]},
        )

    def test_customization_reachability(self):
        from scripts import import_forever_skyborne as importer

        tables = {
            "ChrCustomizationOption": [
                {"ID": "1", "ChrModelID": "218"},
                {"ID": "2", "ChrModelID": "1"},
            ],
            "ChrCustomizationChoice": [{"ID": "10", "ChrCustomizationOptionID": "1"}],
            "ChrCustomizationElement": [
                {
                    "ChrCustomizationChoiceID": "10",
                    "ChrCustomizationMaterialID": "3",
                    "ChrCustomizationSkinnedModelID": "4",
                }
            ],
            "ChrCustomizationMaterial": [{"ID": "3", "MaterialResourcesID": "5"}],
            "TextureFileData": [
                {"MaterialResourcesID": "5", "FileDataID": "60"},
                {"MaterialResourcesID": "6", "FileDataID": "61"},
            ],
            "ChrCustomizationSkinnedModel": [
                {"ID": "4", "CollectionsFileDataID": "70"}
            ],
        }
        self.assertEqual(importer.customization_assets(tables), ({60}, {70}))

    def test_map_identity_columns_lead_new_header(self):
        from scripts import import_forever_skyborne as importer

        columns = [
            (name, "id")
            for name in ("ID", "Directory", "MapName_lang", "Flags", "WdtFileDataID")
        ]
        row = {
            "ID": 2991,
            "Directory": "2991",
            "MapName_lang": "Zephras Isle",
            "Flags": 1,
            "WdtFileDataID": 7198644,
        }
        output, missing = importer.encode_csv(b"", columns, [row], table="Map")
        self.assertEqual(
            output,
            b"ID,Directory,MapName_lang,WdtFileDataID,Flags\r\n2991,2991,Zephras Isle,7198644,1\r\n",
        )
        self.assertEqual(missing, [])

    def test_retail_header_preserved_and_missing_column_reported(self):
        from scripts import import_forever_skyborne as importer

        header = b"ID,Name,Absent\r\n"
        output, missing = importer.encode_csv(
            header, [("ID", "id"), ("Name", 0)], [{"ID": 95, "Name": "Skyborne"}]
        )
        self.assertTrue(output.startswith(header))
        self.assertEqual(missing, ["Absent"])
        self.assertEqual(
            list(csv.DictReader(io.StringIO(output.decode()))),
            [{"ID": "95", "Name": "Skyborne", "Absent": ""}],
        )


if __name__ == "__main__":
    unittest.main()
