"""A shared-data publication can expose a fresh frontier without own extracted files."""

from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
from asset_closure import Closure
from closure_extract_rounds import fixed_point, missing_runtime_assets


class ConvergenceTests(unittest.TestCase):
    def test_existing_skeleton_still_publishes_new_owner_alias(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            (d / "models").mkdir()

            def chunk(tag, payload):
                return tag.encode() + struct.pack("<I", len(payload)) + payload

            raw = bytearray(0x130)
            raw[:4] = b"MD20"
            payloads = {
                1: chunk("MD21", raw) + chunk("SKID", struct.pack("<I", 2)),
                2: chunk("AFID", struct.pack("<HHI", 0, 0, 0)),
            }
            (d / "models/2.skel").write_bytes(payloads[2])

            def traverse():
                g = Closure(d, {}, "wow", "fixture")
                g.add(1, "m2", "root")
                return g.run()

            def count(m):
                return len(missing_runtime_assets(m["assets"], d, {}))

            def extract(m, n):
                created = 0
                for a in missing_runtime_assets(m["assets"], d, {}):
                    for name in a["locations"]:
                        p = d / name
                        if not p.exists():
                            p.write_bytes(payloads[a["fdid"]])
                            created += int(n != 1)
                return {"files": created}

            result = fixed_point(traverse(), extract, traverse, count, lambda row: None)
            self.assertEqual(result["summary"]["missing"], 0)
            self.assertTrue((d / "models/1.skel").is_file())
            self.assertEqual((d / "models/1.skel").read_bytes(), payloads[2])

    def test_peer_publication_with_equal_missing_count_still_expands_new_frontier(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)

            def chunk(tag, payload):
                return tag + struct.pack("<I", len(payload)) + payload

            payloads = {
                1: chunk(b"DIAM", struct.pack("<8I", 2, 0, 0, 0, 0, 0, 0, 0)),
                2: chunk(b"DIDM", struct.pack("<I", 3)),
                3: b"BLP2fixture",
            }

            def traverse():
                g = Closure(d, {}, "wow", "fixture")
                g.add(1, "wdt", "root")
                return g.run()

            def extract(m, n):
                # First WDT arrives from a peer; it is not an own publication.
                own = 0
                for a in m["assets"]:
                    if not a["present"]:
                        target = d / a["locations"][0]
                        target.parent.mkdir(parents=True, exist_ok=True)
                        target.write_bytes(payloads[a["fdid"]])
                        own += int(n != 1)
                return {"files": own}

            rounds = []
            result = fixed_point(
                traverse(),
                extract,
                traverse,
                lambda m: m["summary"]["missing"],
                rounds.append,
            )
            self.assertEqual(result["summary"]["present"], 3)
            self.assertEqual(result["summary"]["missing"], 0)
            self.assertEqual([r["extraction"]["files"] for r in rounds], [0, 1, 1])


if __name__ == "__main__":
    unittest.main()
