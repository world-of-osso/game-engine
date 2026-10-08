"""Concrete raw-BLP icon oracle decoding and malformed input boundaries."""

import importlib.util
import struct
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "charcreate_skyborne", Path(__file__).with_name("charcreate_skyborne.py")
)
fixture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fixture)


def raw_atlas() -> bytes:
    header = bytearray(148)
    header[:4] = b"BLP2"
    header[8:12] = bytes([3, 8, 8, 0])
    struct.pack_into("<II", header, 12, 2, 1)
    struct.pack_into("<I", header, 20, 148)
    struct.pack_into("<I", header, 84, 8)
    return bytes(header) + bytes([7, 8, 9, 255, 30, 40, 50, 60])


class IconOracleTests(unittest.TestCase):
    def test_reads_raw_bgra_atlas_as_rgba(self):
        self.assertEqual(
            fixture.decode_atlas_pixels(raw_atlas()),
            (2, 1, bytes([9, 8, 7, 255, 50, 40, 30, 60])),
        )

    def test_rejects_truncated_mip(self):
        with self.assertRaisesRegex(ValueError, "mip"):
            fixture.decode_atlas_pixels(raw_atlas()[:-1])

    def test_rejects_other_encodings(self):
        data = bytearray(raw_atlas())
        data[8] = 2
        with self.assertRaisesRegex(ValueError, "raw BGRA"):
            fixture.decode_atlas_pixels(bytes(data))


if __name__ == "__main__":
    unittest.main()
