"""BLTE trust chain and NPC membership boundaries, with concrete byte fixtures."""

import hashlib
import importlib.util
from pathlib import Path
import struct
import sys
import unittest
import zlib

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import closure_casc
from test_import_npc_appearance import fixture


def blte(blocks):
    header = b"BLTE" + struct.pack(">I", 12 + 24 * len(blocks))
    header += b"\x0f" + len(blocks).to_bytes(3, "big")
    for raw, size in blocks:
        header += struct.pack(">II", len(raw), size) + hashlib.md5(raw).digest()
    return header + b"".join(raw for raw, _ in blocks), hashlib.md5(header).hexdigest()


class ChunkTests(unittest.TestCase):
    def authenticate(self, raw, key, size):
        fn = getattr(closure_casc, "authenticate_blte_chunks", None)
        self.assertIsNotNone(fn, "chunk authentication not implemented")
        return fn(raw, key, size)

    def test_plain_chunks_authenticated_without_encrypted_plaintext(self):
        encrypted = b"E\x08" + bytes.fromhex("368D53C220AE5525") + b"\x04abcdSxyz"
        raw, key = blte(
            [(b"Nhello", 5), (encrypted, 3), (b"Z" + zlib.compress(b"world"), 5)]
        )
        decoded, receipt = self.authenticate(raw, key, 13)
        self.assertEqual(decoded, b"hello\0\0\0world")
        self.assertEqual(
            [c["authenticated_plaintext"] for c in receipt["chunks"]],
            [True, False, True],
        )
        self.assertEqual(receipt["chunks"][1]["key_name"], "2555AE20C2538D36")
        self.assertEqual(
            [(c["decoded_start"], c["decoded_end"]) for c in receipt["chunks"]],
            [(0, 5), (5, 8), (8, 13)],
        )

    def test_corrupt_plain_or_encrypted_chunk_rejected(self):
        raw, key = blte([(b"Nhello", 5), (b"E\x08abcdefgh\x04abcdSxyz", 3)])
        for index in (len(raw) - 1, 12 + 48 + 3):
            corrupt = bytearray(raw)
            corrupt[index] ^= 1
            with (
                self.subTest(index=index),
                self.assertRaisesRegex(ValueError, "chunk checksum"),
            ):
                self.authenticate(bytes(corrupt), key, 8)

    def test_changed_header_rejected_even_with_matching_chunk_checksum(self):
        raw, key = blte([(b"Nhello", 5)])
        corrupt = bytearray(raw)
        corrupt[19] ^= 1
        with self.assertRaisesRegex(ValueError, "encoding key"):
            self.authenticate(bytes(corrupt), key, 5)

    def test_total_size_and_trailing_data_rejected(self):
        raw, key = blte([(b"Nhello", 5)])
        for data, size in [(raw, 6), (raw + b"x", 5)]:
            with self.assertRaises(ValueError):
                self.authenticate(data, key, size)

    def test_headerless_encoding_key_covers_entire_file(self):
        raw = b"BLTE\0\0\0\0Nhello"
        decoded, receipt = self.authenticate(raw, hashlib.md5(raw).hexdigest(), 5)
        self.assertEqual(decoded, b"hello")
        self.assertTrue(receipt["chunks"][0]["authenticated_plaintext"])


