"""Behavioral WDC5 fixtures for pinned Retail cache inputs."""
import csv
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# table, layout, stored values, expected CSV row (ID is a non-inline fixture ID).
CASES = [
    ("ChrModel", 0x03FAB755, [0, 0, 1, 0, 101, 103, 0, 0, 0, 0, 0, 0x3F800000, 0, 0xBEAE147B, 0, 0, 0],
     {"ID": "1", "Sex": "0", "DisplayID": "101", "CharComponentTextureLayoutID": "103", "CustomizeScale": "1", "CameraDistanceOffset": "-0.340000004"}),
    ("ChrCustomizationOption", 0xDCC2A86E, [0, 890, 0, 0, 1, 10, 3, 0, 0, 0, 12, 0, 0],
     {"ID": "890", "Name_lang": "", "ChrModelID": "1", "OrderIndex": "10", "ChrCustomizationCategoryID": "3", "OptionType": "0", "Requirement": "12"}),
    ("ChrCustomizationChoice", 0x9559C358, [0, 90001, 890, 19, 23, 2, 0, 0, 0, 0, 0xFF000001 | (0xFF000002 << 32)],
     {"ID": "90001", "Name_lang": "", "ChrCustomizationOptionID": "890", "ChrCustomizationReqID": "19", "ChrCustomizationVisReqID": "23", "OrderIndex": "2", "SwatchColor_0": "-16777215", "SwatchColor_1": "-16777214"}),
    ("ChrCustomizationGeoset", 0xBF55F2FB, [32, 2, 0],
     {"ID": "1", "GeosetType": "32", "GeosetID": "2"}),
    ("CharHairGeosets", 0x5B2ACAEF, [1, 0, 0, 7, 1, 0, 2, 0, 0, 0, 0],
     {"ID": "1", "RaceID": "1", "SexID": "0", "GeosetID": "7", "Showscalp": "1", "GeosetType": "2"}),
    ("ChrCustomizationCategory", 0x1605B68A, [0, 11985, 11984, 3, 0, 0, 0, 0],
     {"ID": "1", "CategoryName_lang": "", "CustomizeIcon": "11985", "CustomizeIconSelected": "11984", "OrderIndex": "3"}),
    ("CharComponentTextureLayouts", 0x0B9AF134, [2048, 1024],
     {"ID": "1", "Width": "2048", "Height": "1024"}),
    ("CharComponentTextureSections", 0x2173BA71, [103, 2, 16, 32, 128, 256, 0],
     {"ID": "1", "CharComponentTextureLayoutID": "103", "SectionType": "2", "X": "16", "Y": "32", "Width": "128", "Height": "256"}),
    ("ItemDisplayInfoMaterialRes", 0xAA462C0E, [2, 101],
     {"ID": "1", "ComponentSection": "2", "MaterialResourcesID": "101", "ItemDisplayInfoID": "700"}),
]


def fixture(layout, values):
    widths = [64 if value > 0xFFFFFFFF else 32 for value in values]
    size = sum(widths) // 8
    start = 204 + 40 + len(values) * 28
    raw = bytearray(start)
    raw[:4] = b"WDC5"
    struct.pack_into("<6I", raw, 136, 1, len(values), size, 0, 0, layout)
    struct.pack_into("<HH7I", raw, 172, 4, 0, len(values), 0, len(values) * 24, 0, 0, 0, 1)
    relation = struct.pack("<5I", 1, 700, 700, 700, 0)
    struct.pack_into("<Q8I", raw, 204, 0, start, 1, 0, start + size, 4, len(relation), 0, 0)
    offset = 0
    record = bytearray()
    for index, (value, width) in enumerate(zip(values, widths)):
        struct.pack_into("<HH5I", raw, 244 + len(values) * 4 + index * 24, offset, width, 0, 0, 0, 0, 0)
        record += value.to_bytes(width // 8, "little")
        offset += width
    row_id = values[1] if layout in (0xDCC2A86E, 0x9559C358) else 1
    return raw + record + struct.pack("<I", row_id) + relation


class CacheImportExportTests(unittest.TestCase):
    def test_exports_concrete_customization_texture_and_material_records(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "input.db2"
            output = Path(directory) / "out.csv"
            for table, layout, values, expected in CASES:
                with self.subTest(table=table):
                    source.write_bytes(fixture(layout, values))
                    result = subprocess.run([sys.executable, str(ROOT / "scripts/export_db2_csv.py"), table, str(source), str(output)], capture_output=True, text=True)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    with output.open() as handle:
                        self.assertEqual(list(csv.DictReader(handle)), [expected])


if __name__ == "__main__":
    unittest.main()
