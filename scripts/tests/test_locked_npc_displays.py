"""Unknown sections cannot prevent importing readable NPC records."""

import struct
import unittest

from scripts import import_npc_appearance as importer
from scripts.test_import_npc_appearance import fixture


def mixed_sections(kind="extra"):
    related = kind != "extra"
    layout = importer.LAYOUTS[kind][0]
    original = fixture(layout=layout, related=related)
    old_start = struct.unpack_from("<I", original, 212)[0]
    header = bytearray(original[:204])
    struct.pack_into("<I", header, 136, 3)
    struct.pack_into("<I", header, 200, 2)
    first = bytearray(original[204:244])
    start = old_start + 48
    struct.pack_into("<I", first, 8, start)
    size = struct.unpack_from("<I", original, 144)[0]
    second_start = start + len(original) - old_start
    unknown_id = 1088419 if related else 165799
    relation_size = 20 if related else 0
    second = struct.pack(
        "<Q8I", 0x057DC814574BD5B6, second_start, 1, 0, 0,
        4 if related else 0, relation_size, 0, 0,
    )
    return (
        bytes(header) + bytes(first) + second + original[244:old_start]
        + struct.pack("<2I", 1, unknown_id) + original[old_start:]
        + bytes(size + (4 if related else 0) + relation_size)
    )


class LockedNpcDisplayTests(unittest.TestCase):
    def test_readable_extra_survives_unknown_section_and_reports_its_key_and_id(self):
        skipped = []
        rows = importer.read_wdc5(mixed_sections(), "extra", skipped=skipped)
        self.assertEqual(rows, importer.read_wdc5(fixture(), "extra"))
        self.assertNotIn(165799, rows)
        self.assertEqual(skipped, [{
            "section": 1, "key_name": "057DC814574BD5B6",
            "record_ids": [165799], "parent": "not applicable (inline Extra)",
        }])

    def test_hidden_option_parent_is_reported_unknown_not_invented(self):
        skipped = []
        rows = importer.read_wdc5(mixed_sections("option"), "option", skipped=skipped)
        self.assertEqual(rows, {101: ((3, 9), 17), 102: ((4, 10), 18)})
        self.assertEqual(skipped[0]["record_ids"], [1088419])
        self.assertEqual(skipped[0]["parent"], "unknown (zero-filled)")

    def test_explicitly_required_hidden_extra_still_fails(self):
        extras = importer.read_wdc5(mixed_sections(), "extra", skipped=[])
        with self.assertRaisesRegex(ValueError, "display 42: missing Extra 165799"):
            importer.join_appearances(
                {42: 165799}, extras, {}, {}, {42: "character/test_hd.m2"}, {},
            )


if __name__ == "__main__":
    unittest.main()
