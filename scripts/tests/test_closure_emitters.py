"""Emitter asset dependencies match the native 272/274 parser layout."""

from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
from asset_closure import Closure


def chunk(tag, payload):
    return tag.encode() + struct.pack("<I", len(payload)) + payload


def emitter_model(geometry=b"particle/geometry.m2\0", child=b"particle/child.m2\0"):
    raw = bytearray(0x130 + 0x1EC + 176 + 64 + len(geometry) + len(child))
    raw[:4] = b"MD20"
    struct.pack_into("<I", raw, 4, 274)
    p = 0x130
    r = p + 0x1EC
    t = r + 176
    strings = t + 64
    struct.pack_into("<II", raw, 0x128, 1, p)
    struct.pack_into("<II", raw, 0x120, 1, r)
    struct.pack_into("<II", raw, 0x50, 3, t)
    struct.pack_into("<I", raw, p + 4, 0x10000000)
    struct.pack_into("<H", raw, p + 0x16, 0 | (1 << 5) | (2 << 10))
    struct.pack_into("<II", raw, p + 0x18, len(geometry), strings)
    struct.pack_into("<II", raw, p + 0x20, len(child), strings + len(geometry))
    raw[strings : strings + len(geometry)] = geometry
    raw[strings + len(geometry) :] = child
    struct.pack_into("<II", raw, r + 0x14, 1, t + 48)
    struct.pack_into("<H", raw, t + 48, 2)
    return raw


class EmitterTests(unittest.TestCase):
    def test_recursive_model_and_geometry_join_with_multitexture_and_ribbon(self):
        with tempfile.TemporaryDirectory() as tmp:
            data = Path(tmp)
            (data / "models").mkdir()
            (data / "models/1.m2").write_bytes(
                chunk("MD21", emitter_model())
                + chunk("TXID", struct.pack("<3I", 10, 11, 12))
            )
            (data / "models/2.m2").write_bytes(
                chunk("MD21", emitter_model(b"", b"particle/root.m2\0"))
                + chunk("TXID", struct.pack("<3I", 10, 11, 12))
            )
            paths = {
                1: "particle/root.m2",
                2: "particle/child.m2",
                3: "particle/geometry.m2",
            }
            g = Closure(data, paths, "wow", "fixture")
            g.add(1, "m2", "seed")
            result = g.run()
            self.assertEqual(
                sorted(g.assets),
                [
                    (1, "m2"),
                    (2, "m2"),
                    (3, "m2"),
                    (10, "blp"),
                    (11, "blp"),
                    (12, "blp"),
                ],
            )
            self.assertFalse(
                any(
                    i["code"] == "emitter_auxiliary_edges" for i in result["unresolved"]
                )
            )
            self.assertIn("resolver_tool_sha256", result)
            import hashlib

            source = Path(__file__).parents[1] / "closure_emitters.py"
            self.assertEqual(
                result["resolver_tool_sha256"]["closure_emitters.py"],
                hashlib.sha256(source.read_bytes()).hexdigest(),
            )
            self.assertTrue(
                any(
                    e["parent_fdid"] == 2
                    for a in result["assets"]
                    if a["fdid"] == 1
                    for e in a["edges"]
                )
            )

    def test_bone_and_physics_satellite_identities(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            (d / "models").mkdir()
            raw = bytearray(0x130)
            raw[:4] = b"MD20"
            (d / "models/1.m2").write_bytes(
                chunk("MD21", raw)
                + chunk("BFID", struct.pack("<2I", 100, 101))
                + chunk("PFID", struct.pack("<I", 102))
            )
            g = Closure(d, {}, "wow", "fixture")
            g.add(1, "m2", "seed")
            r = g.run()
            self.assertEqual(
                sorted(g.assets),
                [(1, "m2"), (100, "bone"), (101, "bone"), (102, "phys")],
            )
            self.assertFalse(
                any(i["code"] == "unsupported_model_edge" for i in r["unresolved"])
            )

    def test_truncated_filename_array_is_not_a_resolved_emitter(self):
        raw = emitter_model()
        struct.pack_into("<II", raw, 0x130 + 0x20, 20, len(raw) - 1)
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            (d / "models").mkdir()
            (d / "models/1.m2").write_bytes(chunk("MD21", raw))
            g = Closure(d, {}, "wow", "fixture")
            g.add(1, "m2", "seed")
            r = g.run()
            self.assertTrue(
                any(
                    i["code"] == "parse_error" and "filename" in i["reason"]
                    for i in r["unresolved"]
                )
            )


if __name__ == "__main__":
    unittest.main()