class ProfileTests(unittest.TestCase):
    def setUp(self):
        path = Path(__file__).resolve().parents[1] / "certify_npc_metadata.py"
        self.assertTrue(path.exists(), "record certification not implemented")
        spec = importlib.util.spec_from_file_location("certification", path)
        self.m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.m)

    def test_record_crossing_chunk_boundary_has_both_dependencies(self):
        data = fixture()
        split = len(data) - 9
        raw, key = blte(
            [
                (b"N" + data[:split], split),
                (b"Z" + zlib.compress(data[split:]), len(data) - split),
            ]
        )
        decoded, receipt = closure_casc.authenticate_blte_chunks(raw, key, len(data))
        table = self.m.map_table(decoded, "extra", receipt)
        self.assertEqual(table["records"][17]["chunks"], [0, 1])
        self.assertTrue(table["records"][17]["authenticated"])
        self.assertEqual(table["rows"][17][0][:3], (17, 1, 0))

    def test_missing_section_records_mapped_without_trusting_zero_filled_parents(self):
        data = bytearray(fixture(0x5E539080, True, encrypted=True))
        start = struct.unpack_from("<I", data, 212)[0]
        data[start:start] = struct.pack("<3I", 2, 101, 102)
        struct.pack_into("<I", data, 212, start + 12)
        start += 12
        encrypted = b"E\x08abcdefgh\x04abcdSxyz"
        raw, key = blte(
            [
                (b"N" + data[:start], start),
                (encrypted, 4),
                (b"N" + data[start + 4 :], len(data) - start - 4),
            ]
        )
        decoded, receipt = closure_casc.authenticate_blte_chunks(raw, key, len(data))
        table = self.m.map_table(decoded, "geoset", receipt)
        self.assertEqual(table["blocked_sections"][0]["record_ids"], [101, 102])
        self.assertEqual(table["blocked_sections"][0]["parents"], [17, 18])
        self.assertFalse(table["sections"][0]["authenticated"])
        self.assertEqual(table["rows"], {})

    def test_summary_does_not_label_unknown_membership_as_known_required(self):
        summarize = getattr(self.m, "summarize_profiles", None)
        self.assertIsNotNone(summarize, "profile gap summary not implemented")
        profiles = [
            {"display": 42, "status": "membership_unresolved"},
            {"display": 43, "status": "missing_required_records"},
            {"display": 44, "status": "certified"},
        ]
        self.assertEqual(
            summarize(profiles),
            {
                "certified_profiles": [44],
                "known_required_gaps": [43],
                "membership_unresolved_profiles": [42],
            },
        )

    def test_certified_publication_adds_coverage_and_backup_collision_preserves_cache(
        self,
    ):
        import tempfile
        import import_npc_appearance as npc

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cache = root / "npc_appearance.sqlite"
            npc.write_database(cache, ([], [], [], [(7, 0)]))
            profiles = [{"display": 42, "status": "certified"}]
            rows = ([], [], [], [(42, 0)])
            self.m.publish_cache(cache, rows, profiles, root / "backup.sqlite")
            self.assertEqual(
                npc.read_sqlite(
                    cache, "SELECT * FROM display_coverage ORDER BY display_id"
                ),
                [(7, 0), (42, 0)],
            )
            before = cache.read_bytes()
            with self.assertRaises(FileExistsError):
                self.m.publish_cache(cache, rows, profiles, root / "backup.sqlite")
            self.assertEqual(cache.read_bytes(), before)
            self.assertFalse(cache.with_name(cache.name + ".chunk-staged").exists())

    def test_gapped_profile_never_adds_runtime_coverage_on_publication(self):
        import tempfile
        import contextlib
        import sqlite3
        import import_npc_appearance as npc

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cache = root / "npc_appearance.sqlite"
            npc.write_database(cache, ([], [], [], [(7, 0)]))
            before = cache.read_bytes()
            rows = ([(42, 1, 0, 2, 501)], [(42, 9)], [(42, 3, 2)], [(42, 1)])
            profiles = [{"display": 42, "status": "membership_unresolved"}]
            publish = getattr(self.m, "publish_cache", None)
            self.assertIsNotNone(publish, "certified cache publication not implemented")
            proof = publish(cache, rows, profiles, root / "backup.sqlite")
            self.assertEqual((root / "backup.sqlite").read_bytes(), before)
            with contextlib.closing(sqlite3.connect(cache)) as conn:
                self.assertEqual(
                    conn.execute("SELECT * FROM appearances").fetchall(),
                    [(42, 1, 0, 2, 501)],
                )
                self.assertEqual(
                    conn.execute("SELECT * FROM display_coverage").fetchall(), [(7, 0)]
                )
            self.assertEqual(
                proof["after_sha256"], hashlib.sha256(cache.read_bytes()).hexdigest()
            )

    def test_authenticated_profile_and_positive_dependency_list(self):
        tables = {
            "extra": {
                "records": {17: {"authenticated": True}},
                "rows": {17: ((17,), None)},
                "blocked_sections": [],
            },
            "option": {
                "records": {101: {"authenticated": True}},
                "rows": {101: ((3, 9), 17)},
                "blocked_sections": [],
            },
            "geoset": {
                "records": {201: {"authenticated": True}},
                "rows": {201: ((3, 2), 42)},
                "blocked_sections": [],
            },
        }
        result = self.m.certify_profile(42, 17, tables)
        self.assertEqual(result["status"], "certified")
        self.assertEqual(
            result["required_records"],
            {"extra": [17], "option": [101], "geoset": [201]},
        )
        tables["geoset"]["blocked_sections"] = [
            {"section": 1, "record_ids": [999], "parents": None}
        ]
        result = self.m.certify_profile(42, 17, tables)
        self.assertEqual(result["status"], "membership_unresolved")
        self.assertEqual(
            result["gaps"][0]["reason"], "encrypted_parent_membership_unknown"
        )
        self.assertFalse(result["gaps"][0]["known_required"])
        tables["geoset"]["blocked_sections"][0]["parents"] = [42]
        self.assertEqual(
            self.m.certify_profile(42, 17, tables)["status"], "missing_required_records"
        )
        tables["geoset"]["blocked_sections"][0]["parents"] = [43]
        self.assertEqual(self.m.certify_profile(42, 17, tables)["status"], "certified")


if __name__ == "__main__":
    unittest.main()
