"""Verified original local-CASC extraction bytes are accepted; changed roots are not."""

import hashlib
import sqlite3
import tempfile
import unittest
from pathlib import Path

from scripts import import_forever_skyborne as importer


class VerifiedProbeTests(unittest.TestCase):
    def test_probe_matches_root_and_records_original_source(self):
        raw = b"MD21" + bytes(12)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            probe_dir = root / "data" / ("forever-" + importer.BUILD) / "skyborne-probe"
            probe_dir.mkdir(parents=True)
            probe = probe_dir / "8035354.dat"
            probe.write_bytes(raw)
            staging = root / "staging"
            staging.mkdir()
            with sqlite3.connect(":memory:") as connection:
                connection.execute(
                    "create table resolution(fdid integer,content_key blob)"
                )
                connection.execute(
                    "insert into resolution values(?,?)",
                    (8035354, hashlib.md5(raw).digest()),
                )
                provenance = importer.stage_verified_probes(
                    root / "data", staging, [8035354], connection
                )
                self.assertEqual((staging / "8035354.dat").read_bytes(), raw)
                self.assertEqual(
                    provenance["8035354"]["content_key"], hashlib.md5(raw).hexdigest()
                )
                self.assertEqual(provenance["8035354"]["source"], str(probe))
                self.assertEqual(
                    provenance["8035354"]["origin"], "original local CASC extraction"
                )
                connection.execute("update resolution set content_key=?", (bytes(16),))
                with self.assertRaisesRegex(ValueError, "content key mismatch"):
                    importer.stage_verified_probes(
                        root / "data", staging, [8035354], connection
                    )


if __name__ == "__main__":
    unittest.main()
