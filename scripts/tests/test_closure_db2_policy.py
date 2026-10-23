"""Pinned partial DB2s: named unavailable-key policy, never whole-file promotion."""

import copy
import hashlib
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from closure_receipts import ReceiptClosure
from closure_seeds import Catalogs

POLICY = "DB2_ENCRYPTED_SECTION_KEY_UNAVAILABLE"
MISSING_KEY = "583C5B29BF208655"
SECOND_KEY = "46BCA0B39E7864FF"


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def partial_db2(excluded, copies=0):
    start = 204 + 40 * (1 + len(excluded))
    raw = bytearray(start + 4 * (1 + sum(count for _, count in excluded)) + 8 * copies)
    raw[:4] = b"WDC5"
    struct.pack_into(
        "<9I",
        raw,
        136,
        1 + sum(n for _, n in excluded),
        1,
        4,
        0,
        0,
        0x12345678,
        42,
        90,
        2,
    )
    struct.pack_into("<HH", raw, 172, 4, 0)
    struct.pack_into("<7I", raw, 176, 1, 0, 0, 0, 0, 0, 1 + len(excluded))
    struct.pack_into("<Q8I", raw, 204, 0, start, 1, 0, 0, 0, 0, 0, 0)
    struct.pack_into("<I", raw, start, 42)
    start += 4
    warnings = []
    for index, (key, count) in enumerate(excluded, 1):
        section_copies = copies if index == 1 else 0
        struct.pack_into(
            "<Q8I",
            raw,
            204 + index * 40,
            int(key, 16),
            start,
            count,
            0,
            0,
            0,
            0,
            0,
            section_copies,
        )
        size = 4 * count + 8 * section_copies
        warnings.append(
            f"CASC warning: FDID 1349477 has encrypted chunks with unknown TACT keys, zero-filled: {key} (chunk {index}, {size} bytes at offset {start})"
        )
        start += size
    return bytes(raw), warnings


