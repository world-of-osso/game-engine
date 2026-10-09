import csv
import hashlib
import json
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from scripts import export_db2_csv as export
from scripts import import_forever_skyborne as importer


# Concrete WDC5 records: first names and surnames for both Skyborne races/sexes.
NAME_RECORDS = [
    (24985, "Faladiel", 95, 0, 0),
    (24986, "Dawnwing", 95, 0, 1),
    (24987, "Ailee", 95, 1, 0),
    (24988, "Sunweaver", 95, 1, 1),
    (24989, "Galen", 96, 0, 0),
    (24990, "Windcaller", 96, 0, 1),
    (24991, "Liora", 96, 1, 0),
    (24992, "Cloudsong", 96, 1, 1),
]
NAME_DEFINITION = b"""COLUMNS
int ID
string Name
int RaceID
int Sex
int NameType

LAYOUT 584300FA
BUILD 1.60.1.70205
$noninline,id$ID<32>
Name
RaceID<8>
Sex<8>
NameType<u8>
"""


def namegen_fixture():
    count = len(NAME_RECORDS)
    record_size = 7  # string offset (u32), race (i8), sex (i8), type (u8)
    field_count = 4
    record_start = 204 + 40 + field_count * (4 + 24)
    strings = bytearray()
    records = bytearray()
    ids = bytearray()
    for index, (row_id, name, race, sex, name_type) in enumerate(NAME_RECORDS):
        string_offset = (count - index) * record_size + len(strings)
        records += struct.pack("<IbbB", string_offset, race, sex, name_type)
        strings += name.encode() + b"\0"
        ids += struct.pack("<I", row_id)
    raw = bytearray(record_start)
    raw[:4] = b"WDC5"
    layout, _ = export.TABLES["NameGen"]
    struct.pack_into(
        "<6I", raw, 136, count, field_count, record_size, len(strings), 0, layout
    )
    struct.pack_into(
        "<HH7I", raw, 172, 4, 0, field_count, 0, field_count * 24, 0, 0, 0, 1
    )
    struct.pack_into(
        "<Q8I", raw, 204, 0, record_start, count, len(strings),
        record_start + len(records), len(ids), 0, 0, 0,
    )
    field_start = 204 + 40 + field_count * 4
    for index, (offset, width) in enumerate([(0, 32), (32, 8), (40, 8), (48, 8)]):
        struct.pack_into(
            "<HH5I", raw, field_start + index * 24, offset, width, 0, 0, 0, 0, 0
        )
    return bytes(raw + records + strings + ids)


class ForeverNameTests(unittest.TestCase):
    def test_registered_schema_decodes_first_names_and_surnames(self):
        self.assertEqual(importer.TABLES.get("NameGen"), 1122117)
        self.assertIn("NameGen", importer.NEW_TABLES)
        layout, columns = export.TABLES["NameGen"]
        rows, dropped = importer.decode_rows(namegen_fixture(), layout, columns, 0)
        expected = [
            dict(zip([name for name, _ in columns], record))
            for record in NAME_RECORDS
        ]
        self.assertEqual((rows, dropped), (expected, 0))
        encoded, missing = importer.encode_csv(b"", columns, rows, table="NameGen")
        self.assertEqual(missing, [])
        decoded = list(csv.DictReader(encoded.decode().splitlines()))
        self.assertEqual(
            decoded,
            [{key: str(value) for key, value in row.items()} for row in expected],
        )

    def test_names_only_command_exports_pinned_source_and_provenance(self):
        raw = namegen_fixture()
        source_sha256 = hashlib.sha256(raw).hexdigest()
        definition_sha256 = hashlib.sha256(NAME_DEFINITION).hexdigest()
        repo = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as temporary:
            data = Path(temporary)
            probe = data / f"forever-{importer.BUILD}/skyborne-probe"
            definitions = probe.parent / "definitions"
            probe.mkdir(parents=True)
            definitions.mkdir()
            source = probe / "1122117.db2"
            source.write_bytes(raw)
            definition = definitions / "NameGen.dbd"
            definition.write_bytes(NAME_DEFINITION)
            retail = data / "NameGen.csv"
            retail.write_text("ID,Name,RaceID,Sex\n1,Anduin,1,0\n")
            # Pin the concrete fixture in the child process; production pins stay unchanged.
            command = [
                sys.executable,
                "-c",
                "import sys; from scripts import export_forever_names as exporter; "
                "exporter.SOURCE_SHA256 = sys.argv.pop(1); "
                "exporter.DBD_SHA256 = sys.argv.pop(1); "
                "raise SystemExit(exporter.main())",
                source_sha256,
                definition_sha256,
                "--data",
                str(data),
            ]
            result = subprocess.run(
                command, cwd=repo, capture_output=True, text=True, check=False
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(retail.read_text(), "ID,Name,RaceID,Sex\n1,Anduin,1,0\n")
            output = data / "db2" / importer.BUILD
            with (output / "NameGen.csv").open(newline="") as handle:
                rows = list(csv.DictReader(handle))
            self.assertEqual(len(rows), len(NAME_RECORDS))
            self.assertEqual(
                {
                    (race, sex): sum(
                        row["RaceID"] == str(race) and row["Sex"] == str(sex)
                        for row in rows
                    )
                    for race in (95, 96)
                    for sex in (0, 1)
                },
                {(95, 0): 2, (95, 1): 2, (96, 0): 2, (96, 1): 2},
            )
            provenance = json.loads((output / "NameGen.provenance.json").read_text())
            self.assertEqual(provenance["sha256"], source_sha256)
            self.assertEqual(provenance["dbd_sha256"], definition_sha256)
            self.assertEqual(
                provenance["csv_sha256"],
                hashlib.sha256((output / "NameGen.csv").read_bytes()).hexdigest(),
            )
            self.assertEqual(provenance["LayoutHash"], "584300FA")
            self.assertEqual(provenance["encrypted_rows_dropped"], 0)
            self.assertEqual(provenance["rows_decoded"], len(NAME_RECORDS))
            self.assertEqual(
                provenance["columns"], ["ID", "Name", "RaceID", "Sex", "NameType"]
            )
            self.assertEqual(provenance["build"], importer.BUILD)
            self.assertEqual(provenance["fdid"], 1122117)

            # A mismatched source must not replace the accepted export.
            accepted = (output / "NameGen.csv").read_bytes()
            source.write_bytes(raw + b"corrupt")
            failed = subprocess.run(
                command, cwd=repo, capture_output=True, text=True, check=False
            )
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn("SHA256", failed.stderr)
            self.assertEqual((output / "NameGen.csv").read_bytes(), accepted)
