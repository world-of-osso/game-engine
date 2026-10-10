"""A shared-data publication can expose a fresh frontier without own extracted files."""

from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
from asset_closure import Closure
from closure_extract_rounds import fixed_point


class ConvergenceTests(unittest.TestCase):
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