class EncryptedDb2PolicyTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.data = Path(self.temporary.name)
        self.raw_path = "provenance/db2/1349477.db2"
        self.csv = b"ID,Directory,WdtFileDataID\n42,ReadableMap,1000\n"
        self.prepare([(MISSING_KEY, 1)])

    def put(self, path, raw):
        destination = self.data / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(raw)
        return {"path": path, "sha256": digest(raw)}

    def prepare(self, excluded, copies=0):
        raw, warnings = partial_db2(excluded, copies)
        self.put(self.raw_path, raw)
        self.put("db2/fixture/Map.csv", self.csv)
        ring = b"# Keyring Configuration\nkey-1000000000000000 = 11111111111111111111111111111111\n"
        ring_key = hashlib.md5(ring).hexdigest()
        info = (
            "Active!DEC:1|Build Key!HEX:16|Version!STRING:0|KeyRing!HEX:16|Product!STRING:0\n"
            f"1|{'b' * 32}|12.1.5.70529|{ring_key}|wow\n"
        ).encode()
        census = {
            "product": "wow",
            "build": "12.1.5.70529",
            "build_key": "b" * 32,
            "build_info": self.put("provenance/keys/.build.info", info),
            "keyring": self.put("provenance/keys/keyring", ring),
            "external": self.put(
                "provenance/keys/WoW.txt",
                b"0000000000000020 22222222222222222222222222222222\n",
            ),
        }
        self.document = {
            "identity": {
                "product": "wow",
                "build": "12.1.5.70448",
                "build_key": "a" * 32,
            },
            "tables": {
                "Map": {
                    "fdid": 1349477,
                    "layout": "12345678",
                    "rows": 1,
                    "db2_sha256": digest(raw),
                    "csv_sha256": digest(self.csv),
                    "content_authenticated": False,
                    "unreadable_records": sum(count for _, count in excluded),
                    "extraction_warnings": warnings,
                    "readable_section_policy": {
                        "name": POLICY,
                        "raw_path": self.raw_path,
                        "key_census": census,
                    },
                }
            },
        }
        self.save()

    def save(self):
        self.put("db2/fixture/receipt.json", json.dumps(self.document).encode())

    def graph(self):
        graph = ReceiptClosure(self.data, {}, "wow", "fixture")
        graph.seeds = {}
        return graph

    def rows(self, graph):
        return Catalogs(graph).index("Map")

    def test_only_readable_rows_qualify_without_promoting_whole_file(self):
        graph = self.graph()
        self.assertEqual(set(self.rows(graph)), {42})
        result = graph.run()
        self.assertEqual(result["summary"]["db2_policy_excluded_records"], 1)
        proof = result["db2_policy_exclusions"][0]
        self.assertEqual(proof["policy"], POLICY)
        self.assertEqual(
            proof["sections"],
            [{"index": 1, "key_id": MISSING_KEY, "records": 1, "copy_records": 0}],
        )
        inputs = {row["path"]: row for row in result["inputs"]}
        self.assertEqual(
            inputs["db2/fixture/Map.csv"]["identity_status"],
            "authenticated_readable_sections",
        )
        self.assertEqual(inputs["db2/fixture/Map.csv"]["actual_build"], "12.1.5.70448")
        self.assertIn(self.raw_path, inputs)
        self.assertFalse(
            json.loads((self.data / "db2/fixture/receipt.json").read_text())["tables"][
                "Map"
            ]["content_authenticated"]
        )
        self.assertFalse(
            any(
                row["code"] == "metadata_unreadable_records"
                for row in result["unresolved"]
            )
        )

    def test_each_excluded_section_keeps_its_key_and_record_count(self):
        self.prepare([(MISSING_KEY, 1), (SECOND_KEY, 2)])
        graph = self.graph()
        self.assertEqual(set(self.rows(graph)), {42})
        result = graph.run()
        self.assertEqual(result["summary"]["db2_policy_excluded_records"], 3)
        self.assertEqual(
            result["db2_policy_exclusions"][0]["sections"],
            [
                {"index": 1, "key_id": MISSING_KEY, "records": 1, "copy_records": 0},
                {"index": 2, "key_id": SECOND_KEY, "records": 2, "copy_records": 0},
            ],
        )

    def test_newly_available_key_refuses_exclusion_even_with_updated_census_hash(self):
        raw = f"{MISSING_KEY} 33333333333333333333333333333333\n".encode()
        census = self.document["tables"]["Map"]["readable_section_policy"]["key_census"]
        census["external"] = self.put("provenance/keys/WoW.txt", raw)
        self.save()
        graph = self.graph()
        self.assertEqual(self.rows(graph), {})
        self.assertTrue(
            any(
                POLICY in reason and MISSING_KEY in reason
                for _, reason, _ in graph.unresolved
            )
        )

    def test_unproven_partial_source_is_never_parsed(self):
        original = copy.deepcopy(self.document)
        for failure in [
            "wrong_raw_hash",
            "unkeyed_section",
            "missing_warning",
            "unexplained_record",
            "wrong_layout",
            "wrong_census_product",
            "older_census",
            "unknown_policy",
        ]:
            with self.subTest(failure=failure):
                self.prepare([(MISSING_KEY, 1)])
                table = self.document["tables"]["Map"]
                policy = table["readable_section_policy"]
                if failure == "wrong_raw_hash":
                    self.put(
                        self.raw_path,
                        (self.data / self.raw_path).read_bytes() + b"changed",
                    )
                elif failure == "unkeyed_section":
                    raw = bytearray((self.data / self.raw_path).read_bytes())
                    struct.pack_into("<Q", raw, 244, 0)
                    table["db2_sha256"] = digest(raw)
                    self.put(self.raw_path, raw)
                elif failure == "missing_warning":
                    table["extraction_warnings"] = []
                elif failure == "unexplained_record":
                    table["unreadable_records"] = 2
                elif failure == "wrong_layout":
                    table["layout"] = "87654321"
                elif failure == "wrong_census_product":
                    policy["key_census"]["product"] = "wow_classic_beta"
                elif failure == "older_census":
                    policy["key_census"]["build"] = "12.1.5.70391"
                else:
                    policy["name"] = "ACCEPT_ANY_PARTIAL"
                self.save()
                graph = self.graph()
                self.assertEqual(self.rows(graph), {})
        self.document = original

    def test_encrypted_section_counts_base_and_copy_rows_separately(self):
        # Production SpellVisual section1 has three encrypted records and two copies.
        self.prepare([(MISSING_KEY, 3)], copies=2)
        graph = self.graph()
        self.assertEqual(set(self.rows(graph)), {42})
        result = graph.run()
        self.assertEqual(result["summary"]["db2_policy_excluded_records"], 3)
        self.assertEqual(result["summary"].get("db2_policy_excluded_copy_records"), 2)
        self.assertEqual(result["summary"].get("db2_policy_excluded_rows"), 5)
        proof = result["db2_policy_exclusions"][0]
        self.assertEqual(proof.get("excluded_copy_records"), 2)
        self.assertEqual(proof["sections"][0].get("copy_records"), 2)
        self.assertEqual(set(self.rows(graph)), {42})

    def test_key_census_drift_invalidates_previously_qualified_rows(self):
        graph = self.graph()
        self.assertEqual(set(self.rows(graph)), {42})
        self.put(
            "provenance/keys/WoW.txt",
            f"{MISSING_KEY} 33333333333333333333333333333333\n".encode(),
        )
        self.assertEqual(self.rows(graph), {})
        result = graph.run()
        self.assertEqual(result["summary"]["db2_policy_excluded_records"], 0)
        self.assertTrue(
            any(row["code"] == "receipt_input_changed" for row in result["unresolved"])
        )


if __name__ == "__main__":
    unittest.main()
