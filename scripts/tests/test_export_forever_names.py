import csv
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from scripts import export_db2_csv as export
from scripts import import_forever_skyborne as importer


class ForeverNameTests(unittest.TestCase):
    def test_registered_schema_decodes_real_first_names_and_surnames(self):
        self.assertEqual(importer.TABLES.get("NameGen"), 1122117)
        self.assertIn("NameGen", importer.NEW_TABLES)
        layout, columns = export.TABLES["NameGen"]
        raw = (importer.PROBE_DIRECTORY / "1122117.db2").read_bytes()
        rows, dropped = importer.decode_rows(raw, layout, columns, 0)
        self.assertEqual((len(rows), dropped), (2741, 0))
        self.assertEqual(
            next(row for row in rows if row["ID"] == 24985),
            {"ID": 24985, "Name": "Faladiel", "RaceID": 95, "Sex": 0, "NameType": 0},
        )
        self.assertTrue(any(row["NameType"] == 1 for row in rows))
        encoded, missing = importer.encode_csv(b"", columns, rows, table="NameGen")
        self.assertEqual(missing, [])
        decoded = list(csv.DictReader(encoded.decode().splitlines()))
        self.assertEqual(len(decoded), 2741)
        self.assertEqual(decoded[0].keys(), {"ID", "Name", "RaceID", "Sex", "NameType"})

    def test_names_only_command_exports_pinned_source_and_provenance(self):
        source = importer.PROBE_DIRECTORY / "1122117.db2"
        definition = importer.PROBE_DIRECTORY.parent / "definitions/NameGen.dbd"
        repo = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as temporary:
            data = Path(temporary)
            probe = data / f"forever-{importer.BUILD}/skyborne-probe"
            definitions = probe.parent / "definitions"
            probe.mkdir(parents=True)
            definitions.mkdir()
            (probe / source.name).write_bytes(source.read_bytes())
            (definitions / definition.name).write_bytes(definition.read_bytes())
            retail = data / "NameGen.csv"
            retail.write_text("ID,Name,RaceID,Sex\n1,Anduin,1,0\n")
            result = subprocess.run(
                [
                    sys.executable,
                    str(repo / "scripts/export_forever_names.py"),
                    "--data",
                    str(data),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(retail.read_text(), "ID,Name,RaceID,Sex\n1,Anduin,1,0\n")
            output = data / "db2" / importer.BUILD
            with (output / "NameGen.csv").open(newline="") as handle:
                rows = list(csv.DictReader(handle))
            self.assertEqual(len(rows), 2741)
            self.assertEqual(
                {
                    (race, sex): sum(
                        row["RaceID"] == str(race) and row["Sex"] == str(sex)
                        for row in rows
                    )
                    for race in (95, 96)
                    for sex in (0, 1)
                },
                {(95, 0): 139, (95, 1): 133, (96, 0): 137, (96, 1): 128},
            )
            provenance = json.loads((output / "NameGen.provenance.json").read_text())
            self.assertEqual(
                provenance["sha256"], hashlib.sha256(source.read_bytes()).hexdigest()
            )
            self.assertEqual(
                provenance["dbd_sha256"],
                hashlib.sha256(definition.read_bytes()).hexdigest(),
            )
            self.assertEqual(
                provenance["csv_sha256"],
                hashlib.sha256((output / "NameGen.csv").read_bytes()).hexdigest(),
            )
            self.assertEqual(provenance["LayoutHash"], "584300FA")
            self.assertEqual(provenance["encrypted_rows_dropped"], 0)
            self.assertEqual(provenance["rows_decoded"], 2741)
            self.assertEqual(
                provenance["columns"], ["ID", "Name", "RaceID", "Sex", "NameType"]
            )
            self.assertEqual(provenance["build"], importer.BUILD)
            self.assertEqual(provenance["fdid"], 1122117)

            # A mismatched source must not replace the accepted export.
            accepted = (output / "NameGen.csv").read_bytes()
            (probe / source.name).write_bytes(source.read_bytes() + b"corrupt")
            failed = subprocess.run(
                result.args, capture_output=True, text=True, check=False
            )
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn("SHA256", failed.stderr)
            self.assertEqual((output / "NameGen.csv").read_bytes(), accepted)
